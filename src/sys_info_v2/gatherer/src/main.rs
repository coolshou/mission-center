/* sys_info_v2/gatherer/src/main.rs
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
use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    atomic::{self, AtomicBool, AtomicU64},
    Arc, Mutex, PoisonError, RwLock,
};

use crate::platform::{FanInfo, FansInfo, FansInfoExt};
use dbus::arg::{Append, Arg, ArgType, IterAppend, RefArg};
use dbus::{arg, blocking::SyncConnection, channel::MatchingReceiver, Signature};
use dbus_crossroads::Crossroads;
use glob::glob;
use lazy_static::lazy_static;
use logging::{critical, debug, error, message, warning};
use platform::{
    Apps, AppsExt, CpuDynamicInfo, CpuInfo, CpuInfoExt, CpuStaticInfo, CpuStaticInfoExt, DiskInfo,
    DisksInfo, DisksInfoExt, GpuDynamicInfo, GpuInfo, GpuInfoExt, GpuStaticInfo,
    PlatformUtilitiesExt, Processes, ProcessesExt, Service, ServiceController,
    ServiceControllerExt, Services, ServicesError, ServicesExt,
};
use pollster::FutureExt;
use udisks2::zbus::zvariant::Value;

#[allow(unused_imports)]
mod logging;
mod platform;
mod utils;

const DBUS_OBJECT_PATH: &str = "/io/missioncenter/MissionCenter/Gatherer";

lazy_static! {
    static ref SYSTEM_STATE: SystemState<'static> = {
        let system_state = SystemState::new();

        let service_controller = system_state
            .services
            .read()
            .unwrap()
            .controller()
            .map(|sc| Some(sc))
            .unwrap_or_else(|e| {
                error!(
                    "Gatherer::Main",
                    "Failed to create service controller: {}", e
                );
                None
            });

        *system_state.service_controller.write().unwrap() = service_controller;

        system_state
            .cpu_info
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_static_info_cache();

        system_state.gpu_info.write().unwrap().refresh_gpu_list();
        system_state
            .gpu_info
            .write()
            .unwrap()
            .refresh_static_info_cache();

        system_state.snapshot();

        system_state
    };
    static ref LOGICAL_CPU_COUNT: u32 = {
        SYSTEM_STATE
            .cpu_info
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .static_info()
            .logical_cpu_count()
    };
}

#[derive(Debug)]
pub struct OrgFreedesktopDBusNameLost {
    pub arg0: String,
}

impl arg::AppendAll for OrgFreedesktopDBusNameLost {
    fn append(&self, i: &mut arg::IterAppend) {
        arg::RefArg::append(&self.arg0, i);
    }
}

impl arg::ReadAll for OrgFreedesktopDBusNameLost {
    fn read(i: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
        Ok(OrgFreedesktopDBusNameLost { arg0: i.read()? })
    }
}

impl dbus::message::SignalArgs for OrgFreedesktopDBusNameLost {
    const NAME: &'static str = "NameLost";
    const INTERFACE: &'static str = "org.freedesktop.DBus";
}

struct SystemState<'a> {
    cpu_info: Arc<RwLock<CpuInfo>>,
    disk_info: Arc<RwLock<DisksInfo>>,
    gpu_info: Arc<RwLock<GpuInfo>>,
    fan_info: Arc<RwLock<FansInfo>>,
    services: Arc<RwLock<Services<'a>>>,
    service_controller: Arc<RwLock<Option<ServiceController<'a>>>>,
    processes: Arc<RwLock<Processes>>,
    apps: Arc<RwLock<Apps>>,

    refresh_interval: Arc<AtomicU64>,
    core_count_affects_percentages: Arc<AtomicBool>,
}

impl SystemState<'_> {
    pub fn snapshot(&self) {
        {
            let mut processes = self
                .processes
                .write()
                .unwrap_or_else(PoisonError::into_inner);

            let timer = std::time::Instant::now();
            processes.refresh_cache();
            if !self
                .core_count_affects_percentages
                .load(atomic::Ordering::Relaxed)
            {
                let logical_cpu_count = *LOGICAL_CPU_COUNT as f32;
                for (_, p) in processes.process_list_mut() {
                    p.usage_stats.cpu_usage /= logical_cpu_count;
                }
            }
            debug!(
                "Gatherer::Perf",
                "Refreshed process cache in {:?}",
                timer.elapsed()
            );
        }

        let timer = std::time::Instant::now();
        self.cpu_info
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_dynamic_info_cache(
                &self
                    .processes
                    .read()
                    .unwrap_or_else(PoisonError::into_inner),
            );
        debug!(
            "Gatherer::Perf",
            "Refreshed CPU dynamic info cache in {:?}",
            timer.elapsed()
        );

        let timer = std::time::Instant::now();
        self.disk_info
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_cache();
        debug!(
            "Gatherer::Perf",
            "Refreshed disk info cache in {:?}",
            timer.elapsed()
        );

        let timer = std::time::Instant::now();
        self.gpu_info
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_dynamic_info_cache(
                &mut self
                    .processes
                    .write()
                    .unwrap_or_else(PoisonError::into_inner),
            );
        debug!(
            "Gatherer::Perf",
            "Refreshed GPU dynamic info cache in {:?}",
            timer.elapsed()
        );

        let timer = std::time::Instant::now();
        self.fan_info
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_cache();
        debug!(
            "Gatherer::Perf",
            "Refreshed fan info cache in {:?}",
            timer.elapsed()
        );

        let timer = std::time::Instant::now();
        self.apps
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_cache(
                self.processes
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .process_list(),
            );
        debug!(
            "Gatherer::Perf",
            "Refreshed app cache in {:?}",
            timer.elapsed()
        );

        let timer = std::time::Instant::now();
        self.services
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_cache()
            .unwrap_or_else(|e| {
                debug!("Gatherer::Main", "Failed to refresh service cache: {}", e);
            });
        debug!(
            "Gatherer::Perf",
            "Refreshed service cache in {:?}",
            timer.elapsed()
        );
    }
}

impl<'a> SystemState<'a> {
    pub fn new() -> Self {
        Self {
            cpu_info: Arc::new(RwLock::new(CpuInfo::new())),
            disk_info: Arc::new(RwLock::new(DisksInfo::new())),
            gpu_info: Arc::new(RwLock::new(GpuInfo::new())),
            fan_info: Arc::new(RwLock::new(FansInfo::new())),
            services: Arc::new(RwLock::new(Services::new())),
            service_controller: Arc::new(RwLock::new(None)),
            processes: Arc::new(RwLock::new(Processes::new())),
            apps: Arc::new(RwLock::new(Apps::new())),

            refresh_interval: Arc::new(AtomicU64::new(1000)),
            core_count_affects_percentages: Arc::new(AtomicBool::new(true)),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Exit if any arguments are passed to this executable. This is done since the main app needs
    // to check if the executable can be run in its current environment (glibc or musl libc)
    for (i, _) in std::env::args().enumerate() {
        if i > 0 {
            eprintln!("👋");
            std::process::exit(0);
        }
    }

    #[cfg(target_os = "linux")]
    unsafe {
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
    }

    message!(
        "Gatherer::Main",
        "Starting v{}...",
        env!("CARGO_PKG_VERSION")
    );

    message!("Gatherer::Main", "Initializing system state...");
    let _ = &*SYSTEM_STATE;
    let _ = &*LOGICAL_CPU_COUNT;

    message!(
        "Gatherer::Main",
        "Setting up background data refresh thread..."
    );
    std::thread::spawn({
        move || loop {
            let refresh_interval = SYSTEM_STATE
                .refresh_interval
                .load(atomic::Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_millis(refresh_interval));

            SYSTEM_STATE.snapshot();
        }
    });

    message!("Gatherer::Main", "Initializing platform utilities...");
    let plat_utils = platform::PlatformUtilities::default();

    message!("Gatherer::Main", "Setting up connection to main app...");
    // Set up so that the Gatherer exists when the main app exits
    plat_utils.on_main_app_exit(Box::new(|| {
        message!("Gatherer::Main", "Parent process exited, exiting...");
        std::process::exit(0);
    }));

    message!("Gatherer::Main", "Setting up D-Bus connection...");
    let c = Arc::new(SyncConnection::new_session()?);

    message!("Gatherer::Main", "Requesting bus name...");
    c.request_name("io.missioncenter.MissionCenter.Gatherer", true, true, true)?;
    message!("Gatherer::Main", "Bus name acquired");

    message!("Gatherer::Main", "Setting up D-Bus proxy...");
    let proxy = c.with_proxy(
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        std::time::Duration::from_millis(5000),
    );

    message!("Gatherer::Main", "Setting up D-Bus signal match...");
    let _id = proxy.match_signal(
        |h: OrgFreedesktopDBusNameLost, _: &SyncConnection, _: &dbus::Message| {
            if h.arg0 != "io.missioncenter.MissionCenter.Gatherer" {
                return true;
            }
            message!("Gatherer::Main", "Bus name {} lost, exiting...", &h.arg0);
            std::process::exit(0);
        },
    )?;

    message!("Gatherer::Main", "Setting up D-Bus crossroads...");
    let mut cr = Crossroads::new();
    let iface_token = cr.register("io.missioncenter.MissionCenter.Gatherer", |builder| {
        message!(
            "Gatherer::Main",
            "Registering D-Bus properties and methods..."
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus property `RefreshInterval`..."
        );
        builder
            .property("RefreshInterval")
            .get_with_cr(|_, _| {
                Ok(SYSTEM_STATE
                    .refresh_interval
                    .load(atomic::Ordering::Relaxed))
            })
            .set_with_cr(|_, _, value| {
                if let Some(value) = value.as_u64() {
                    SYSTEM_STATE
                        .refresh_interval
                        .store(value, atomic::Ordering::Relaxed);
                    Ok(Some(value))
                } else {
                    Err(dbus::MethodErr::failed(&"Invalid value"))
                }
            });

        builder
            .property("CoreCountAffectsPercentages")
            .get_with_cr(|_, _| {
                Ok(SYSTEM_STATE
                    .core_count_affects_percentages
                    .load(atomic::Ordering::Relaxed))
            })
            .set_with_cr(|_, _, value| {
                if let Some(value) = value.as_u64() {
                    let value = value != 0;
                    SYSTEM_STATE
                        .core_count_affects_percentages
                        .store(value, atomic::Ordering::Relaxed);
                    Ok(Some(value))
                } else {
                    Err(dbus::MethodErr::failed(&"Invalid value"))
                }
            });

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetCPUStaticInfo`..."
        );
        builder.method_with_cr_custom::<(), (CpuStaticInfo,), &str, _>(
            "GetCPUStaticInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                                  .cpu_info
                                  .read()
                                  .unwrap_or_else(PoisonError::into_inner)
                                  .static_info(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetCPUDynamicInfo`..."
        );
        builder.method_with_cr_custom::<(), (CpuDynamicInfo,), &str, _>(
            "GetCPUDynamicInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                                  .cpu_info
                                  .read()
                                  .unwrap_or_else(PoisonError::into_inner)
                                  .dynamic_info(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetDisksInfo`..."
        );
        builder.method_with_cr_custom::<(), (Vec<DiskInfo>,), &str, _>(
            "GetDisksInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                                  .disk_info
                                  .read()
                                  .unwrap_or_else(PoisonError::into_inner)
                                  .info()
                                  .collect::<Vec<_>>(),)));

                Some(ctx)
            },
        );

        // using `force` doesn't really do what we want. yes, the unmount happens but leases on the
        // files that made us use force aren't killed. We just get zombies so lets instead just
        // murder processes by hand to naturally allow unmounting + ejecting
        message!("Gatherer::Main", "Registering D-Bus method `EjectDisk`...");
        builder.method_with_cr_custom::<(String, bool, u32), (EjectResult,), &str, _>(
            "EjectDisk",
            ("eject_disk", "kill_all", "kill_pid"),
            ("eject_result",),
            move |mut ctx, _, (id, killall, kill_pid): (String, bool, u32)| {
                let mut rezult = EjectResult::default();

                debug!(
                    "Gatherer::EjectDisk",
                    "Ejecting {}, killing {}/{}",
                    id,
                    kill_pid,
                    killall
                );

                let Ok(client) = &SYSTEM_STATE
                    .disk_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .client
                else {
                    critical!("Gatherer::EjectDisk", "Failed to get dbus client", );
                    ctx.reply(Ok((rezult,)));
                    return Some(ctx);
                };

                let object =
                    match client.object(format!("/org/freedesktop/UDisks2/block_devices/{}", id)) {
                        Ok(object) => object,
                        Err(e) => {
                            critical!("Gatherer::EjectDisk", "Failed to find block object {}: {}", id, e);
                            ctx.reply(Ok((rezult,)));
                            return Some(ctx);
                        }
                    };

                let block = match object.block().block_on() {
                    Ok(block) => block,
                    Err(e) => {
                        critical!("Gatherer::EjectDisk", "Failed to find block {}: {}", id, e);
                        ctx.reply(Ok((rezult,)));
                        return Some(ctx);
                    }
                };

                let drive = match client.drive_for_block(&block).block_on() {
                    Ok(drive) => drive,
                    Err(e) => {
                        critical!("Gatherer::EjectDisk", "Failed to find drive {}: {}", id, e);
                        ctx.reply(Ok((rezult,)));
                        return Some(ctx);
                    }
                };

                let entries = match glob(format!("/sys/block/{}/{}*", id, id).as_str()) {
                    Ok(e) => e,
                    Err(e) => {
                        critical!(
                            "Gatherer::EjectDisk",
                            "Failed to read filesystem information for '{}': {}",
                            id,
                            e
                        );

                        ctx.reply(Ok((rezult,)));
                        return Some(ctx);
                    }
                }
                    .filter_map(Result::ok);

                let mut probable_entries = vec!(id);

                for entry in entries {
                    probable_entries.push(entry.file_name().unwrap().to_str().unwrap().to_string());
                }

                let mut some_path = false;
                let mut some_err = false;

                for filename in probable_entries {
                    // let Some(filename) = entry.file_name() else {
                    //     continue;
                    // };

                    some_path = true;

                    // let filename = filename.to_str().unwrap();

                    let fsobject = match client.object(format!(
                        "/org/freedesktop/UDisks2/block_devices/{}",
                        filename
                    )) {
                        Ok(object) => object,
                        Err(e) => {
                            critical!("Gatherer::EjectDisk", "Failed to find drive block {}: {}", filename, e);
                            continue;
                        }
                    };

                    let Ok(fs) = fsobject.filesystem().block_on() else {
                        continue;
                    };

                    let mut options = HashMap::new();
                    options.insert("auth.no_user_interaction", Value::from(false));

                    let mountpoints = fs.mount_points().block_on().unwrap_or(Vec::new());

                    if !mountpoints.is_empty() {
                        let last_err = some_err;
                        match fs.unmount(options.clone()).block_on() {
                            Ok(_) => {}
                            Err(e) => {
                                some_err = true;
                                critical!("Gatherer::EjectDisk", "Failed to unmount filesystem {}: {}", filename, e);
                                let points = mountpoints
                                    .iter()
                                    .map(|c| {
                                        Path::new(
                                            std::str::from_utf8(c)
                                                .unwrap_or("")
                                                .trim_matches(char::from(0)),
                                        )
                                    })
                                    .collect::<std::collections::HashSet<_>>();
                                let process = SYSTEM_STATE
                                    .processes
                                    .read()
                                    .unwrap_or_else(PoisonError::into_inner);
                                let processes = process.process_cache.clone();

                                let blocks = &mut rezult.blocking_processes;

                                for (pid, _) in processes.iter() {
                                    let mut cwds = Vec::new();
                                    let mut paths = Vec::new();
                                    if let Ok(cwd) = std::fs::read_link(format!("/proc/{pid}/cwd"))
                                    {
                                        let cwdd = cwd.as_path();
                                        if points.iter().any(|p| cwdd.starts_with(p)) {
                                            cwds.push(cwdd.display().to_string());
                                        }
                                    }

                                    if let Ok(readdir) =
                                        std::fs::read_dir(format!("/proc/{pid}/fd"))
                                    {
                                        for fd in readdir.filter_map(Result::ok) {
                                            let real_path = match fd.path().read_link() {
                                                Ok(path) => path,
                                                Err(_) => continue,
                                            };

                                            if points.iter().any(|p| real_path.starts_with(p)) {
                                                // todo kill this process and then retry if force
                                                paths.push(real_path.display().to_string());
                                            }
                                        }
                                    }

                                    if paths.len() > 0 || cwds.len() > 0 {
                                        // if we are not killing add to list
                                        if !killall && (kill_pid != *pid) {
                                            blocks.push((*pid, cwds, paths));
                                        } else {
                                            process.kill_process(*pid);
                                        }
                                    }
                                }
                            }
                        }

                        // lets try again, juuuust in case it works
                        match fs.unmount(options).block_on() {
                            Ok(_) => {
                                some_err = last_err;
                            }
                            Err(e) => {}
                        }
                    } else {
                        debug!(
                            "Gatherer::EjectDisk",
                            "{:?} does not have any mountpoints",
                            filename
                        )
                    }
                }

                // for optic
                if !some_path {
                    match object.filesystem().block_on() {
                        Ok(fs) => {
                            let mut options = HashMap::new();
                            options.insert("auth.no_user_interaction", Value::from(false));

                            match fs.unmount(options).block_on() {
                                Ok(_) => {}
                                Err(e) => {
                                    some_err = true;
                                    critical!("Gatherer::EjectDisk", "Failed to eject solo partition: {}", e);
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }

                if !some_err {
                    let mut options = HashMap::new();
                    options.insert("auth.no_user_interaction", Value::from(false));
                    let result = drive.eject(options).block_on();

                    match result {
                        Ok(_) => rezult.success = true,
                        Err(e) => {
                            critical!("Gatherer::EjectDisk", "Failed to eject disk {}", e)
                        }
                    }
                }

                ctx.reply(Ok((rezult,)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `SataSmartInfo`..."
        );
        builder.method_with_cr_custom::<(String,), (SataSmartResult,), &str, _>(
            "SataSmartInfo",
            ("smart_disk",),
            ("smart_info",),
            move |mut ctx, _, (id, ): (String,)| {
                let mut rezult = SataSmartResult::default();

                debug!(
                    "Gatherer::SataSmartInfo",
                    "Getting Smart for {}",
                    id,
                );

                let Ok(client) = &SYSTEM_STATE
                    .disk_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .client
                else {
                    critical!("Gatherer::SataSmartInfo", "Failed to get dbus client", );
                    ctx.reply(Ok((rezult,)));
                    return Some(ctx);
                };

                let object =
                    match client.object(format!("/org/freedesktop/UDisks2/block_devices/{}", id)) {
                        Ok(object) => object,
                        Err(e) => {
                            critical!("Gatherer::SataSmartInfo", "Failed to find block object {}: {}", id, e);
                            ctx.reply(Ok((rezult,)));
                            return Some(ctx);
                        }
                    };

                let block = match object.block().block_on() {
                    Ok(block) => block,
                    Err(e) => {
                        critical!("Gatherer::SataSmartInfo", "Failed to find block {}: {}", id, e);
                        ctx.reply(Ok((rezult,)));
                        return Some(ctx);
                    }
                };

                let drive_path = match block.drive().block_on() {
                    Ok(drive) => drive,
                    Err(e) => {
                        critical!("Gatherer::SataSmartInfo", "Failed to find drive for {}: {}", id, e);
                        ctx.reply(Ok((rezult,)));
                        return Some(ctx);
                    }
                };

                let drive_object = match client.object(drive_path) {
                    Ok(drive_object) => drive_object,
                    Err(e) => {
                        critical!("Gatherer::SataSmartInfo", "Failed to find drive object {}: {}", id, e);
                        ctx.reply(Ok((rezult,)));
                        return Some(ctx);
                    }
                };

                match drive_object.drive_ata().block_on() {
                    Ok(ata) => {
                        rezult.common_data.powered_on_seconds =
                            ata.smart_power_on_seconds().block_on().unwrap();
                        rezult.common_data.last_update_time = ata.smart_updated().block_on().unwrap();
                        rezult.common_data.test_result =
                            SmartTestResult::from(ata.smart_selftest_status().block_on().unwrap());

                        let options = HashMap::new();

                        let attributes = match ata.smart_get_attributes(options).block_on() {
                            Ok(res) => res,
                            Err(e) => {
                                critical!("Gatherer::SataSmartInfo", "Failed to find ata interface {}: {}", id, e);
                                ctx.reply(Ok((rezult,)));
                                return Some(ctx);
                            }
                        };

                        for entry in attributes {
                            rezult.data.push(SataSmartEntry {
                                id: entry.0,
                                name: entry.1,
                                flags: entry.2,
                                value: entry.3,
                                worst: entry.4,
                                threshold: entry.5,
                                pretty: entry.6,
                                pretty_unit: entry.7,
                            });
                        }

                        rezult.common_data.success = true;
                    }
                    Err(e) => {
                        critical!("Gatherer::SataSmartInfo", "Failed to find ata interface {}: {}", id, e);
                    }
                }

                ctx.reply(Ok((rezult,)));
                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `NVMeSmartInfo`..."
        );
        builder.method_with_cr_custom::<(String,), (NVMeSmartResult,), &str, _>(
            "NVMeSmartInfo",
            ("smart_disk",),
            ("smart_info",),
            move |mut ctx, _, (id, ): (String,)| {
                let mut rezult = NVMeSmartResult::default();

                debug!(
                    "Gatherer::NVMeSmartInfo",
                    "Getting Smart for {}",
                    id,
                );

                let Ok(client) = &SYSTEM_STATE
                    .disk_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .client
                else {
                    critical!("Gatherer::NVMeSmartInfo", "Failed to get dbus client", );
                    ctx.reply(Ok((rezult,)));
                    return Some(ctx);
                };

                let object =
                    match client.object(format!("/org/freedesktop/UDisks2/block_devices/{}", id)) {
                        Ok(object) => object,
                        Err(e) => {
                            critical!("Gatherer::NVMeSmartInfo", "Failed to find block object {}: {}", id, e);
                            ctx.reply(Ok((rezult,)));
                            return Some(ctx);
                        }
                    };

                let block = match object.block().block_on() {
                    Ok(block) => block,
                    Err(e) => {
                        critical!("Gatherer::NVMeSmartInfo", "Failed to find block {}: {}", id, e);
                        ctx.reply(Ok((rezult,)));
                        return Some(ctx);
                    }
                };

                let drive_path = match block.drive().block_on() {
                    Ok(drive) => drive,
                    Err(e) => {
                        critical!("Gatherer::NVMeSmartInfo", "Failed to find drive for {}: {}", id, e);
                        ctx.reply(Ok((rezult,)));
                        return Some(ctx);
                    }
                };

                let drive_object = match client.object(drive_path) {
                    Ok(drive_object) => drive_object,
                    Err(e) => {
                        critical!("Gatherer::NVMeSmartInfo", "Failed to find drive {}: {}", id, e);
                        ctx.reply(Ok((rezult,)));
                        return Some(ctx);
                    }
                };

                match drive_object.nvme_controller().block_on() {
                    Ok(nvme) => {
                        let options = HashMap::new();

                        let attributes = match nvme.smart_get_attributes(options).block_on() {
                            Ok(res) => res,
                            Err(e) => {
                                critical!("Gatherer::NVMeSmartInfo", "Failed to get attributes for {}: {}", id, e);

                                ctx.reply(Ok((rezult,)));
                                return Some(ctx);
                            }
                        };

                        if let Some(num_err_log_entries) = attributes.get("num_err_log_entries") {
                            rezult.num_err_log_entries =
                                num_err_log_entries.try_into().unwrap_or_default();
                        }

                        if let Some(critical_temp_time) = attributes.get("critical_temp_time") {
                            rezult.critical_temp_time =
                                critical_temp_time.try_into().unwrap_or_default();
                        }

                        if let Some(wctemp) = attributes.get("wctemp") {
                            rezult.wctemp = wctemp.try_into().unwrap_or_default();
                        }

                        if let Some(ctrl_busy_time) = attributes.get("ctrl_busy_time") {
                            rezult.ctrl_busy_minutes = ctrl_busy_time.try_into().unwrap_or_default();
                        }

                        if let Some(media_errors) = attributes.get("media_errors") {
                            rezult.media_errors = media_errors.try_into().unwrap_or_default();
                        }

                        if let Some(warning_temp_time) = attributes.get("warning_temp_time") {
                            rezult.warning_temp_time = warning_temp_time.try_into().unwrap_or_default();
                        }

                        if let Some(avail_spare) = attributes.get("avail_spare") {
                            rezult.avail_spare = avail_spare.try_into().unwrap_or_default();
                        }

                        if let Some(power_cycles) = attributes.get("power_cycles") {
                            rezult.power_cycles = power_cycles.try_into().unwrap_or_default();
                        }

                        if let Some(cctemp) = attributes.get("cctemp") {
                            rezult.cctemp = cctemp.try_into().unwrap_or_default();
                        }

                        if let Some(unsafe_shutdowns) = attributes.get("unsafe_shutdowns") {
                            rezult.unsafe_shutdowns = unsafe_shutdowns.try_into().unwrap_or_default();
                        }

                        if let Some(spare_thresh) = attributes.get("spare_thresh") {
                            rezult.spare_thresh = spare_thresh.try_into().unwrap_or_default();
                        }

                        if let Some(total_data_written) = attributes.get("total_data_written") {
                            rezult.total_data_written =
                                total_data_written.try_into().unwrap_or_default();
                        }

                        // if let Some(temp_sensors) = attributes.get("temp_sensors") {
                        //     rezult.temp_sensors = temp_sensors.try_into().unwrap_or_default();
                        // }

                        if let Some(total_data_read) = attributes.get("total_data_read") {
                            rezult.total_data_read = total_data_read.try_into().unwrap_or_default();
                        }

                        if let Some(percent_used) = attributes.get("percent_used") {
                            rezult.percent_used = percent_used.try_into().unwrap_or_default();
                        }

                        rezult.common_smart_result.success = true;

                        rezult.common_smart_result.powered_on_seconds =
                            nvme.smart_power_on_hours().block_on().unwrap() * 3600;
                        rezult.common_smart_result.test_result =
                            SmartTestResult::from(nvme.smart_selftest_status().block_on().unwrap());
                        rezult.common_smart_result.last_update_time =
                            nvme.smart_updated().block_on().unwrap();
                    }
                    Err(e) => {
                        critical!("Gatherer::NVMeSmartInfo", "Failed to get nvme interface for {}: {}", id, e);
                    }
                }

                ctx.reply(Ok((rezult,)));
                Some(ctx)
            },
        );

        message!("Gatherer::Main", "Registering D-Bus method `GetGPUList`...");
        builder.method_with_cr_custom::<(), (Vec<String>,), &str, _>(
            "GetGPUList",
            (),
            ("gpu_list",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                                  .gpu_info
                                  .read()
                                  .unwrap_or_else(PoisonError::into_inner)
                                  .enumerate()
                                  .map(|id| id.to_owned())
                                  .collect::<Vec<_>>(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetGPUStaticInfo`..."
        );
        builder.method_with_cr_custom::<(), (Vec<GpuStaticInfo>,), &str, _>(
            "GetGPUStaticInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                let gpu_info = SYSTEM_STATE
                    .gpu_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner);
                ctx.reply(Ok((gpu_info
                                  .enumerate()
                                  .map(|id| gpu_info.static_info(id).cloned().unwrap())
                                  .collect::<Vec<_>>(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetGPUDynamicInfo`..."
        );
        builder.method_with_cr_custom::<(), (Vec<GpuDynamicInfo>,), &str, _>(
            "GetGPUDynamicInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                let gpu_info = SYSTEM_STATE
                    .gpu_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner);
                ctx.reply(Ok((gpu_info
                                  .enumerate()
                                  .map(|id| gpu_info.dynamic_info(id).cloned().unwrap())
                                  .collect::<Vec<_>>(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetFansInfo`..."
        );
        builder.method_with_cr_custom::<(), (Vec<FanInfo>,), &str, _>(
            "GetFansInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                                  .fan_info
                                  .read()
                                  .unwrap_or_else(PoisonError::into_inner)
                                  .info()
                                  .collect::<Vec<_>>(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetProcesses`..."
        );
        builder.method_with_cr_custom::<(), (Processes,), &str, _>(
            "GetProcesses",
            (),
            ("process_list",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((&*SYSTEM_STATE
                    .processes
                    .write()
                    .unwrap_or_else(PoisonError::into_inner),)));

                Some(ctx)
            },
        );

        message!("Gatherer::Main", "Registering D-Bus method `GetApps`...");
        builder.method_with_cr_custom::<(), (Apps,), &str, _>(
            "GetApps",
            (),
            ("app_list",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                                  .apps
                                  .read()
                                  .unwrap_or_else(PoisonError::into_inner)
                                  .app_list(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetServices`..."
        );
        builder.method_with_cr_custom::<(), (Vec<Service>,), &str, _>(
            "GetServices",
            (),
            ("service_list",),
            move |mut ctx, _, (): ()| {
                match SYSTEM_STATE
                    .services
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .services()
                {
                    Ok(s) => {
                        ctx.reply(Ok((s,)));
                    }
                    Err(e) => {
                        error!("Gatherer::Main", "Failed to get services: {}", e);
                        ctx.reply::<(Vec<Service>,)>(Ok((vec![],)));
                    }
                }

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `TerminateProcess`..."
        );
        builder.method(
            "TerminateProcess",
            ("process_id",),
            (),
            move |_, _: &mut (), (pid, ): (u32,)| {
                execute_no_reply(
                    SYSTEM_STATE.processes.clone(),
                    move |processes| -> Result<(), u8> { Ok(processes.terminate_process(pid)) },
                    "terminating process",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `KillProcess`..."
        );
        builder.method(
            "KillProcess",
            ("process_id",),
            (),
            move |_, _: &mut (), (pid, ): (u32,)| {
                execute_no_reply(
                    SYSTEM_STATE.processes.clone(),
                    move |processes| -> Result<(), u8> { Ok(processes.kill_process(pid)) },
                    "terminating process",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `EnableService`..."
        );
        builder.method(
            "EnableService",
            ("service_name",),
            (),
            move |_, _: &mut (), (service, ): (String,)| {
                execute_no_reply(
                    SYSTEM_STATE.service_controller.clone(),
                    move |sc| {
                        if let Some(sc) = sc.as_ref() {
                            sc.enable_service(&service)
                        } else {
                            Err(ServicesError::MissingServiceController)
                        }
                    },
                    "enabling service",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `DisableService`..."
        );
        builder.method(
            "DisableService",
            ("service_name",),
            (),
            move |_, _: &mut (), (service, ): (String,)| {
                execute_no_reply(
                    SYSTEM_STATE.service_controller.clone(),
                    move |sc| {
                        if let Some(sc) = sc.as_ref() {
                            sc.disable_service(&service)
                        } else {
                            Err(ServicesError::MissingServiceController)
                        }
                    },
                    "disabling service",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `StartService`..."
        );
        builder.method(
            "StartService",
            ("service_name",),
            (),
            move |_, _: &mut (), (service, ): (String,)| {
                execute_no_reply(
                    SYSTEM_STATE.service_controller.clone(),
                    move |sc| {
                        if let Some(sc) = sc.as_ref() {
                            sc.start_service(&service)
                        } else {
                            Err(ServicesError::MissingServiceController)
                        }
                    },
                    "starting service",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `StopService`..."
        );
        builder.method(
            "StopService",
            ("service_name",),
            (),
            move |_, _: &mut (), (service, ): (String,)| {
                execute_no_reply(
                    SYSTEM_STATE.service_controller.clone(),
                    move |sc| {
                        if let Some(sc) = sc.as_ref() {
                            sc.stop_service(&service)
                        } else {
                            Err(ServicesError::MissingServiceController)
                        }
                    },
                    "stopping service",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `RestartService`..."
        );
        builder.method(
            "RestartService",
            ("service_name",),
            (),
            move |_, _: &mut (), (service, ): (String,)| {
                execute_no_reply(
                    SYSTEM_STATE.service_controller.clone(),
                    move |sc| {
                        if let Some(sc) = sc.as_ref() {
                            sc.restart_service(&service)
                        } else {
                            Err(ServicesError::MissingServiceController)
                        }
                    },
                    "restarting service",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetServiceLogs`..."
        );
        builder.method_with_cr_custom::<(String, u32), (String,), &str, _>(
            "GetServiceLogs",
            ("name", "pid"),
            ("service_list",),
            move |mut ctx, _, (name, pid): (String, u32)| {
                match SYSTEM_STATE
                    .services
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .service_logs(&name, std::num::NonZeroU32::new(pid))
                {
                    Ok(s) => {
                        ctx.reply(Ok((s.as_ref().to_owned(),)));
                    }
                    Err(e) => {
                        ctx.reply(Result::<(Vec<Service>,), dbus::MethodErr>::Err(
                            dbus::MethodErr::failed::<String>(&format!(
                                "Failed to get service logs: {e}"
                            )),
                        ));
                    }
                }

                Some(ctx)
            },
        );
    });

    message!(
        "Gatherer::Main",
        "Registering D-Bus interface `org.freedesktop.DBus.Peer`..."
    );
    let peer_itf = cr.register("org.freedesktop.DBus.Peer", |builder| {
        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetMachineId`..."
        );
        builder.method("GetMachineId", (), ("machine_uuid",), |_, _, (): ()| {
            Ok((std::fs::read_to_string("/var/lib/dbus/machine-id")
                    .map_or("UNKNOWN".into(), |s| s.trim().to_owned()),))
        });

        message!("Gatherer::Main", "Registering D-Bus method `Ping`...");
        builder.method("Ping", (), (), |_, _, (): ()| Ok(()));
    });

    message!(
        "Gatherer::Main",
        "Instantiating System and inserting it into Crossroads..."
    );
    cr.insert(DBUS_OBJECT_PATH, &[peer_itf, iface_token], ());

    message!("Gatherer::Main", "Creating thread pool...");
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build_global()?;

    message!("Gatherer::Main", "Serving D-Bus requests...");

    let cr = Arc::new(Mutex::new(cr));
    c.start_receive(dbus::message::MatchRule::new_method_call(), {
        Box::new(move |msg, conn| {
            cr.lock()
                .unwrap()
                .handle_message(msg, conn)
                .unwrap_or_else(|_| error!("Gatherer::Main", "Failed to handle message"));
            true
        })
    });

    loop {
        c.process(std::time::Duration::from_millis(1000))?;
    }
}

fn execute_no_reply<SF: Send + Sync + 'static, E: std::fmt::Display>(
    stats: Arc<RwLock<SF>>,
    command: impl FnOnce(&SF) -> Result<(), E> + Send + 'static,
    description: &'static str,
) -> Result<(), dbus::MethodErr> {
    rayon::spawn(move || {
        let stats = match stats.read() {
            Ok(s) => s,
            Err(poisoned_lock) => {
                warning!(
                    "Gatherer::Main",
                    "Lock poisoned while executing command for {}",
                    description
                );
                poisoned_lock.into_inner()
            }
        };

        if let Err(e) = command(&stats) {
            error!("Gatherer::Main", "Failed to execute command: {}", e);
        }
    });

    Ok(())
}

pub struct EjectResult {
    success: bool,

    blocking_processes: Vec<(u32, Vec<String>, Vec<String>)>,
}

impl Default for EjectResult {
    fn default() -> Self {
        Self {
            success: false,
            blocking_processes: vec![],
        }
    }
}

impl Append for EjectResult {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append((self.success, self.blocking_processes.clone()));
    }
}

impl Arg for EjectResult {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(ba(ua(s)a(s)))")
    }
}

#[derive(Debug, Clone)]
pub struct SataSmartEntry {
    id: u8,
    name: String,
    flags: u16,
    value: i32,
    worst: i32,
    threshold: i32,
    pretty: i64,
    pretty_unit: i32,
}

impl Default for SataSmartEntry {
    fn default() -> Self {
        Self {
            id: 0,
            name: "".to_string(),
            flags: 0,
            value: 0,
            worst: 0,
            threshold: 0,
            pretty: 0,
            pretty_unit: 0,
        }
    }
}

impl Append for SataSmartEntry {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append((
            self.id,
            self.name.clone(),
            self.flags,
            self.value,
            self.worst,
            self.threshold,
            self.pretty,
            self.pretty_unit,
        ));
    }
}

impl Arg for SataSmartEntry {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(ysqiiixi)")
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SmartTestResult {
    UNKNOWN_RESULT = 0,
    Success,
    Aborted,
    FatalError,
    InProgress,
    // ATA only
    Interrupted,
    ErrorUnknown,
    ErrorElectrical,
    ErrorServo,
    ErrorRead,
    ErrorHandling,
    //NVMe only
    CtrlReset,
    NsRemoved,
    AbortedFormat,
    UnknownSegmentFailed,
    KnownSegmentFailed,
    AbortedUnknown,
    AbortedSanitize,
}

impl From<String> for SmartTestResult {
    fn from(value: String) -> Self {
        match value.as_str() {
            "success" => SmartTestResult::Success,
            "aborted" => SmartTestResult::Aborted,
            "fatal" => SmartTestResult::FatalError,
            "fatal_error" => SmartTestResult::FatalError,
            "inprogress" => SmartTestResult::InProgress,

            "error_unknown" => SmartTestResult::ErrorUnknown,
            "error_electrical" => SmartTestResult::ErrorElectrical,
            "error_servo" => SmartTestResult::ErrorServo,
            "error_read" => SmartTestResult::ErrorRead,
            "error_handling" => SmartTestResult::ErrorHandling,

            "ctrl_reset" => SmartTestResult::CtrlReset,
            "ns_removed" => SmartTestResult::NsRemoved,
            "aborted_format" => SmartTestResult::AbortedFormat,
            "unknown_seg_fail" => SmartTestResult::UnknownSegmentFailed,
            "known_seg_fail" => SmartTestResult::KnownSegmentFailed,
            "aborted_unknown" => SmartTestResult::AbortedUnknown,
            "aborted_sanitize" => SmartTestResult::AbortedSanitize,

            _ => SmartTestResult::UNKNOWN_RESULT,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommonSmartResult {
    pub success: bool,

    pub powered_on_seconds: u64,
    pub last_update_time: u64,
    pub test_result: SmartTestResult,
}

impl Default for CommonSmartResult {
    fn default() -> Self {
        Self {
            success: false,
            powered_on_seconds: 0,
            last_update_time: 0,
            test_result: SmartTestResult::UNKNOWN_RESULT,
        }
    }
}

impl Append for CommonSmartResult {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append_struct(|ia| {
            ia.append(self.success);
            ia.append(self.powered_on_seconds);
            ia.append(self.last_update_time);
            ia.append(self.test_result as u8);
        });
    }
}

#[derive(Debug)]
pub struct SataSmartResult {
    common_data: CommonSmartResult,

    data: Vec<SataSmartEntry>,
}

impl Default for SataSmartResult {
    fn default() -> Self {
        Self {
            common_data: Default::default(),
            data: vec![],
        }
    }
}

impl Append for SataSmartResult {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append((self.common_data.clone(), self.data.clone()));
    }
}

impl Arg for SataSmartResult {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("((bts)a(ysqiiixi))")
    }
}

#[derive(Debug)]
pub struct NVMeSmartResult {
    common_smart_result: CommonSmartResult,

    avail_spare: u8,
    spare_thresh: u8,
    percent_used: u8,
    total_data_read: u64,
    total_data_written: u64,
    ctrl_busy_minutes: u64,
    power_cycles: u64,
    unsafe_shutdowns: u64,
    media_errors: u64,
    num_err_log_entries: u64,
    temp_sensors: Vec<u16>,
    wctemp: u16,
    cctemp: u16,
    warning_temp_time: u32,
    critical_temp_time: u32,
}

impl Default for NVMeSmartResult {
    fn default() -> Self {
        Self {
            common_smart_result: Default::default(),
            avail_spare: 0,
            spare_thresh: 0,
            percent_used: 0,
            total_data_read: 0,
            total_data_written: 0,
            ctrl_busy_minutes: 0,
            power_cycles: 0,
            unsafe_shutdowns: 0,
            media_errors: 0,
            num_err_log_entries: 0,
            temp_sensors: vec![],
            wctemp: 0,
            cctemp: 0,
            warning_temp_time: 0,
            critical_temp_time: 0,
        }
    }
}

impl Append for NVMeSmartResult {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append_struct(|ia| {
            ia.append(self.common_smart_result.clone());
            ia.append(self.avail_spare);
            ia.append(self.spare_thresh);
            ia.append(self.percent_used);
            ia.append(self.total_data_read);
            ia.append(self.total_data_written);
            ia.append(self.ctrl_busy_minutes);
            ia.append(self.power_cycles);
            ia.append(self.unsafe_shutdowns);
            ia.append(self.media_errors);
            ia.append(self.num_err_log_entries);
            ia.append(self.temp_sensors.clone());
            ia.append(self.wctemp);
            ia.append(self.cctemp);
            ia.append(self.warning_temp_time);
            ia.append(self.critical_temp_time);
        });
    }
}

impl Arg for NVMeSmartResult {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("((bts)yyyttttttta(q)qquu)")
    }
}
