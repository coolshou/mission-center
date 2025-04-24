/* magpie_client/mod.rs
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

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::atomic;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use gtk::glib::{g_critical, g_debug, g_warning, idle_add_once};

use crate::app;
use crate::application::{BASE_INTERVAL, INTERVAL_STEP};

pub use client::{
    App, Client, Connection, Cpu, Disk, DiskKind, ErrorEjectFailed, Fan, Gpu, Memory, MemoryDevice,
    Process, Service, SmartData,
};
use magpie_types::processes::processes_response::process_map::NetworkError;

macro_rules! cmd_flatpak_host {
    ($cmd: expr) => {{
        use std::process::Command;

        const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

        let mut cmd = Command::new(FLATPAK_SPAWN_CMD);
        cmd.arg("--host").arg("sh").arg("-c");
        cmd.arg($cmd);

        cmd
    }};
}

mod client;

pub type Pid = u32;

fn flatpak_app_path() -> &'static str {
    static FLATPAK_APP_PATH: OnceLock<String> = OnceLock::new();

    FLATPAK_APP_PATH
        .get_or_init(|| {
            let ini = match ini::Ini::load_from_file("/.flatpak-info") {
                Err(_) => return "".to_owned(),
                Ok(ini) => ini,
            };

            let section = match ini.section(Some("Instance")) {
                None => panic!("Unable to find `Instance` section in `/.flatpak-info`"),
                Some(section) => section,
            };

            match section.get("app-path") {
                None => {
                    panic!("Unable to find `app-path` key in section `Instance` missing from `/.flatpak-info`")
                }
                Some(app_path) => app_path.to_owned(),
            }
        })
        .as_str()
}

enum Message {
    ContinueReading,
    UpdateCoreCountAffectsPercentages(bool),
    TerminateProcesses(Vec<Pid>),
    KillProcesses(Vec<Pid>),
    GetServiceLogs(String, Option<NonZeroU32>),
    StartService(String),
    StopService(String),
    RestartService(String),
    EnableService(String),
    DisableService(String),
    EjectDisk(String),
    SmartData(String),
}

enum Response {
    String(String),
    EjectResult(Result<(), ErrorEjectFailed>),
    SmartData(Option<SmartData>),
}

#[derive(Debug)]
pub struct Readings {
    pub cpu: Cpu,
    pub mem_info: Memory,
    pub mem_devices: Vec<MemoryDevice>,
    pub disks_info: Vec<Disk>,
    pub network_connections: Vec<Connection>,
    pub gpus: HashMap<String, Gpu>,
    pub fans: Vec<Fan>,

    pub running_apps: HashMap<String, App>,
    pub running_processes: HashMap<u32, Process>,

    pub network_status: Option<NetworkError>,

    pub services: HashMap<String, Service>,
}

impl Readings {
    pub fn new() -> Self {
        Self {
            cpu: Default::default(),
            mem_info: Memory::default(),
            mem_devices: vec![],
            disks_info: vec![],
            network_connections: vec![],
            gpus: HashMap::new(),
            fans: vec![],

            running_apps: HashMap::new(),
            running_processes: HashMap::new(),
            network_status: None,

            services: HashMap::new(),
        }
    }
}

pub struct MagpieClient {
    speed: Arc<AtomicU64>,

    refresh_thread: Option<std::thread::JoinHandle<()>>,
    refresh_thread_running: Arc<AtomicBool>,

    sender: Sender<Message>,
    receiver: Receiver<Response>,
}

impl Drop for MagpieClient {
    fn drop(&mut self) {
        self.refresh_thread_running
            .store(false, atomic::Ordering::Release);

        if let Some(refresh_thread) = std::mem::take(&mut self.refresh_thread) {
            refresh_thread
                .join()
                .expect("Unable to stop the refresh thread");
        }
    }
}

impl Default for MagpieClient {
    fn default() -> Self {
        let (tx, _) = mpsc::channel::<Message>();
        let (_, resp_rx) = mpsc::channel::<Response>();

        Self {
            speed: Arc::new(0.into()),

            refresh_thread: None,
            refresh_thread_running: Arc::new(true.into()),

            sender: tx,
            receiver: resp_rx,
        }
    }
}

impl MagpieClient {
    pub fn new() -> Self {
        let speed = Arc::new(AtomicU64::new(
            (BASE_INTERVAL / INTERVAL_STEP).round() as u64
        ));
        let refresh_thread_running = Arc::new(AtomicBool::new(true));

        let s = speed.clone();
        let run = refresh_thread_running.clone();

        let (tx, rx) = mpsc::channel::<Message>();
        let (resp_tx, resp_rx) = mpsc::channel::<Response>();
        Self {
            speed,
            refresh_thread: Some(std::thread::spawn(move || {
                Self::gather_and_proxy(rx, resp_tx, run, s);
            })),
            refresh_thread_running,
            sender: tx,
            receiver: resp_rx,
        }
    }

    pub fn set_update_speed(&self, speed: u64) {
        self.speed.store(speed, atomic::Ordering::Release);
    }

    pub fn set_core_count_affects_percentages(&self, show: bool) {
        match self
            .sender
            .send(Message::UpdateCoreCountAffectsPercentages(show))
        {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending UpdateCoreCountAffectsPercentages to Gatherer: {e}"
                );
            }
            _ => {}
        }
    }

    pub fn continue_reading(&self) {
        match self.sender.send(Message::ContinueReading) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending ContinueReading to gatherer: {}",
                    e
                );
            }
            _ => {}
        }
    }

    #[inline(always)]
    pub fn terminate_process(&self, pid: u32) {
        self.terminate_processes(vec![pid]);
    }

    pub fn terminate_processes(&self, pids: Vec<u32>) {
        match self.sender.send(Message::TerminateProcesses(pids)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending TerminateProcesses to gatherer: {e}",
                );
            }
            _ => {}
        }
    }

    #[inline(always)]
    pub fn kill_process(&self, pid: u32) {
        self.kill_processes(vec![pid]);
    }

    pub fn kill_processes(&self, pids: Vec<u32>) {
        match self.sender.send(Message::KillProcesses(pids)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending KillProcesses to gatherer: {e}",
                );
            }
            _ => {}
        }
    }

    pub fn service_logs(&self, service_id: String, pid: Option<NonZeroU32>) -> String {
        let sid = service_id.clone();
        match self.sender.send(Message::GetServiceLogs(service_id, pid)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending GetServiceLogs({sid}) to gatherer: {e}",
                );

                return String::new();
            }
            _ => {}
        }

        match self.receiver.recv() {
            Ok(Response::String(logs)) => logs,
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error receiving GetServiceLogs response: {}",
                    e
                );
                String::new()
            }
            _ => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error receiving GetServiceLogs response. Wrong type"
                );

                String::new()
            }
        }
    }

    pub fn start_service(&self, service_id: String) {
        let sid = service_id.clone();
        match self.sender.send(Message::StartService(service_id)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending StartService({sid}) to gatherer: {e}",
                );
            }
            _ => {}
        }
    }

    pub fn stop_service(&self, service_id: String) {
        let sid = service_id.clone();
        match self.sender.send(Message::StopService(service_id)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending StopService({sid}) to gatherer: {e}",
                );
            }
            _ => {}
        }
    }

    pub fn restart_service(&self, service_id: String) {
        let sid = service_id.clone();
        match self.sender.send(Message::RestartService(service_id)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending RestartService({sid}) to gatherer: {e}",
                );
            }
            _ => {}
        }
    }

    pub fn enable_service(&self, service_id: String) {
        let sid = service_id.clone();
        match self.sender.send(Message::EnableService(service_id)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending EnableService({sid}) to gatherer: {e}",
                );
            }
            _ => {}
        }
    }

    pub fn disable_service(&self, service_id: String) {
        let sid = service_id.clone();
        match self.sender.send(Message::DisableService(service_id)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending DisableService({sid}) to gatherer: {e}",
                );
            }
            _ => {}
        }
    }

    pub fn eject_disk(&self, disk_id: &str) -> Result<(), ErrorEjectFailed> {
        match self.sender.send(Message::EjectDisk(disk_id.to_owned())) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending EjectDisk({}) to gatherer: {}",
                    disk_id,
                    e
                );

                return Ok(());
            }
            _ => {}
        }

        match self.receiver.recv() {
            Ok(Response::EjectResult(res)) => res,
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error receiving EjectDisk response: {}",
                    e
                );
                Ok(())
            }
            _ => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error receiving EjectDisk response. Wrong type"
                );
                Ok(())
            }
        }
    }

    pub fn smart_data(&self, disk_id: String) -> Option<SmartData> {
        let did = disk_id.clone();
        match self.sender.send(Message::SmartData(disk_id)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending SataSmartInfo({did}) to gatherer: {e}",
                );

                return None;
            }
            _ => {}
        }

        match self.receiver.recv() {
            Ok(Response::SmartData(sd)) => sd,
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error receiving SataSmartResult response: {e}",
                );
                None
            }
            _ => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error receiving SataSmartResult response. Wrong type"
                );
                None
            }
        }
    }
}

impl MagpieClient {
    fn handle_incoming_message(
        magpie: &Client,
        rx: &mut Receiver<Message>,
        tx: &mut Sender<Response>,
        timeout: Duration,
    ) -> bool {
        match rx.recv_timeout(timeout) {
            Ok(message) => match message {
                Message::ContinueReading => {
                    g_warning!(
                        "MissionCenter::SysInfo",
                        "Received ContinueReading message while not reading"
                    );
                }
                Message::UpdateCoreCountAffectsPercentages(show) => {
                    magpie.set_scale_cpu_usage_to_core_count(show);
                }
                Message::TerminateProcesses(pid) => {
                    magpie.terminate_processes(pid);
                }
                Message::KillProcesses(pids) => {
                    magpie.kill_processes(pids);
                }
                Message::StartService(name) => {
                    magpie.start_service(name);
                }
                Message::StopService(name) => {
                    magpie.stop_service(name);
                }
                Message::RestartService(name) => {
                    magpie.restart_service(name);
                }
                Message::EnableService(name) => {
                    magpie.enable_service(name);
                }
                Message::DisableService(name) => {
                    magpie.disable_service(name);
                }
                Message::GetServiceLogs(name, pid) => {
                    let resp = magpie.service_logs(name, pid);
                    if let Err(e) = tx.send(Response::String(resp)) {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Error sending GetServiceLogs response: {}",
                            e
                        );
                    }
                }
                Message::EjectDisk(disk_id) => {
                    if let Err(e) = tx.send(Response::EjectResult(magpie.eject_disk(disk_id))) {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Error sending EjectDisk response: {e}",
                        );
                    }
                }
                Message::SmartData(disk_id) => {
                    if let Err(e) = tx.send(Response::SmartData(magpie.smart_data(disk_id))) {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Error sending SataSmartInfo response: {}",
                            e
                        );
                    }
                }
            },
            Err(_) => {}
        }

        true
    }

    fn gather_and_proxy(
        mut rx: Receiver<Message>,
        mut tx: Sender<Response>,
        running: Arc<AtomicBool>,
        speed: Arc<AtomicU64>,
    ) {
        let magpie = Client::new();
        magpie.start();

        let (running_processes, network_status) = magpie.processes();
        let mut readings = Readings {
            running_processes,
            network_status,
            running_apps: magpie.apps(),
            disks_info: magpie.disks_info(),
            gpus: magpie.gpus(),
            cpu: magpie.cpu(),
            mem_info: magpie.memory(),
            mem_devices: magpie.memory_devices(),
            fans: magpie.fans_info(),
            network_connections: magpie.network_connections(),
            services: magpie.services(),
        };

        readings
            .disks_info
            .sort_unstable_by(|d1, d2| d1.id.cmp(&d2.id));
        readings
            .network_connections
            .sort_unstable_by(|n1, n2| n1.id.cmp(&n2.id));

        idle_add_once({
            let initial_readings = Readings {
                cpu: readings.cpu.clone(),
                mem_info: readings.mem_info.clone(),
                mem_devices: std::mem::take(&mut readings.mem_devices),
                disks_info: std::mem::take(&mut readings.disks_info),
                fans: std::mem::take(&mut readings.fans),
                network_connections: std::mem::take(&mut readings.network_connections),
                gpus: std::mem::take(&mut readings.gpus),
                running_apps: std::mem::take(&mut readings.running_apps),
                running_processes: std::mem::take(&mut readings.running_processes),
                network_status: std::mem::take(&mut readings.network_status),
                services: std::mem::take(&mut readings.services),
            };

            move || {
                app!().set_initial_readings(initial_readings);
                app!().setup_animations();
            }
        });

        loop {
            match rx.recv() {
                Ok(message) => match message {
                    Message::ContinueReading => {
                        break;
                    }
                    Message::UpdateCoreCountAffectsPercentages(show) => {
                        magpie.set_scale_cpu_usage_to_core_count(show);
                    }
                    _ => {}
                },
                Err(_) => {
                    g_warning!(
                        "MissionCenter::SysInfo",
                        "No more messages in the buffer and channel closed",
                    );
                    return;
                }
            }
        }

        'read_loop: while running.load(atomic::Ordering::Acquire) {
            let loop_start = std::time::Instant::now();

            let timer = std::time::Instant::now();
            (readings.running_processes, readings.network_status) = magpie.processes();
            g_debug!(
                "MissionCenter::Perf",
                "Process load load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.running_apps = magpie.apps();
            g_debug!(
                "MissionCenter::Perf",
                "Running apps load took: {:?}",
                timer.elapsed(),
            );

            let timer = std::time::Instant::now();
            readings.disks_info = magpie.disks_info();
            g_debug!(
                "MissionCenter::Perf",
                "Disks info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.gpus = magpie.gpus();
            g_debug!(
                "MissionCenter::Perf",
                "GPU info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.cpu = magpie.cpu();
            g_debug!(
                "MissionCenter::Perf",
                "CPU info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.mem_info = magpie.memory();
            g_debug!(
                "MissionCenter::Perf",
                "Memory info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.network_connections = magpie.network_connections();
            g_debug!(
                "MissionCenter::Perf",
                "Network devices info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.fans = magpie.fans_info();
            g_debug!(
                "MissionCenter::Perf",
                "Fans info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.services = magpie.services();
            g_debug!(
                "MissionCenter::Perf",
                "Services load took: {:?}",
                timer.elapsed()
            );

            readings
                .disks_info
                .sort_unstable_by(|d1, d2| d1.id.cmp(&d2.id));
            readings
                .network_connections
                .sort_unstable_by(|n1, n2| n1.id.cmp(&n2.id));

            if !running.load(atomic::Ordering::Acquire) {
                break 'read_loop;
            }

            idle_add_once({
                let mut new_readings = Readings {
                    cpu: readings.cpu.clone(),
                    mem_info: readings.mem_info.clone(),
                    mem_devices: readings.mem_devices.clone(),
                    disks_info: std::mem::take(&mut readings.disks_info),
                    fans: std::mem::take(&mut readings.fans),
                    network_connections: std::mem::take(&mut readings.network_connections),
                    gpus: std::mem::take(&mut readings.gpus),
                    running_apps: std::mem::take(&mut readings.running_apps),
                    running_processes: std::mem::take(&mut readings.running_processes),
                    network_status: std::mem::take(&mut readings.network_status),
                    services: std::mem::take(&mut readings.services),
                };

                move || {
                    let app = app!();
                    let now = std::time::Instant::now();
                    let timer = std::time::Instant::now();
                    if !app.refresh_readings(&mut new_readings) {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Readings were not completely refreshed, stale readings will be displayed"
                        );
                    }
                    g_debug!(
                        "MissionCenter::Perf",
                        "UI refresh took: {:?}",
                        timer.elapsed()
                    );
                    g_debug!(
                        "MissionCenter::SysInfo",
                        "Refreshed readings in {:?}",
                        now.elapsed()
                    );
                }
            });

            let mut wait_time = Duration::from_millis(
                ((speed.load(atomic::Ordering::Relaxed) as f64 * INTERVAL_STEP) * 1000.) as u64,
            )
            .saturating_sub(loop_start.elapsed());

            const ITERATIONS_COUNT: u32 = 10;

            let wait_time_fraction = wait_time / ITERATIONS_COUNT;
            for _ in 0..ITERATIONS_COUNT {
                let wait_timer = std::time::Instant::now();

                if !Self::handle_incoming_message(&magpie, &mut rx, &mut tx, wait_time_fraction) {
                    break 'read_loop;
                }

                if !running.load(atomic::Ordering::Acquire) {
                    break 'read_loop;
                }

                wait_time = wait_time.saturating_sub(wait_timer.elapsed());
                if wait_time.is_zero() {
                    break;
                }
            }

            if !Self::handle_incoming_message(&magpie, &mut rx, &mut tx, wait_time) {
                break 'read_loop;
            }

            let elapsed_since_start = loop_start.elapsed();
            g_debug!(
                "MissionCenter::Perf",
                "Full read-publish cycle took {elapsed_since_start:?}",
            );
        }
    }
}
