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

use lazy_static::lazy_static;

use crate::i18n::*;

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

            std::path::Path::new(cache_dir.as_str())
                .canonicalize()
                .unwrap_or(std::path::PathBuf::from(
                    "/tmp/io.missioncenter.MissionCenter/state",
                ))
                .to_string_lossy()
                .to_string()
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

struct GathererSupervisor {
    gatherer: gatherer::Gatherer<gatherer::SharedData>,
}

impl Drop for GathererSupervisor {
    fn drop(&mut self) {
        match self.gatherer.stop() {
            Ok(_) => {}
            Err(e) => {
                use gtk::glib::g_critical;
                g_critical!("MissionCenter::SysInfo", "Unable to stop gatherer: {}", e);
            }
        };
    }
}

impl GathererSupervisor {
    pub fn new() -> Result<Self, gatherer::GathererError> {
        let mut gatherer = gatherer::Gatherer::<gatherer::SharedData>::new(Self::executable())?;
        // SAFETY: We are the only ones that have access to the shared memory instance
        let mut shm_data = unsafe { gatherer.shared_memory_unchecked() };
        (*shm_data).clear();

        Ok(Self { gatherer })
    }

    pub fn send_message(&mut self, message: gatherer::Message) {
        self.execute(message, |_, _| true);
    }

    #[inline]
    fn execute<F>(&mut self, message: gatherer::Message, mut f: F)
    where
        F: FnMut(
            &mut gatherer::Gatherer<gatherer::SharedData>,
            /* process_restarted */ bool,
        ) -> bool,
    {
        use gtk::glib::g_critical;

        let mut restarted = false;
        let mut error_counter = 0;

        loop {
            if error_counter >= 5 {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Too many errors when trying to send {:?} to the gatherer, giving up",
                    message
                );
                break;
            }

            if let Err(e) = self.gatherer.send_message(message) {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to send message to daemon: {:#?}",
                    e
                );

                restarted = true;

                if let Err(e) = self.gatherer.is_running() {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "Gatherer is no longer running: {:?}. Restarting...",
                        e
                    );

                    match self.gatherer.start() {
                        Ok(_) => {
                            g_critical!(
                                "MissionCenter::SysInfo",
                                "Gatherer restarted successfully"
                            );
                        }
                        Err(e) => {
                            g_critical!(
                                "MissionCenter::SysInfo",
                                "Failed to restart gatherer: {}",
                                e
                            );

                            error_counter += 1;
                        }
                    }
                    continue;
                } else {
                    match self.gatherer.stop() {
                        Ok(_) => {
                            g_critical!(
                                "MissionCenter::SysInfo",
                                "Gatherer hung and was stopped successfully"
                            );
                        }
                        Err(e) => {
                            g_critical!(
                                "MissionCenter::SysInfo",
                                "Gatherer hung and could not be stopped, will try again : {:#?}",
                                e
                            );

                            error_counter += 1;
                        }
                    }
                    continue;
                }
            }

            if f(&mut self.gatherer, restarted) {
                break;
            }

            restarted = false;
        }
    }

    fn executable() -> String {
        use gtk::glib::g_debug;

        let executable_name = if *IS_FLATPAK {
            let flatpak_app_path = FLATPAK_APP_PATH.as_str();

            let cmd_status = cmd_flatpak_host!(&format!(
                "{}/bin/missioncenter-gatherer-glibc",
                flatpak_app_path
            ))
            .status();
            if let Ok(status) = cmd_status {
                if status.success()
                    || status.code() == Some(gatherer::ExitCode::MissingProgramArgument as i32)
                {
                    format!("{}/bin/missioncenter-gatherer-glibc", flatpak_app_path)
                } else {
                    format!("{}/bin/missioncenter-gatherer-musl", flatpak_app_path)
                }
            } else {
                format!("{}/bin/missioncenter-gatherer-musl", flatpak_app_path)
            }
        } else {
            "missioncenter-gatherer".to_string()
        };

        g_debug!(
            "MissionCenter::ProcInfo",
            "Proxy executable name: {}",
            executable_name
        );

        executable_name
    }
}

pub struct SysInfoV2 {
    refresh_interval: std::sync::Arc<std::sync::atomic::AtomicU8>,
    refresh_thread: Option<std::thread::JoinHandle<()>>,
    refresh_thread_running: std::sync::Arc<std::sync::atomic::AtomicBool>,

    sender: std::sync::mpsc::Sender<gatherer::Message>,
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

        let (tx, _) = mpsc::channel::<gatherer::Message>();

