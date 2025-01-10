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
use std::time::Duration;
use std::{cell::RefCell, collections::HashMap, sync::Arc};

use arrayvec::ArrayString;
use gtk::glib::g_critical;
use magpie_types::ipc::{request, response, Request, Response};
use magpie_types::processes::{processes_response, ProcessesRequest};
pub use magpie_types::processes::{Process, ProcessUsageStats};
use magpie_types::prost::Message;
use zeromq::prelude::*;
use zeromq::ReqSocket;

pub use super::dbus_interface::*;
use crate::show_error_dialog_and_exit;

pub fn random_string<const CAP: usize>() -> ArrayString<CAP> {
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

pub(crate) struct Gatherer {
    socket: RefCell<ReqSocket>,
    tokio_runtime: tokio::runtime::Runtime,

    child: RefCell<Option<std::process::Child>>,
}

impl Drop for Gatherer {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Gatherer {
    pub fn new() -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .expect("Failed to create Tokio runtime");

        Self {
            socket: RefCell::new(ReqSocket::new()),
            tokio_runtime: rt,

            child: RefCell::new(None),
        }
    }

    pub fn start(&self) {
        let socket_addr = format!("ipc:///tmp/magpie_{}.ipc", random_string::<8>());

        let mut command = if crate::is_flatpak() {
            const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

            let mut cmd = std::process::Command::new(FLATPAK_SPAWN_CMD);
            cmd.arg("-v")
                .arg("--watch-bus")
                .arg("--host")
                .arg(Self::executable());
            cmd
        } else {
            let mut cmd = std::process::Command::new(Self::executable());

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
            .arg(&socket_addr);

        self.child.borrow_mut().replace(match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                g_critical!(
                    "MissionCenter::Gatherer",
                    "Failed to spawn Magpie process: {}",
                    &e
                );
                show_error_dialog_and_exit(&format!("Failed to spawn Magpie process: {}", e));
            }
        });

        const START_WAIT_TIME_MS: u64 = 300;
        const RETRY_COUNT: i32 = 50;

        // Let the child process start up
        for i in 0..RETRY_COUNT {
            std::thread::sleep(Duration::from_millis(START_WAIT_TIME_MS / 2));

            match self
                .tokio_runtime
                .block_on(async { self.socket.borrow_mut().connect(&socket_addr).await })
            {
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
        let child = self.child.borrow_mut().take();
        if let Some(mut child) = child {
            // Try to get the child to wake up in case it's stuck
            #[cfg(target_family = "unix")]
            unsafe {
                libc::kill(child.id() as _, libc::SIGCONT);
            }

            let _ = child.kill();
            for _ in 0..2 {
                match child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => {
                        // Wait a bit and try again, the child process might just be slow to stop
                        std::thread::sleep(Duration::from_millis(20));
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
        }
    }

    pub fn is_running(&self) -> Result<(), i32> {
        let mut lock = self.child.borrow_mut();

        let child = match lock.as_mut() {
            Some(child) => child,
            None => return Err(-1),
        };

        let status = match child.try_wait() {
            Ok(None) => return Ok(()),
            Ok(Some(status)) => status,
            Err(_) => {
                return Err(-1);
            }
        };

        match status.code() {
            Some(status_code) => Err(status_code),
            None => Err(-1),
        }
    }

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

    #[allow(unused)]
    pub fn gpu_list(&self) -> Vec<Arc<str>> {
        vec![]
    }

    pub fn gpu_static_info(&self) -> Vec<GpuStaticInfo> {
        vec![]
    }

    pub fn gpu_dynamic_info(&self) -> Vec<GpuDynamicInfo> {
        vec![]
    }

    pub fn processes(&self) -> HashMap<u32, Process> {
        let mut request = Vec::new();

        let encode_res = Request {
            body: Some(request::Body::GetProcesses(ProcessesRequest::default())),
        }
        .encode(&mut request);

        if let Err(e) = encode_res {
            g_critical!("MissionCenter::Gatherer", "Failed to encode request: {}", e);
            return HashMap::new();
        }

        let mut socket = self.socket.borrow_mut();
        let send_res = self
            .tokio_runtime
            .block_on(async { socket.send(request.into()).await });
        match send_res {
            Err(e) => {
                g_critical!("MissionCenter::Gatherer", "Failed to send request: {}", e);
                return HashMap::new();
            }
            _ => {}
        }

        let recv_res = self.tokio_runtime.block_on(async { socket.recv().await });
        let response = match recv_res {
            Ok(response) => response.into_vec(),
            Err(e) => {
                g_critical!(
                    "MissionCenter::Gatherer",
                    "Failed to receive response: {}",
                    e
                );
                return HashMap::new();
            }
        };
        if response.is_empty() {
            g_critical!(
                "MissionCenter::Gatherer",
                "Empty reply when getting processes"
            );
            return HashMap::new();
        }
        let decode_result = if response.len() > 1 {
            Response::decode(response.concat().as_slice())
        } else {
            Response::decode(response[0].iter().as_slice())
        };
        let response = match decode_result {
            Ok(r) => r,
            Err(e) => {
                g_critical!(
                    "MissionCenter::Gatherer",
                    "Error while getting process list: {:?}",
                    e
                );
                return HashMap::new();
            }
        };

        match response.body {
            Some(response::Body::Processes(processes)) => match processes.response {
                Some(processes_response::Response::Processes(mut process_map)) => {
                    std::mem::take(&mut process_map.processes)
                }
                Some(processes_response::Response::Error(e)) => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Error while getting process list: {:?}",
                        e
                    );
                    HashMap::new()
                }
                _ => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Unexpected response: {:?}",
                        processes.response
                    );
                    HashMap::new()
                }
            },
            _ => {
                g_critical!(
                    "MissionCenter::Gatherer",
                    "Unexpected response: {:?}",
                    response
                );
                HashMap::new()
            }
        }
    }

    pub fn apps(&self) -> HashMap<Arc<str>, App> {
        HashMap::new()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gatherer_new() {
        let gatherer = Gatherer::new();
        assert!(gatherer.child.unwrap_or_else(|e| e.into_inner()).is_none());
    }

    #[test]
    fn test_gatherer_start() {
        let gatherer = Gatherer::new();
        let _ = gatherer.start();
        assert!(gatherer.child.unwrap_or_else(|e| e.into_inner()).is_some());
    }

    #[test]
    fn test_gatherer_stop() {
        let gatherer = Gatherer::new();
        let _ = gatherer.start();
        gatherer.stop();
    }
}
