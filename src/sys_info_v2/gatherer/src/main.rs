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

macro_rules! acknowledge {
    ($connection: ident) => {{
        use std::io::Write;

        if let Err(e) = $connection.write_all(common::to_binary(&common::ipc::Message::Acknowledge))
        {
            eprintln!("Gatherer: Failed to write to IPC socket, exiting: {:#?}", e);
            std::process::exit(common::ExitCode::SendAcknowledgeFailed as i32);
        }
    }};
}

macro_rules! data_ready {
    ($connection: ident) => {{
        use std::io::Write;

        if let Err(e) = $connection.write_all(common::to_binary(&common::ipc::Message::DataReady)) {
            eprintln!("Gatherer: Failed to write to IPC socket, exiting: {:#?}", e);
            std::process::exit(common::ExitCode::SendDataReadyFailed as i32);
        }
    }};
}

#[path = "../common/mod.rs"]
mod common;

fn main() {
    use common::{ipc, AppPIDs, Apps, ExitCode, Processes, SharedData, SharedDataContent};
    use interprocess::local_socket::*;
    use std::io::Read;

    let parent_pid = unsafe { libc::getppid() };

    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        eprintln!("Gatherer: not enough arguments");
        std::process::exit(ExitCode::MissingProgramArgument as i32);
    }

    if !std::path::Path::new(&args[1]).exists() {
        eprintln!("Gatherer: IPC socket '{}' does not exist", args[1]);
        std::process::exit(ExitCode::SocketConnectionFailed as i32);
    }
    let mut connection = match LocalSocketStream::connect(args[1].as_str()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Gatherer: Unable to connect to parent: {}", e);
            std::process::exit(ExitCode::SocketConnectionFailed as i32);
        }
    };

    if !std::path::Path::new(&args[2]).exists() {
        eprintln!("Gatherer: File link '{}' does not exist", args[2]);
        std::process::exit(ExitCode::FileLinkNotFound as i32);
    }
    let mut shared_memory = match ipc::SharedMemory::<SharedData>::new(&args[2], false) {
        Ok(sm) => sm,
        Err(e) => {
            eprintln!("Gatherer: Unable to create shared memory: {}", e);
            std::process::exit(ExitCode::UnableToCreateSharedMemory as i32);
        }
    };

    let mut message = ipc::Message::Unknown;
    loop {
        if unsafe { libc::getppid() } != parent_pid {
            eprintln!("Gatherer: Parent process no longer running, exiting");
            break;
        }

        if let Err(e) = connection.read_exact(common::to_binary_mut(&mut message)) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                eprintln!("Gatherer: Main application has disconnected, shutting down");
                std::process::exit(0);
            } else {
                eprintln!("Gatherer: Failed to read from IPC socket, exiting: {}", e);
                std::process::exit(ExitCode::ReadFromSocketFailed as i32);
            }
        }

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
                eprintln!("Gatherer: Unknown message received");
                std::process::exit(ExitCode::UnknownMessageReceived as i32);
            }
        }
    }
}