        Self {
            refresh_interval: Arc::new(0.into()),
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
        let refresh_thread_running = Arc::new(AtomicBool::new(true));

        let ri = refresh_interval.clone();
        let run = refresh_thread_running.clone();

        let (tx, rx) = mpsc::channel::<gatherer::Message>();
        Self {
            refresh_interval,
            refresh_thread: Some(std::thread::spawn(move || {
                use adw::prelude::*;
                use gtk::glib::*;
                use sysinfo::{System, SystemExt};

                let mut system = System::new();
                let mut readings = Readings::new(&mut system);

                let mut gatherer_supervisor = match GathererSupervisor::new() {
                    Ok(gs) => gs,
                    Err(e) => {
                        idle_add_once(move || {
                            g_critical!(
                                "MissionCenter::SysInfo",
                                "Unable to start gatherer: {:#?}",
                                &e
                            );

                            let app_window = crate::MissionCenterApplication::default_instance()
                                .and_then(|app| app.active_window());

                            let error_dialog = adw::MessageDialog::new(
                                app_window.as_ref(),
                                Some("A fatal error has occurred"),
                                Some(&format!("Unable to start data gatherer process: {:#?}", e)),
                            );
                            error_dialog.set_modal(true);
                            error_dialog.add_responses(&[("close", &i18n("_Quit"))]);
                            error_dialog.set_response_appearance(
                                "close",
                                adw::ResponseAppearance::Destructive,
                            );
                            error_dialog.connect_response(None, |dialog, _| {
                                dialog.close();
                                std::process::exit(1);
                            });
                            error_dialog.present();
                        });

                        return;
                    }
                };

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

                let mut processes = gatherer_supervisor.processes();
                for gpu in &readings.gpus {
                    for (pid, gpu_usage) in &gpu.dynamic_info.processes {
                        if let Some(process) = processes.get_mut(&(*pid as u32)) {
                            process.stats_mut().gpu_usage = *gpu_usage;
                        }
                    }
                }
                readings.process_tree =
                    proc_info::process_hierarchy(&processes).unwrap_or_default();
                readings.running_apps = gatherer_supervisor.apps();

                let cpu_static_info = readings.cpu_info.static_info.clone();

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
                    let start_load_readings = std::time::Instant::now();

                    let timer = std::time::Instant::now();
                    let cpu_info = CpuInfoDynamic::load(&mut system);
                    eprintln!("CPU load took: {:?}", timer.elapsed());

                    let timer = std::time::Instant::now();
                    let mem_info = MemInfo::load().unwrap_or(MemInfo::default());
                    eprintln!("Mem load took: {:?}", timer.elapsed());

                    let timer = std::time::Instant::now();
                    let disks = DiskInfo::load(&mut disk_stats);
                    eprintln!("Disk load took: {:?}", timer.elapsed());

                    let timer = std::time::Instant::now();
                    let network_devices = if let Some(net_info) = net_info.as_mut() {
                        net_info.load_devices()
                    } else {
                        vec![]
                    };
                    eprintln!("Net load took: {:?}", timer.elapsed());

                    let timer = std::time::Instant::now();
                    let gpus = if let Some(gpu_info) = gpu_info.as_mut() {
                        gpu_info.load_gpus()
                    } else {
                        vec![]
                    };
                    eprintln!("GPU load took: {:?}", timer.elapsed());

                    let timer = std::time::Instant::now();
                    let mut processes = gatherer_supervisor.processes();
                    for gpu in &gpus {
                        for (pid, gpu_usage) in &gpu.dynamic_info.processes {
                            if let Some(process) = processes.get_mut(&(*pid as u32)) {
                                process.stats_mut().gpu_usage = *gpu_usage;
                            }
                        }
                    }
                    eprintln!("Process load took: {:?}", timer.elapsed());

                    let timer = std::time::Instant::now();
                    let process_tree = proc_info::process_hierarchy(&processes).unwrap_or_default();
                    eprintln!("Process tree load took: {:?}", timer.elapsed());

                    let timer = std::time::Instant::now();
                    let running_apps = gatherer_supervisor.apps();
                    eprintln!("App load took: {:?}", timer.elapsed());

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

                            let timer = std::time::Instant::now();
                            if !app.refresh_readings(&mut readings) {
                                g_critical!(
                                        "MissionCenter::SysInfo",
                                        "Readings were not completely refreshed, stale readings will be displayed"
                                    );
                            }
                            eprintln!("UI refresh took: {:?}", timer.elapsed());

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

                    let mut sleep_duration = refresh_interval - elapsed;
                    let sleep_duration_fraction = sleep_duration / 10;
                    for _ in 0..10 {
                        let timer = std::time::Instant::now();

                        match rx.recv_timeout(sleep_duration_fraction) {
                            Ok(message) => gatherer_supervisor.send_message(message),
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

    pub fn terminate_process(&self, terminate_type: TerminateType, pid: u32) {
        use gtk::glib::g_critical;

        match terminate_type {
            TerminateType::Normal => {
                match self.sender.send(gatherer::Message::TerminateProcess(pid)) {
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Error sending TerminateProcess({}) to gatherer: {}",
                            pid,
                            e
                        );
                    }
                    _ => {}
                }
            }
            TerminateType::Force => match self.sender.send(gatherer::Message::KillProcess(pid)) {
                Err(e) => {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "Error sending KillProcess({}) to gatherer: {}",
                        pid,
                        e
                    );
                }
                _ => {}
            },
        }
    }
}
