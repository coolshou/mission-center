/* sys_info_v2/gatherer.rs
 *
 * Copyright 2024 Romeo Calota
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
use std::thread::JoinHandle;
use std::time::Duration;
use std::{cell::RefCell, collections::HashMap, sync::Arc};

use arrayvec::ArrayString;
use gtk::glib::g_critical;
use zeromq::prelude::*;
use zeromq::{ReqSocket, ZmqError};

use magpie_types::apps::apps_response;
use magpie_types::apps::apps_response::AppList;
pub use magpie_types::apps::App;
use magpie_types::gpus::gpus_response;
use magpie_types::gpus::gpus_response::GpuMap;
pub use magpie_types::gpus::Gpu;
use magpie_types::ipc::{self, response};
use magpie_types::processes::processes_response;
use magpie_types::processes::processes_response::ProcessMap;
pub use magpie_types::processes::{Process, ProcessUsageStats};
use magpie_types::prost::Message;

pub use super::dbus_interface::*;
use crate::show_error_dialog_and_exit;

type ResponseBody = response::Body;
type ProcessesResponse = processes_response::Response;
type AppsResponse = apps_response::Response;
type GpusResponse = gpus_response::Response;

const ENV_MC_DEBUG_MAGPIE_PROCESS_SOCK: &str = "MC_DEBUG_MAGPIE_PROCESS_SOCK";

macro_rules! parse_response {
    ($response: ident, $body_kind: path, $response_kind_ok: path, $response_kind_err: path, $do: expr) => {{
        match $response {
            Some($body_kind(response)) => match response.response {
                Some($response_kind_ok(arg)) => $do(arg),
                Some($response_kind_err(e)) => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Error while getting {}: {:?}",
                        stringify!($response_variant),
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

        if crate::is_flatpak() {
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

    let mut command = if crate::is_flatpak() {
        const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

        let mut cmd = std::process::Command::new(FLATPAK_SPAWN_CMD);
        cmd.arg("-v")
            .arg("--watch-bus")
            .arg("--host")
            .arg(executable());
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

async fn zero_mq_request(
    request: ipc::Request,
    socket: &mut ReqSocket,
    socket_addr: &str,
) -> Option<ipc::Response> {
    async fn try_reconnect(socket: &mut ReqSocket, socket_addr: &str) {
        let _ = std::mem::replace(socket, ReqSocket::new());
        for i in 0..=5 {
            match socket.connect(socket_addr).await {
                Err(e) => {
                    let error_msg = format!("Failed to reconnect to Magpie socket in {i} tries: {e}");
                    g_critical!("MissionCenter::Gatherer", "{}", &error_msg);
                }
                _ => {
                    // We reconnected, try again next time
                    return;
                }
            }
        }
        show_error_dialog_and_exit("Lost connection to Magpie and failed to reconnect after 5 tries. Giving up.");
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

    let send_res = socket.send(req_buf.into()).await;
    match send_res {
        Err(ZmqError::Codec(..)) => {
            // Might mean that the socket was closed, try to reconnect
            try_reconnect(socket, socket_addr).await;
            return None;
        }
        Err(e) => {
            g_critical!("MissionCenter::Gatherer", "Failed to send request: {}", e);
            return None;
        }
        _ => {}
    }

    let recv_res = socket.recv().await;
    let response = match recv_res {
        Ok(response) => response.into_vec(),
        Err(ZmqError::Codec(..)) => {
            // Might mean that the socket was closed, try to reconnect
            try_reconnect(socket, socket_addr).await;
            return None;
        }
        Err(e) => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to receive response: {}",
                e
            );
            return None;
        }
    };
    if response.is_empty() {
        g_critical!(
            "MissionCenter::Gatherer",
            "Empty reply when getting processes"
        );
        return None;
    }
    let decode_result = if response.len() > 1 {
        ipc::Response::decode(response.concat().as_slice())
    } else {
        ipc::Response::decode(response[0].iter().as_slice())
    };
    let response = match decode_result {
        Ok(r) => r,
        Err(e) => {
            g_critical!(
                "MissionCenter::Gatherer",
                "Error while getting process list: {:?}",
                e
            );
            return None;
        }
    };

    Some(response)
}

pub struct Gatherer {
    socket: RefCell<ReqSocket>,
    tokio_runtime: tokio::runtime::Runtime,

    socket_addr: Arc<str>,
    child_thread: RefCell<std::thread::JoinHandle<()>>,
    stop_requested: Arc<AtomicBool>,
}

impl Drop for Gatherer {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Gatherer {
    pub fn new() -> Self {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");

        let socket_addr = if let Ok(existing_sock) = std::env::var(ENV_MC_DEBUG_MAGPIE_PROCESS_SOCK) {
            Arc::from(existing_sock)
        } else {
            Arc::from(format!("ipc:///tmp/magpie_{}.ipc", random_string::<8>()))
        };

        Self {
            socket: RefCell::new(ReqSocket::new()),
            tokio_runtime: rt,

            socket_addr,
            child_thread: RefCell::new(std::thread::spawn(|| {})),
            stop_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        fn start_magpie_process_thread(socket_addr: Arc<str>, stop_requested: Arc<AtomicBool>) -> JoinHandle<()> {
            std::thread::spawn(
                move || {
                    fn spawn_child(socket_addr: &str) -> std::process::Child {
                        match magpie_command(&socket_addr).spawn() {
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

                    let _ = std::fs::remove_file(&socket_addr[6..]);
                    let _ = child.kill();
                }
            )
        }

        if !std::env::var(ENV_MC_DEBUG_MAGPIE_PROCESS_SOCK).is_ok() {
            *self.child_thread.borrow_mut() = start_magpie_process_thread(self.socket_addr.clone(), self.stop_requested.clone());
        }

        const START_WAIT_TIME_MS: u64 = 300;
        const RETRY_COUNT: i32 = 50;

        // Let the child process start up
        for _ in 0..RETRY_COUNT {
            std::thread::sleep(Duration::from_millis(START_WAIT_TIME_MS / 2));

            match self.tokio_runtime.block_on(async {
                self.socket
                    .borrow_mut()
                    .connect(self.socket_addr.as_ref())
                    .await
            }) {
                Ok(_) => return,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Failed to connect to Gatherer socket: {}",
                        e
                    );
                }
            }

            std::thread::sleep(Duration::from_millis(START_WAIT_TIME_MS / 2));
        }

        show_error_dialog_and_exit("Failed to connect to Gatherer socket");
    }

    pub fn stop(&self) {
        self.stop_requested.store(true, Ordering::Relaxed);
        let child_thread = std::mem::replace(&mut *self.child_thread.borrow_mut(), std::thread::spawn(|| {}));
        let _ = child_thread.join();
    }
}

impl Gatherer {
    pub fn set_refresh_interval(&self, _interval: u64) {}

    pub fn set_core_count_affects_percentages(&self, _v: bool) {}

    pub fn cpu_static_info(&self) -> CpuStaticInfo {
        CpuStaticInfo::default()
    }

    pub fn cpu_dynamic_info(&self) -> CpuDynamicInfo {
        CpuDynamicInfo::default()
    }

    pub fn disks_info(&self) -> Vec<DiskInfo> {
        vec![]
    }

    pub fn fans_info(&self) -> Vec<FanInfo> {
        vec![]
    }

    pub fn gpus(&self) -> HashMap<String, Gpu> {
        let mut socket = self.socket.borrow_mut();

        let response = self
            .tokio_runtime
            .block_on(zero_mq_request(
                ipc::req_get_gpus(),
                &mut socket,
                self.socket_addr.as_ref(),
            ))
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

        let response = self
            .tokio_runtime
            .block_on(zero_mq_request(
                ipc::req_get_processes(),
                &mut socket,
                self.socket_addr.as_ref(),
            ))
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

        let response = self
            .tokio_runtime
            .block_on(zero_mq_request(
                ipc::req_get_apps(),
                &mut socket,
                self.socket_addr.as_ref(),
            ))
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

    pub fn services(&self) -> HashMap<Arc<str>, Service> {
        HashMap::new()
    }

    pub fn terminate_process(&self, _pid: u32) {}

    pub fn kill_process(&self, _pid: u32) {}

    pub fn start_service(&self, _service_name: &str) {}

    pub fn stop_service(&self, _service_name: &str) {}

    pub fn restart_service(&self, _service_name: &str) {}

    pub fn enable_service(&self, _service_name: &str) {}

    pub fn disable_service(&self, _service_name: &str) {}

    pub fn get_service_logs(&self, _service_name: &str, _pid: Option<NonZeroU32>) -> Arc<str> {
        Arc::from("")
    }
}
