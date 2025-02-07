/* magpie_client/client.rs
 *
 * Copyright 2025 Romeo Calota
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

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{cell::RefCell, collections::HashMap, sync::Arc};

use arrayvec::ArrayString;
use gtk::glib::{g_critical, g_debug};

use magpie_types::apps::apps_response;
use magpie_types::apps::apps_response::AppList;
pub use magpie_types::apps::App;
use magpie_types::common::Empty;
use magpie_types::cpu::cpu_response;
pub use magpie_types::cpu::Cpu;
use magpie_types::disks::disks_response;
use magpie_types::disks::disks_response::{DiskList, OptionalSmartData};
use magpie_types::disks::disks_response_error::Error;
pub use magpie_types::disks::{Disk, DiskKind, ErrorEjectFailed, SmartData};
use magpie_types::gpus::gpus_response;
use magpie_types::gpus::gpus_response::GpuMap;
pub use magpie_types::gpus::Gpu;
use magpie_types::ipc::{self, response};
use magpie_types::memory::memory_response::MemoryInfo;
use magpie_types::memory::{memory_request, memory_response};
pub use magpie_types::memory::{Memory, MemoryDevice};
use magpie_types::network::connections_response;
use magpie_types::network::connections_response::ConnectionList;
pub use magpie_types::network::Connection;
use magpie_types::processes::processes_response;
use magpie_types::processes::processes_response::ProcessMap;
pub use magpie_types::processes::{Process, ProcessUsageStats};
use magpie_types::prost::Message;
use magpie_types::services::services_response;
use magpie_types::services::services_response::ServiceList;
pub use magpie_types::services::Service;

use crate::{flatpak_data_dir, is_flatpak, show_error_dialog_and_exit};

mod nng {
    pub use nng_c_sys::nng_errno_enum::*;
    pub use nng_c_sys::*;

    pub const NNG_OK: i32 = 0;
}

#[derive(Debug, Clone)]
pub struct FanInfo {
    pub fan_label: Arc<str>,
    pub temp_name: Arc<str>,
    pub temp_amount: i64,
    pub rpm: u64,
    pub percent_vroomimg: f32,

    pub fan_index: u64,
    pub hwmon_index: u64,

    pub max_speed: u64,
}

impl Default for FanInfo {
    fn default() -> Self {
        Self {
            fan_label: Arc::from(""),
            temp_name: Arc::from(""),
            temp_amount: 0,
            rpm: 0,
            percent_vroomimg: 0.0,

            fan_index: 0,
            hwmon_index: 0,

            max_speed: 0,
        }
    }
}

impl Eq for FanInfo {}

impl PartialEq<Self> for FanInfo {
    fn eq(&self, other: &Self) -> bool {
        self.fan_index == other.fan_index && self.hwmon_index == other.hwmon_index
    }
}

impl PartialOrd<Self> for FanInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(if self.hwmon_index == other.hwmon_index {
            self.fan_index.cmp(&other.fan_index)
        } else {
            self.hwmon_index.cmp(&other.hwmon_index)
        })
    }
}

impl Ord for FanInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.hwmon_index == other.hwmon_index {
            self.fan_index.cmp(&other.fan_index)
        } else {
            self.hwmon_index.cmp(&other.hwmon_index)
        }
    }
}

type ResponseBody = response::Body;
type AppsResponse = apps_response::Response;
type CpuResponse = cpu_response::Response;
type DisksResponse = disks_response::Response;
type GpusResponse = gpus_response::Response;
type MemoryResponse = memory_response::Response;
type ConnectionsResponse = connections_response::Response;
type ProcessesResponse = processes_response::Response;
type ServicesResponse = services_response::Response;

const ENV_MC_DEBUG_MAGPIE_PROCESS_SOCK: &str = "MC_DEBUG_MAGPIE_PROCESS_SOCK";

macro_rules! parse_response {
    ($response: ident, $body_kind: path, $response_kind_ok: path, $response_kind_err: path, $do: expr) => {{
        let expected_type = stringify!($response_kind_ok);
        match $response {
            Some($body_kind(response)) => match response.response {
                Some($response_kind_ok(arg)) => $do(arg),
                Some($response_kind_err(e)) => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Error while getting {}: {:?}",
                        expected_type,
                        e
                    );
                    Default::default()
                }
                _ => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Unexpected response: {:?}",
                        response.response
                    );
                    Default::default()
                }
            },
            _ => {
                g_critical!(
                    "MissionCenter::Gatherer",
                    "Unexpected response: {:?}",
                    $response
                );
                Default::default()
            }
        }
    }};
}

macro_rules! parse_response_with_err {
    ($response: ident, $body_kind: path, $response_kind_ok: path, $response_kind_err: path, $do: expr) => {{
        match $response {
            Some($body_kind(response)) => match response.response {
                Some($response_kind_ok(arg)) => Some(Ok($do(arg))),
                Some($response_kind_err(e)) => Some(Err(e)),
                _ => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Unexpected response: {:?}",
                        response.response
                    );
                    None
                }
            },
            _ => {
                g_critical!(
                    "MissionCenter::Gatherer",
                    "Unexpected response: {:?}",
                    $response
                );
                None
            }
        }
    }};
}

fn random_string<const CAP: usize>() -> ArrayString<CAP> {
    let mut result = ArrayString::new();
    for _ in 0..CAP {
        if rand::random::<bool>() {
            result.push(rand::random_range(b'a'..=b'z') as char);
        } else {
            result.push(rand::random_range(b'0'..=b'9') as char);
        }
    }

    result
}

fn magpie_command(socket_addr: &str) -> std::process::Command {
    fn executable() -> String {
        use gtk::glib::g_debug;

        let exe_simple = "missioncenter-magpie".to_owned();

        if is_flatpak() {
            let flatpak_app_path = super::flatpak_app_path();

            let cmd_glibc_status = cmd_flatpak_host!(&format!(
                "{}/bin/missioncenter-magpie-glibc --test",
                flatpak_app_path
            ))
            .status()
            .is_ok_and(|exit_status| exit_status.success());
            if cmd_glibc_status {
                let exe_glibc = format!("{}/bin/missioncenter-magpie-glibc", flatpak_app_path);
                g_debug!(
                    "MissionCenter::Gatherer",
                    "Magpie executable name: {}",
                    &exe_glibc
                );
                return exe_glibc;
            }

            let cmd_musl_status = cmd_flatpak_host!(&format!(
                "{}/bin/missioncenter-magpie-musl --test",
                flatpak_app_path
            ))
            .status()
            .is_ok_and(|exit_status| exit_status.success());
            if cmd_musl_status {
                let exe_musl = format!("{}/bin/missioncenter-magpie-musl", flatpak_app_path);
                g_debug!(
                    "MissionCenter::Gatherer",
                    "Magpie executable name: {}",
                    &exe_musl
                );
                return exe_musl;
            }
        }

        g_debug!(
            "MissionCenter::Gatherer",
            "Magpie executable name: {}",
            &exe_simple
        );

        exe_simple
    }

    let mut command = if is_flatpak() {
        const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

        let mut cmd = std::process::Command::new(FLATPAK_SPAWN_CMD);
        cmd.arg("-v")
            .arg("--watch-bus")
            .arg("--host")
            .arg(executable());
        cmd.env(
            "MC_MAGPIE_HW_DB",
            format!("{}/share/missioncenter/hw.db", super::flatpak_app_path()),
        );
        cmd
    } else {
        let mut cmd = std::process::Command::new(executable());

        if let Some(mut appdir) = std::env::var_os("APPDIR") {
            appdir.push("/runtime/default");
            cmd.current_dir(appdir);
        }

        cmd
    };
    command
        .env_remove("LD_PRELOAD")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .arg("--addr")
        .arg(socket_addr);

    command
}

fn connect_socket(socket: &mut nng::nng_socket, socket_addr: &str) -> bool {
    let _ = unsafe { nng::nng_close(*socket) };
    socket.id = 0;

    let res = unsafe { nng::nng_req0_open(socket) };
    match res {
        nng::NNG_OK => {}
        nng::NNG_ENOMEM => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to open socket: Out of memory"
            );
            return false;
        }
        nng::NNG_ENOTSUP => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to open socket: Protocol not supported"
            );
            return false;
        }
        _ => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to open socket: Unknown error: {}",
                res
            );
            return false;
        }
    }

    let res = unsafe { nng::nng_dial(*socket, socket_addr.as_ptr() as _, std::ptr::null_mut(), 0) };
    match res {
        nng::NNG_OK => {}
        nng::NNG_EADDRINVAL => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to dial socket: An invalid url was specified"
            );
            return false;
        }
        nng::NNG_ECLOSED => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to dial socket: The socket is not open"
            );
            return false;
        }
        nng::NNG_ECONNREFUSED => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to dial socket: The remote peer refused the connection"
            );
            return false;
        }
        nng::NNG_ECONNRESET => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to dial socket: The remote peer reset the connection"
            );
            return false;
        }
        nng::NNG_EINVAL => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to dial socket: An invalid set of flags or an invalid url was specified"
            );
            return false;
        }
        nng::NNG_ENOMEM => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to dial socket: Insufficient memory is available"
            );
            return false;
        }
        nng::NNG_EPEERAUTH => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to dial socket: Authentication or authorization failure"
            );
            return false;
        }
        nng::NNG_EPROTO => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to dial socket: A protocol error occurred"
            );
            return false;
        }
        nng::NNG_EUNREACHABLE => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to dial socket: The remote address is not reachable"
            );
            return false;
        }
        _ => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to dial socket: Unknown error: {}",
                res
            );
            return false;
        }
    }

    true
}

fn make_request(
    request: ipc::Request,
    socket: &mut nng::nng_socket,
    socket_addr: &str,
) -> Option<ipc::Response> {
    fn try_reconnect(socket: &mut nng::nng_socket, socket_addr: &str) {
        unsafe { nng::nng_close(*socket) };
        socket.id = 0;

        for i in 0..=5 {
            if !connect_socket(socket, socket_addr) {
                g_critical!(
                    "MissionCenter::Gatherer",
                    "Failed to reconnect to Magpie. Retrying in 100ms (try {}/5)",
                    i + 1
                );
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }

            return;
        }

        show_error_dialog_and_exit(
            "Lost connection to Magpie and failed to reconnect after 5 tries. Giving up.",
        );
    }

    let mut req_buf = Vec::new();

    if let Err(e) = request.encode(&mut req_buf) {
        g_critical!(
            "MissionCenter::Gatherer",
            "Failed to encode request {:?}: {}",
            req_buf,
            e
        );
        return None;
    }

    let res = unsafe { nng::nng_send(*socket, req_buf.as_ptr() as *mut _, req_buf.len(), 0) };
    match res {
        nng::NNG_OK => {}
        nng::NNG_EAGAIN => {
            g_critical!("MissionCenter::Gatherer","Failed to send request: The operation would block, but NNG_FLAG_NONBLOCK was specified");
            return None;
        }
        nng::NNG_ECLOSED => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to send request: The socket is not open"
            );
            try_reconnect(socket, socket_addr);
            return None;
        }
        nng::NNG_EINVAL => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to send request: An invalid set of flags was specified"
            );
            return None;
        }
        nng::NNG_EMSGSIZE => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to send request: The value of size is too large"
            );
            return None;
        }
        nng::NNG_ENOMEM => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to send request: Insufficient memory is available"
            );
            return None;
        }
        nng::NNG_ENOTSUP => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to send request: The protocol for socket does not support sending"
            );
            return None;
        }
        nng::NNG_ESTATE => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to send request: The socket cannot send data in this state"
            );
            return None;
        }
        nng::NNG_ETIMEDOUT => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to send request: The operation timed out"
            );
            return None;
        }
        _ => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to send request: Unknown error: {}",
                res
            );
            return None;
        }
    }

    let mut message_buffer: *mut libc::c_void = std::ptr::null_mut();
    let mut message_len: libc::size_t = 0;

    let res = unsafe {
        nng::nng_recv(
            *socket,
            (&mut message_buffer) as *mut *mut _ as *mut _,
            &mut message_len,
            nng::NNG_FLAG_ALLOC,
        )
    };
    match res {
        nng::NNG_OK => {}
        nng::NNG_EAGAIN => {
            g_critical!("MissionCenter::Gatherer","Failed to read message: The operation would block, but NNG_FLAG_NONBLOCK was specified");
            return None;
        }
        nng::NNG_ECLOSED => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to read message: The socket is not open"
            );
            try_reconnect(socket, socket_addr);
            return None;
        }
        nng::NNG_EINVAL => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to read message: An invalid set of flags was specified"
            );
            return None;
        }
        nng::NNG_EMSGSIZE => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to read message: The received message did not fit in the size provided"
            );
            return None;
        }
        nng::NNG_ENOMEM => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to read message: Insufficient memory is available"
            );
            return None;
        }
        nng::NNG_ENOTSUP => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to read message: The protocol for socket does not support receiving"
            );
            return None;
        }
        nng::NNG_ESTATE => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to read message: The socket cannot receive data in this state"
            );
            return None;
        }
        nng::NNG_ETIMEDOUT => {
            g_debug!(
                "MissionCenter::Gatherer",
                "No message received for 64ms, waiting and trying again..."
            );
            std::thread::sleep(Duration::from_millis(10));
            return None;
        }
        _ => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to read message: Unknown error: {}",
                res
            );
            return None;
        }
    }

    if message_len == 0 || message_buffer.is_null() {
        g_critical!(
            "MissionCenter::Gatherer",
            "Failed to read response: Empty message"
        );
        return None;
    }

    let response_buffer =
        unsafe { core::slice::from_raw_parts(message_buffer as *const u8, message_len) };

    let response = match ipc::Response::decode(response_buffer) {
        Ok(r) => r,
        Err(e) => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Error while decoding response: {:?}",
                e
            );
            unsafe { nng::nng_free(message_buffer, message_len) };
            return None;
        }
    };

    unsafe { nng::nng_free(message_buffer, message_len) };

    Some(response)
}

pub struct Client {
    socket: RefCell<nng::nng_socket>,

    socket_addr: Arc<str>,
    child_thread: RefCell<std::thread::JoinHandle<()>>,
    stop_requested: Arc<AtomicBool>,
}

impl Drop for Client {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Client {
    pub fn new() -> Self {
        let socket_addr =
            if let Ok(mut existing_sock) = std::env::var(ENV_MC_DEBUG_MAGPIE_PROCESS_SOCK) {
                existing_sock.push('\0');
                Arc::from(existing_sock)
            } else {
                if is_flatpak() {
                    Arc::from(format!(
                        "ipc://{}/magpie.ipc\0",
                        flatpak_data_dir().display()
                    ))
                } else {
                    Arc::from(format!("ipc:///tmp/magpie_{}.ipc\0", random_string::<8>()))
                }
            };

        Self {
            socket: RefCell::new(nng::nng_socket { id: 0 }),

            socket_addr,
            child_thread: RefCell::new(std::thread::spawn(|| {})),
            stop_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        fn start_magpie_process_thread(
            socket_addr: Arc<str>,
            stop_requested: Arc<AtomicBool>,
        ) -> std::thread::JoinHandle<()> {
            std::thread::spawn(move || {
                fn spawn_child(socket_addr: &str) -> std::process::Child {
                    match magpie_command(socket_addr.trim_end_matches('\0')).spawn() {
                        Ok(child) => child,
                        Err(e) => {
                            g_critical!(
                                "MissionCenter::Gatherer",
                                "Failed to spawn Magpie process: {}",
                                &e
                            );
                            show_error_dialog_and_exit(&format!(
                                "Failed to spawn Magpie process: {}",
                                e
                            ));
                        }
                    }
                }

                let mut child = spawn_child(&socket_addr);

                while !stop_requested.load(Ordering::Relaxed) {
                    match child.try_wait() {
                        Ok(Some(exit_status)) => {
                            let _ = std::fs::remove_file(&socket_addr[6..]);

                            if !stop_requested.load(Ordering::Relaxed) {
                                g_critical!(
                                    "MissionCenter::Gatherer",
                                    "Magpie process exited unexpectedly: {}. Restarting...",
                                    exit_status
                                );
                                std::mem::swap(&mut child, &mut spawn_child(&socket_addr));
                            }
                        }
                        Ok(None) => {
                            std::thread::sleep(Duration::from_millis(100));
                            continue;
                        }
                        Err(e) => {
                            g_critical!(
                                "MissionCenter::Gatherer",
                                "Failed to wait for Gatherer process to stop: {}",
                                &e
                            );
                            show_error_dialog_and_exit(&format!(
                                "Failed to wait for Gatherer process to stop: {}",
                                e
                            ));
                        }
                    }
                }

                let _ = child.kill();
            })
        }

        if !std::env::var(ENV_MC_DEBUG_MAGPIE_PROCESS_SOCK).is_ok() {
            *self.child_thread.borrow_mut() =
                start_magpie_process_thread(self.socket_addr.clone(), self.stop_requested.clone());
        }

        const START_WAIT_TIME_MS: u64 = 300;
        const RETRY_COUNT: i32 = 50;

        // Let the child process start up
        for _ in 0..RETRY_COUNT {
            std::thread::sleep(Duration::from_millis(START_WAIT_TIME_MS / 2));

            if connect_socket(&mut *self.socket.borrow_mut(), &self.socket_addr) {
                return;
            }

            std::thread::sleep(Duration::from_millis(START_WAIT_TIME_MS / 2));
        }

        show_error_dialog_and_exit("Failed to connect to Gatherer socket");
    }

    pub fn stop(&self) {
        self.stop_requested.store(true, Ordering::Relaxed);
        let child_thread = std::mem::replace(
            &mut *self.child_thread.borrow_mut(),
            std::thread::spawn(|| {}),
        );
        let _ = child_thread.join();
    }
}

impl Client {
    pub fn set_refresh_interval(&self, _interval: u64) {}

    pub fn set_core_count_affects_percentages(&self, _v: bool) {}

    pub fn cpu(&self) -> Cpu {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(ipc::req_get_cpu(), &mut socket, self.socket_addr.as_ref())
            .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Cpu,
            CpuResponse::Cpu,
            CpuResponse::Error,
            |cpu: Cpu| cpu
        )
    }

    pub fn memory(&self) -> Memory {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_get_memory(memory_request::Kind::Memory),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Memory,
            MemoryResponse::MemoryInfo,
            MemoryResponse::Error,
            |memory: MemoryInfo| {
                let Some(memory_response::memory_info::Response::Memory(memory)) = memory.response
                else {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Unexpected response when getting memory",
                    );
                    return Default::default();
                };

                memory
            }
        )
    }

    pub fn memory_devices(&self) -> Vec<MemoryDevice> {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_get_memory(memory_request::Kind::MemoryDevices),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Memory,
            MemoryResponse::MemoryInfo,
            MemoryResponse::Error,
            |memory: MemoryInfo| {
                let Some(memory_response::memory_info::Response::MemoryDevices(mut devices)) =
                    memory.response
                else {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Unexpected response when getting memory devices",
                    );
                    return vec![];
                };

                std::mem::take(&mut devices.devices)
            }
        )
    }

    pub fn disks_info(&self) -> Vec<Disk> {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(ipc::req_get_disks(), &mut socket, self.socket_addr.as_ref())
            .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Disks,
            DisksResponse::Disks,
            DisksResponse::Error,
            |mut disks: DiskList| { std::mem::take(&mut disks.disks) }
        )
    }

    pub fn eject_disk(&self, disk_id: String) -> Result<(), ErrorEjectFailed> {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_eject_disk(disk_id),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        let result = parse_response_with_err!(
            response,
            ResponseBody::Disks,
            DisksResponse::Eject,
            DisksResponse::Error,
            |_: Empty| { () }
        );

        let Some(result) = result else { return Ok(()) };
        match result {
            Ok(()) => Ok(()),
            Err(e) => match e.error {
                Some(Error::Eject(e)) => Err(e),
                _ => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Unexpected error response: {e:?}"
                    );
                    Ok(())
                }
            },
        }
    }

    pub fn smart_data(&self, disk_id: String) -> Option<SmartData> {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_get_smart_data(disk_id),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Disks,
            DisksResponse::Smart,
            DisksResponse::Error,
            |mut smart_data: OptionalSmartData| { std::mem::take(&mut smart_data.smart) }
        )
    }

    pub fn fans_info(&self) -> Vec<FanInfo> {
        vec![]
    }

    pub fn network_connections(&self) -> Vec<Connection> {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_get_connections(),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Connections,
            ConnectionsResponse::Connections,
            ConnectionsResponse::Error,
            |mut connections: ConnectionList| { std::mem::take(&mut connections.connections) }
        )
    }

    pub fn gpus(&self) -> HashMap<String, Gpu> {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(ipc::req_get_gpus(), &mut socket, self.socket_addr.as_ref())
            .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Gpus,
            GpusResponse::Gpus,
            GpusResponse::Error,
            |mut gpus: GpuMap| { std::mem::take(&mut gpus.gpus) }
        )
    }

    pub fn processes(&self) -> HashMap<u32, Process> {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_get_processes(),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Processes,
            ProcessesResponse::Processes,
            ProcessesResponse::Error,
            |mut processes: ProcessMap| { std::mem::take(&mut processes.processes) }
        )
    }

    pub fn apps(&self) -> HashMap<String, App> {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(ipc::req_get_apps(), &mut socket, self.socket_addr.as_ref())
            .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Apps,
            AppsResponse::Apps,
            AppsResponse::Error,
            |mut app_list: AppList| {
                app_list
                    .apps
                    .drain(..)
                    .map(|app| (app.id.clone(), app))
                    .collect()
            }
        )
    }

    pub fn services(&self) -> HashMap<String, Service> {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_get_services(),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Services,
            ServicesResponse::Services,
            ServicesResponse::Error,
            |mut service_list: ServiceList| {
                service_list
                    .services
                    .drain(..)
                    .map(|service| (service.id.clone(), service))
                    .collect()
            }
        )
    }

    pub fn service_logs(&self, service_id: String, pid: Option<NonZeroU32>) -> String {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_get_logs(service_id, pid),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Services,
            ServicesResponse::Logs,
            ServicesResponse::Error,
            |logs| logs
        )
    }

    pub fn terminate_process(&self, _pid: u32) {}

    pub fn kill_process(&self, _pid: u32) {}
    pub fn kill_processes(&self, _pids: Vec<u32>) {}

    pub fn start_service(&self, service_id: String) {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_start_service(service_id),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Services,
            ServicesResponse::Empty,
            ServicesResponse::Error,
            |_| {}
        )
    }

    pub fn stop_service(&self, service_id: String) {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_stop_service(service_id),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Services,
            ServicesResponse::Empty,
            ServicesResponse::Error,
            |_| {}
        )
    }

    pub fn restart_service(&self, service_id: String) {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_restart_service(service_id),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Services,
            ServicesResponse::Empty,
            ServicesResponse::Error,
            |_| {}
        )
    }

    pub fn enable_service(&self, service_id: String) {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_enable_service(service_id),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Services,
            ServicesResponse::Empty,
            ServicesResponse::Error,
            |_| {}
        )
    }

    pub fn disable_service(&self, service_id: String) {
        let mut socket = self.socket.borrow_mut();

        let response = make_request(
            ipc::req_disable_service(service_id),
            &mut socket,
            self.socket_addr.as_ref(),
        )
        .and_then(|response| response.body);

        parse_response!(
            response,
            ResponseBody::Services,
            ServicesResponse::Empty,
            ServicesResponse::Error,
            |_| {}
        )
    }
}
