/* sys_info/view_models
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

use lazy_static::lazy_static;

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

mod app_info;
mod cpu_info;
mod disk_info;
mod gatherer;
mod gpu_info;
mod mem_info;
mod net_info;
mod proc_info;

#[allow(dead_code)]
pub type CpuInfo = cpu_info::CpuInfo;
#[allow(dead_code)]
pub type CpuInfoStatic = cpu_info::StaticInfo;
#[allow(dead_code)]
pub type CpuInfoDynamic = cpu_info::DynamicInfo;

pub type MemInfo = mem_info::MemInfo;
#[allow(dead_code)]
pub type MemoryDevice = mem_info::MemoryDevice;

pub type Disk = disk_info::Disk;
pub type DiskInfo = disk_info::DiskInfo;
pub type DiskType = disk_info::DiskType;

pub type NetInfo = net_info::NetInfo;
pub type NetworkDevice = net_info::NetworkDevice;
pub type NetDeviceType = net_info::NetDeviceType;

#[allow(dead_code)]
pub type GPUStaticInformation = gpu_info::StaticInfo;
#[allow(dead_code)]
pub type GPUDynamicInformation = gpu_info::DynamicInfo;
pub type GPU = gpu_info::GPU;
pub type GPUInfo = gpu_info::GPUInfo;

pub type App = app_info::App;
pub type Process = proc_info::Process;
#[allow(dead_code)]
pub type ProcessStats = proc_info::Stats;
pub type Pid = proc_info::Pid;

lazy_static! {
    static ref IS_FLATPAK: bool = std::path::Path::new("/.flatpak-info").exists();
    static ref FLATPAK_APP_PATH: String = {
        use ini::*;

        let ini = Ini::load_from_file("/.flatpak-info");
        if ini.is_err() {
            return "".to_owned();
        }
        let ini = ini.unwrap();

        let section = ini.section(Some("Instance"));
        if section.is_none() {
            panic!("Unable to find Instance section in /.flatpak-info");
        }
        let section = section.unwrap();

        let app_path = section.get("app-path");
        if app_path.is_none() {
            panic!("Unable to find 'app-path' key in Instance section in /.flatpak-info");
        }
        app_path.unwrap().to_owned()
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
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSpeed {
    VerySlow = 4,
    Slow = 3,
    Normal = 2,
    Fast = 1,
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
    pub cpu_info: CpuInfo,
    pub mem_info: MemInfo,
    pub disks: Vec<Disk>,
    pub network_devices: Vec<NetworkDevice>,
    pub gpus: Vec<GPU>,

    pub running_apps: std::collections::HashMap<String, App>,
    pub process_tree: Process,
}

impl Readings {
    pub fn new(system: &mut sysinfo::System) -> Self {
        use std::collections::HashMap;

        let mut daemon =
            gatherer::Gatherer::<gatherer::SharedData>::new("/home/kicsyromy/Workspace/MissionCenter/build-meson-debug/src/sys_info_v2/gatherer/missioncenter-gatherer").unwrap();
        daemon.start().unwrap();
        assert!(daemon.is_running().is_ok());

        loop {
            if let Err(e) = daemon.send_message(gatherer::Message::GetInstalledApps) {
                eprintln!("Failed to send message to daemon: {:#?}", e);
                eprintln!("stdout: {:#?}", daemon.stdout());
                eprintln!("stderr: {:#?}", daemon.stderr());

                if let Err(e) = daemon.is_running() {
                    eprintln!("Daemon is no longer running: {:#?}. Restarting...", e);
                    daemon.start().unwrap();

                    continue;
                } else {
                    // TODO: Handle this better
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            dbg!(&daemon.shared_memory().unwrap().content);
            daemon.send_message(gatherer::Message::Exit).unwrap();
            break;
        }

        Self {
            cpu_info: CpuInfo::new(system),
            mem_info: MemInfo::load().expect("Unable to get memory info"),
            disks: vec![],
            network_devices: vec![],
            gpus: vec![],

            running_apps: HashMap::new(),
            process_tree: Process::default(),
        }
    }
}

pub struct SysInfoV2 {
    #[allow(dead_code)]
    refresh_interval: std::sync::Arc<std::sync::atomic::AtomicU8>,
    refresh_thread: Option<std::thread::JoinHandle<()>>,
    refresh_thread_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
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

impl SysInfoV2 {
    pub fn new() -> (Self, Readings) {
        use gtk::glib::*;
        use std::sync::{atomic::*, *};
        use sysinfo::{System, SystemExt};

        let mut system = System::new();
        let mut readings = Readings::new(&mut system);

        let mut disk_stats = vec![];
        readings.disks = DiskInfo::load(&mut disk_stats);

        let mut net_info = NetInfo::new();
        readings.network_devices = if let Some(net_info) = net_info.as_mut() {
            net_info.load_devices()
        } else {
            vec![]
        };

        let mut gpu_info = GPUInfo::new();
        readings.gpus = if let Some(gpu_info) = gpu_info.as_mut() {
            gpu_info.load_gpus()
        } else {
            vec![]
        };

        let (apps, mut processes) = proc_info::load_app_and_process_list();
        for gpu in &readings.gpus {
            for (pid, gpu_usage) in &gpu.dynamic_info.processes {
                if let Some(process) = processes.get_mut(pid) {
                    process.stats.gpu_usage = *gpu_usage;
                }
            }
        }
        readings.process_tree = proc_info::process_hierarchy(&processes).unwrap_or_default();
        readings.running_apps = app_info::running_apps(&readings.process_tree, &apps);

        let refresh_interval = Arc::new(AtomicU8::new(UpdateSpeed::Normal as u8));
        let refresh_thread_running = Arc::new(std::sync::atomic::AtomicBool::new(true));

        let ri = refresh_interval.clone();
        let run = refresh_thread_running.clone();
        let cpu_static_info = readings.cpu_info.static_info.clone();
        (
            Self {
                refresh_interval,
                refresh_thread: Some(std::thread::spawn(move || {
                    let mut system = system;
                    let mut previous_disk_stats = disk_stats;
                    let mut net_info = net_info;
                    let mut gpu_info = gpu_info;

                    'read_loop: while run.load(Ordering::Acquire) {
                        let start_load_readings = std::time::Instant::now();

                        let cpu_info = CpuInfoDynamic::load(&mut system);
                        let mem_info = MemInfo::load().unwrap_or(MemInfo::default());
                        let disks = DiskInfo::load(&mut previous_disk_stats);
                        let network_devices = if let Some(net_info) = net_info.as_mut() {
                            net_info.load_devices()
                        } else {
                            vec![]
                        };
                        let gpus = if let Some(gpu_info) = gpu_info.as_mut() {
                            gpu_info.load_gpus()
                        } else {
                            vec![]
                        };

                        let (apps, mut process_list) = proc_info::load_app_and_process_list();
                        for gpu in &gpus {
                            for (pid, gpu_usage) in &gpu.dynamic_info.processes {
                                if let Some(process) = process_list.get_mut(pid) {
                                    process.stats.gpu_usage = *gpu_usage;
                                }
                            }
                        }
                        let process_tree =
                            proc_info::process_hierarchy(&process_list).unwrap_or_default();
                        let running_apps = app_info::running_apps(&process_tree, &apps);

                        let mut readings = Readings {
                            cpu_info: CpuInfo {
                                static_info: cpu_static_info.clone(),
                                dynamic_info: cpu_info,
                            },
                            mem_info,
                            disks,
                            network_devices,
                            gpus,

                            running_apps,
                            process_tree,
                        };

                        g_debug!(
                            "MissionCenter::SysInfo",
                            "Loaded readings in {}ms",
                            start_load_readings.elapsed().as_millis()
                        );

                        idle_add_once(move || {
                            use gtk::glib::*;

                            if let Some(app) = crate::MissionCenterApplication::default_instance() {
                                let now = std::time::Instant::now();

                                if !app.refresh_readings(&mut readings) {
                                    g_critical!(
                                        "MissionCenter::SysInfo",
                                        "Readings were not completely refreshed, stale readings will be displayed"
                                    );
                                }

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

                        let refresh_interval = ri.clone().load(Ordering::Acquire) as usize * 500;
                        let refresh_interval =
                            std::time::Duration::from_millis(refresh_interval as u64);

                        let elapsed = start_load_readings.elapsed();
                        if elapsed > refresh_interval {
                            g_warning!(
                                "MissionCenter::SysInfo",
                                "Refresh took {}ms, which is longer than the refresh interval of {}ms",
                                elapsed.as_millis(),
                                refresh_interval.as_millis()
                            );
                            continue;
                        }

                        let sleep_duration = refresh_interval - elapsed;
                        let sleep_duration_fraction = sleep_duration / 10;
                        for _ in 0..10 {
                            std::thread::sleep(sleep_duration_fraction);
                            if !run.load(Ordering::Acquire) {
                                break 'read_loop;
                            }
                        }
                    }
                })),
                refresh_thread_running,
            },
            readings,
        )
    }

    pub fn set_update_speed(&self, speed: UpdateSpeed) {
        self.refresh_interval
            .store(speed as u8, std::sync::atomic::Ordering::Release);
    }
}
