/* sys_info_v2/gatherer/src/main.rs
 *
 * Copyright 2023 Romeo Calota
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#[allow(unused_imports)]
pub use arrayvec::ArrayVec;

pub use apps::{AppPIDs, Apps};
#[allow(unused_imports)]
pub(crate) use logging::{critical, debug, error, info, message, warning};
pub use processes::Processes;
pub use util::{to_binary, to_binary_mut};

macro_rules! acknowledge {
    ($connection: ident) => {{
        use std::io::Write;

        $crate::message!("Gatherer::Main", "Sending Acknowledge");

        if let Err(e) = $connection.write_all(to_binary(&ipc::Message::Acknowledge)) {
            $crate::critical!(
                "Gatherer::Main",
                "Failed to write to IPC socket, exiting: {:#?}",
                e
            );
            std::process::exit(exit_code::ExitCode::SendAcknowledgeFailed as i32);
        }
    }};
}

macro_rules! data_ready {
    ($connection: ident) => {{
        use std::io::Write;

        $crate::message!("Gatherer::Main", "Sending DataReady");

        if let Err(e) = $connection.write_all(to_binary(&ipc::Message::DataReady)) {
            $crate::critical!(
                "Gatherer::Main",
                "Failed to write to IPC socket, exiting: {:#?}",
                e
            );
            std::process::exit(exit_code::ExitCode::SendDataReadyFailed as i32);
        }
    }};
}

mod apps;
#[path = "../common/exit_code.rs"]
mod exit_code;
#[path = "../common/ipc/mod.rs"]
mod ipc;
mod logging;
mod platform;
mod processes;
#[path = "../common/util.rs"]
mod util;

pub type ArrayString = arrayvec::ArrayString<256>;
pub type ProcessStats = processes::Stats;
pub type CpuStaticInfo = platform::cpu::StaticInfo;
pub type CpuDynamicInfo = platform::cpu::DynamicInfo;
pub type LogicalCpuInfo = platform::cpu::LogicalInfo;
pub type GpuPciIds = platform::gpu::PciIds;
pub type GpuStaticInfo = platform::gpu::StaticInfo;
pub type GpuDynamicInfo = platform::gpu::DynamicInfo;

#[path = "../common/shared_data.rs"]
mod shared_data;

pub trait ToArrayStringLossy {
    fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY>;
}

impl ToArrayStringLossy for str {
    fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY> {
        let mut result = arrayvec::ArrayString::new();
        if self.len() > CAPACITY {
            for i in (0..CAPACITY).rev() {
                if self.is_char_boundary(i) {
                    result.push_str(&self[0..i]);
                    break;
                }
            }
        } else {
            result.push_str(self);
        }

        result
    }
}

impl ToArrayStringLossy for std::borrow::Cow<'_, str> {
    fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY> {
        let mut result = arrayvec::ArrayString::new();
        if self.len() > CAPACITY {
            for i in (0..CAPACITY).rev() {
                if self.is_char_boundary(i) {
                    result.push_str(&self[0..i]);
                    break;
                }
            }
        } else {
            result.push_str(self);
        }

        result
    }
}

fn main() {
    use exit_code::ExitCode;
    use interprocess::local_socket::*;
    use platform::{CpuInfoExt, GpuInfoExt};
    use shared_data::{SharedData, SharedDataContent};
    use std::io::Read;

    message!("Gatherer::Main", "Starting gatherer");

    let parent_pid = unsafe { libc::getppid() };
    message!("Gatherer::Main", "Parent PID: {}", parent_pid);

    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        critical!("Gatherer::Main", "Not enough arguments");
        std::process::exit(ExitCode::MissingProgramArgument as i32);
    }

    message!(
        "Gatherer::Main",
        "Connecting to parent IPC socket '{}'",
        args[1]
    );

    if !std::path::Path::new(&args[1]).exists() {
        critical!("Gatherer::Main", "IPC socket '{}' does not exist", args[1]);
        std::process::exit(ExitCode::SocketConnectionFailed as i32);
    }
    let mut connection = match LocalSocketStream::connect(args[1].as_str()) {
        Ok(c) => c,
        Err(e) => {
            critical!("Gatherer::Main", "Unable to connect to parent: {}", e);
            std::process::exit(ExitCode::SocketConnectionFailed as i32);
        }
    };

    message!(
        "Gatherer::Main",
        "Connecting to shared memory '{}'",
        args[2]
    );

    if !std::path::Path::new(&args[2]).exists() {
        critical!("Gatherer::Main", "File link '{}' does not exist", args[2]);
        std::process::exit(ExitCode::FileLinkNotFound as i32);
    }
    let mut shared_memory = match ipc::SharedMemory::<SharedData>::new(&args[2], false) {
        Ok(sm) => sm,
        Err(e) => {
            critical!("Gatherer::Main", "Unable to create shared memory: {}", e);
            std::process::exit(ExitCode::UnableToCreateSharedMemory as i32);
        }
    };

    let mut cpu_info = platform::CpuInfo::new();
    let mut gpu_info = platform::GpuInfo::new();

    let mut message = ipc::Message::Unknown;
    loop {
        if unsafe { libc::getppid() } != parent_pid {
            message!(
                "Gatherer::Main",
                "Parent process no longer running, exiting"
            );
            break;
        }

        message!("Gatherer::Main", "Waiting for message...");

        if let Err(e) = connection.read_exact(to_binary_mut(&mut message)) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                message!(
                    "Gatherer::Main",
                    "Main application has disconnected, shutting down"
                );
                std::process::exit(0);
            } else {
                critical!(
                    "Gatherer::Main",
                    "Failed to read from IPC socket, exiting: {}",
                    e
                );
                std::process::exit(ExitCode::ReadFromSocketFailed as i32);
            }
        }
        message!("Gatherer::Main", "Received message: {:?}", message);

        match message {
            ipc::Message::GetProcesses => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::Processes(Processes::new());

                data_ready!(connection);
            }
            ipc::Message::GetApps => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::Apps(Apps::new());

                data_ready!(connection);
            }
            ipc::Message::GetAppPIDs => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::AppPIDs(AppPIDs::new());

                data_ready!(connection);
            }
            ipc::Message::GetCpuStaticInfo => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::CpuStaticInfo(cpu_info.static_info());

                data_ready!(connection);
            }
            ipc::Message::GetCpuDynamicInfo => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::CpuDynamicInfo(cpu_info.dynamic_info());

                data_ready!(connection);
            }
            ipc::Message::GetLogicalCpuInfo => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::LogicalCpuInfo(cpu_info.logical_cpu_info());

                data_ready!(connection);
            }
            ipc::Message::EnumerateGpus => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::GpuPciIds(gpu_info.enumerate());

                data_ready!(connection);
            }
            ipc::Message::GetGpuStaticInfo => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::GpuStaticInfo(gpu_info.static_info());

                data_ready!(connection);
            }
            ipc::Message::GetGpuDynamicInfo => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::GpuDynamicInfo(gpu_info.dynamic_info());

                data_ready!(connection);
            }
            ipc::Message::TerminateProcess(pid) => {
                acknowledge!(connection);

                unsafe {
                    libc::kill(pid as _, libc::SIGTERM);
                }
            }
            ipc::Message::KillProcess(pid) => {
                acknowledge!(connection);

                unsafe {
                    libc::kill(pid as _, libc::SIGKILL);
                }
            }
            ipc::Message::KillProcessTree(_ppid) => {
                acknowledge!(connection);
            }
            ipc::Message::Acknowledge | ipc::Message::DataReady => {
                // Wierd thing to send, but there you go, send Acknowledge back anyway
                acknowledge!(connection);
            }
            ipc::Message::Exit => {
                acknowledge!(connection);

                std::process::exit(0);
            }
            ipc::Message::Unknown => {
                critical!("Gatherer::Main", "Unknown message received; exiting");
                std::process::exit(ExitCode::UnknownMessageReceived as i32);
            }
        }
    }
}
