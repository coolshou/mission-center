/* sys_info_v2/mod.rs
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

use std::collections::HashMap;
use std::sync::Arc;

use lazy_static::lazy_static;

use gatherer::Gatherer;
pub use gatherer::{
    App, CpuDynamicInfo, CpuStaticInfo, GpuDynamicInfo, GpuStaticInfo, Process, ProcessUsageStats,
};

macro_rules! cmd {
    ($cmd: expr) => {{
        use std::process::Command;

        if *crate::sys_info_v2::IS_FLATPAK {
            const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

            let mut cmd = Command::new(FLATPAK_SPAWN_CMD);
            cmd.arg("--host").arg("sh").arg("-c");
            cmd.arg($cmd);

            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.arg("-c");
            cmd.arg($cmd);

            cmd
        }
    }};
}

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

mod dbus_interface;
mod disk_info;
mod gatherer;
mod mem_info;
mod net_info;
mod proc_info;

pub type MemInfo = mem_info::MemInfo;
#[allow(dead_code)]
pub type MemoryDevice = mem_info::MemoryDevice;

pub type Disk = disk_info::Disk;
pub type DiskInfo = disk_info::DiskInfo;
pub type DiskType = disk_info::DiskType;

pub type NetInfo = net_info::NetInfo;
pub type NetworkDevice = net_info::NetworkDevice;
pub type NetDeviceType = net_info::NetDeviceType;

pub type Pid = u32;

lazy_static! {
    static ref IS_FLATPAK: bool = std::path::Path::new("/.flatpak-info").exists();
    static ref FLATPAK_APP_PATH: String = {
        use ini::*;

        let ini = match Ini::load_from_file("/.flatpak-info") {
            Err(_) => return "".to_owned(),
            Ok(ini) => ini,
        };

        let section = match ini.section(Some("Instance")) {
            None => panic!("Unable to find Instance section in /.flatpak-info"),
            Some(section) => section,
        };

        match section.get("app-path") {
            None => panic!("Unable to find 'app-path' key in Instance section in /.flatpak-info"),
            Some(app_path) => app_path.to_owned(),
        }
    };
    static ref CACHE_DIR: String = {
        let mut cache_dir = std::env::var("XDG_CACHE_HOME").unwrap_or(
            std::env::var("HOME")
                .and_then(|v| Ok(v + "/.cache"))
                .unwrap_or("/tmp".to_string()),
        );
        cache_dir.push_str("/io.missioncenter.MissionCenter");
        std::fs::create_dir_all(cache_dir.as_str()).unwrap_or(());
        cache_dir
    };
    static ref STATE_DIR: String = {
        let state_dir = if *IS_FLATPAK {
            let mut cache_dir = std::env::var("XDG_CACHE_HOME").unwrap_or(
                std::env::var("HOME")
                    .and_then(|v| Ok(v + "/.var/app/io.missioncenter.MissionCenter/.local"))
                    .unwrap_or("/tmp/io.missioncenter.MissionCenter/.local".to_string()),
            );
            cache_dir.push_str("/../.local/state");

            match std::fs::create_dir_all(cache_dir.as_str()) {
                Err(e) => {
                    gtk::glib::g_critical!(
                        "MissionCenter::SysInfo",
                        "Unable to create state dir: {}",
                        e
                    );
                    "/tmp/io.missioncenter.MissionCenter/state".to_string()
                }
                _ => std::path::Path::new(cache_dir.as_str())
                    .canonicalize()
                    .unwrap_or(std::path::PathBuf::from(
                        "/tmp/io.missioncenter.MissionCenter/state",
                    ))
                    .to_string_lossy()
                    .to_string(),
            }
        } else {
            std::env::var("HOME")
                .and_then(|mut v| {
                    Ok({
                        v.push_str("/.local/state/io.missioncenter.MissionCenter");
                        v
                    })
                })
                .unwrap_or("/tmp/io.missioncenter.MissionCenter/state".to_string())
        };

        std::fs::create_dir_all(state_dir.as_str()).unwrap_or(());
        state_dir
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSpeed {
    VerySlow = 4,
    Slow = 3,
    Normal = 2,
    Fast = 1,
}

#[derive(Debug)]
pub enum TerminateType {
    Normal,
    Force,
}

impl From<i32> for UpdateSpeed {
    fn from(value: i32) -> Self {
        use gtk::glib::*;

        match value {
            1 => Self::Fast,
            2 => Self::Normal,
            3 => Self::Slow,
            4 => Self::VerySlow,
            _ => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Invalid update speed value: {}, defaulting to Normal",
                    value
                );
                Self::Normal
            }
        }
    }
}

#[derive(Debug)]
pub struct Readings {
    pub cpu_static_info: CpuStaticInfo,
    pub cpu_dynamic_info: CpuDynamicInfo,
    pub mem_info: MemInfo,
    pub disks: Vec<Disk>,
    pub network_devices: Vec<NetworkDevice>,
    pub gpu_static_info: Vec<GpuStaticInfo>,
    pub gpu_dynamic_info: Vec<GpuDynamicInfo>,

    pub running_apps: HashMap<Arc<str>, App>,
    pub process_tree: Process,
}

impl Readings {
    pub fn new() -> Self {
        Self {
            cpu_static_info: Default::default(),
            cpu_dynamic_info: Default::default(),
            mem_info: MemInfo::default(),
            disks: vec![],
            network_devices: vec![],
            gpu_static_info: vec![],
            gpu_dynamic_info: vec![],

            running_apps: HashMap::new(),
            process_tree: Process::default(),
        }
    }
}

pub struct SysInfoV2 {
    refresh_interval: Arc<std::sync::atomic::AtomicU8>,
    merged_process_stats: Arc<std::sync::atomic::AtomicBool>,

    refresh_thread: Option<std::thread::JoinHandle<()>>,
    refresh_thread_running: Arc<std::sync::atomic::AtomicBool>,

    sender: std::sync::mpsc::Sender<()>,
}

impl Drop for SysInfoV2 {
    fn drop(&mut self) {
        use std::sync::*;
        self.refresh_thread_running
            .store(false, atomic::Ordering::Release);

        if let Some(refresh_thread) = std::mem::take(&mut self.refresh_thread) {
            refresh_thread
                .join()
                .expect("Unable to stop the refresh thread");
        }
    }
}

impl Default for SysInfoV2 {
    fn default() -> Self {
        use std::sync::*;

        let (tx, _) = mpsc::channel::<()>();

        Self {
            refresh_interval: Arc::new(0.into()),
            merged_process_stats: Arc::new(false.into()),

            refresh_thread: None,
            refresh_thread_running: Arc::new(true.into()),

            sender: tx,
        }
    }
}

impl SysInfoV2 {
    pub fn new() -> Self {
        use std::sync::{atomic::*, *};

        let refresh_interval = Arc::new(AtomicU8::new(UpdateSpeed::Normal as u8));
        let merged_process_stats = Arc::new(AtomicBool::new(false));
        let refresh_thread_running = Arc::new(AtomicBool::new(true));

        let ri = refresh_interval.clone();
        let mps = merged_process_stats.clone();
        let run = refresh_thread_running.clone();

        let (tx, rx) = mpsc::channel::<()>();
        Self {
            refresh_interval,
            merged_process_stats,
            refresh_thread: Some(std::thread::spawn(move || {
                use gtk::glib::*;

                let gatherer = Gatherer::new();

                let mut readings = Readings::new();
                readings.process_tree = proc_info::process_hierarchy(
                    &gatherer.processes(),
                    mps.load(Ordering::Acquire),
                )
                .unwrap_or_default();
                readings.running_apps = gatherer.apps();

                let mut disk_stats = vec![];
                readings.disks = DiskInfo::load(&mut disk_stats);

                let mut net_info = NetInfo::new();
                readings.network_devices = if let Some(net_info) = net_info.as_mut() {
                    net_info.load_devices()
                } else {
                    vec![]
                };

                readings.mem_info = MemInfo::load().unwrap_or(MemInfo::default());

                readings.cpu_static_info = gatherer.cpu_static_info();
                let cpu_static_info = readings.cpu_static_info.clone();
                readings.cpu_dynamic_info = gatherer.cpu_dynamic_info();

                let gpu_ids = gatherer.enumerate_gpus();
                readings.gpu_static_info = gpu_ids
                    .iter()
                    .map(|id| gatherer.gpu_static_info(id.as_ref()))
                    .collect();
                let gpu_static_info = readings.gpu_static_info.clone();
                readings.gpu_dynamic_info = gpu_ids
                    .iter()
                    .map(|id| gatherer.gpu_dynamic_info(id.as_ref()))
                    .collect();

                idle_add_once(move || {
                    use gtk::glib::*;

                    if let Some(app) = crate::MissionCenterApplication::default_instance() {
                        app.set_initial_readings(readings);
                    } else {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Default GtkApplication is not a MissionCenterApplication; failed to set initial readings"
                        );
                    }
                });

                'read_loop: while run.load(Ordering::Acquire) {
                    let loop_start = std::time::Instant::now();

                    let timer = std::time::Instant::now();
                    let processes = gatherer.processes();
                    g_debug!(
                        "MissionCenter::Perf",
                        "Process load took: {:?}",
                        timer.elapsed()
                    );

                    let timer = std::time::Instant::now();
                    let merged_stats = mps.load(Ordering::Acquire);
                    let process_tree =
                        proc_info::process_hierarchy(&processes, merged_stats).unwrap_or_default();
                    g_debug!(
                        "MissionCenter::Perf",
                        "Process tree load took: {:?}",
                        timer.elapsed()
                    );

                    let timer = std::time::Instant::now();
                    let running_apps = gatherer.apps();
                    g_debug!(
                        "MissionCenter::Perf",
                        "App load took: {:?}",
                        timer.elapsed()
                    );

                    let timer = std::time::Instant::now();
                    let cpu_dynamic_info = gatherer.cpu_dynamic_info();
                    g_debug!(
                        "MissionCenter::Perf",
                        "CPU load took: {:?}",
                        timer.elapsed()
                    );

                    let timer = std::time::Instant::now();
                    let mem_info = MemInfo::load().unwrap_or(MemInfo::default());
                    g_debug!(
                        "MissionCenter::Perf",
                        "Mem load took: {:?}",
                        timer.elapsed()
                    );

                    let timer = std::time::Instant::now();
                    let disks = DiskInfo::load(&mut disk_stats);
                    g_debug!(
                        "MissionCenter::Perf",
                        "Disk load took: {:?}",
                        timer.elapsed()
                    );

                    let timer = std::time::Instant::now();
                    let network_devices = if let Some(net_info) = net_info.as_mut() {
                        net_info.load_devices()
                    } else {
                        vec![]
                    };
                    g_debug!(
                        "MissionCenter::Perf",
                        "Net load took: {:?}",
                        timer.elapsed()
                    );

                    let timer = std::time::Instant::now();
                    let gpu_dynamic_info = gpu_ids
                        .iter()
                        .map(|id| gatherer.gpu_dynamic_info(id.as_ref()))
                        .collect();
                    g_debug!(
                        "MissionCenter::Perf",
                        "GPU load took: {:?}",
                        timer.elapsed()
                    );

                    let mut readings = Readings {
                        cpu_static_info: cpu_static_info.clone(),
                        cpu_dynamic_info,
                        mem_info,
                        disks,
                        network_devices,
                        gpu_static_info: gpu_static_info.clone(),
                        gpu_dynamic_info,

                        running_apps,
                        process_tree,
                    };

                    g_debug!(
                        "MissionCenter::Perf",
                        "Loaded readings in {}ms",
                        loop_start.elapsed().as_millis()
                    );

                    let time = std::time::Instant::now();
                    let refresh_interval = ri.clone().load(Ordering::Acquire) as usize * 500;
                    let refresh_interval =
                        std::time::Duration::from_millis(refresh_interval as u64);
                    g_debug!(
                        "MissionCenter::Perf",
                        "Refresh interval ({:?}) read in {:?}",
                        refresh_interval,
                        time.elapsed()
                    );
                    let elapsed = loop_start.elapsed();

                    if elapsed > refresh_interval {
                        g_warning!(
                            "MissionCenter::SysInfo",
                            "Refresh took {}ms, which is longer than the refresh interval of {}ms",
                            elapsed.as_millis(),
                            refresh_interval.as_millis()
                        );
                    }

                    let mut sleep_duration = refresh_interval.saturating_sub(elapsed);
                    let sleep_duration_fraction = sleep_duration / 10;
                    for _ in 0..10 {
                        let timer = std::time::Instant::now();

                        match rx.recv_timeout(sleep_duration_fraction) {
                            Ok(message) => {} //gatherer_supervisor.send_message(message),
                            Err(e) => {
                                if e != mpsc::RecvTimeoutError::Timeout {
                                    g_warning!(
                                        "MissionCenter::SysInfo",
                                        "Error receiving message from gatherer: {}",
                                        e
                                    );
                                }
                            }
                        }

                        if !run.load(Ordering::Acquire) {
                            break 'read_loop;
                        }

                        sleep_duration = sleep_duration.saturating_sub(timer.elapsed());
                        if sleep_duration.as_millis() == 0 {
                            break;
                        }
                    }
                    std::thread::sleep(sleep_duration);

                    idle_add_once(move || {
                        use gtk::glib::*;

                        if let Some(app) = crate::MissionCenterApplication::default_instance() {
                            let now = std::time::Instant::now();

                            let timer = std::time::Instant::now();
                            if !app.refresh_readings(&mut readings) {
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
                                "Refreshed readings in {}ms",
                                now.elapsed().as_millis()
                            );
                        } else {
                            g_critical!(
                                "MissionCenter::SysInfo",
                                "Default GtkApplication is not a MissionCenterApplication"
                            );
                        }
                    });

                    g_debug!(
                        "MissionCenter::Perf",
                        "Read-refresh loop executed in: {}ms",
                        loop_start.elapsed().as_millis()
                    );
                }
            })),
            refresh_thread_running,
            sender: tx,
        }
    }

    pub fn set_update_speed(&self, speed: UpdateSpeed) {
        self.refresh_interval
            .store(speed as u8, std::sync::atomic::Ordering::Release);
    }

    pub fn set_merged_process_stats(&self, merged_stats: bool) {
        self.merged_process_stats
            .store(merged_stats, std::sync::atomic::Ordering::Release);
    }

    pub fn terminate_process(&self, terminate_type: TerminateType, pid: u32) {
        // use gtk::glib::g_critical;
        //
        // match terminate_type {
        //     TerminateType::Normal => {
        //         match self.sender.send(gatherer::Message::TerminateProcess(pid)) {
        //             Err(e) => {
        //                 g_critical!(
        //                     "MissionCenter::SysInfo",
        //                     "Error sending TerminateProcess({}) to gatherer: {}",
        //                     pid,
        //                     e
        //                 );
        //             }
        //             _ => {}
        //         }
        //     }
        //     TerminateType::Force => match self.sender.send(gatherer::Message::KillProcess(pid)) {
        //         Err(e) => {
        //             g_critical!(
        //                 "MissionCenter::SysInfo",
        //                 "Error sending KillProcess({}) to gatherer: {}",
        //                 pid,
        //                 e
        //             );
        //         }
        //         _ => {}
        //     },
        // }
    }
}
