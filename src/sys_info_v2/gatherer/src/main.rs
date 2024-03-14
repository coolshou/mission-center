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

use dbus::{arg, blocking::Connection};
use dbus_crossroads::Crossroads;

#[allow(unused_imports)]
use logging::{critical, debug, error, info, message, warning};
use platform::{CpuInfoExt, DisksInfoExt, PlatformUtilitiesExt};

mod logging;
mod platform;
mod utils;

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

struct System {
    cpu_info: platform::CpuInfo,
    disk_info: platform::DisksInfo,
    gpu_info: platform::GpuInfo,
    processes: platform::Processes,
    apps: platform::Apps,
}

impl System {
    pub fn new() -> Self {
        Self {
            cpu_info: platform::CpuInfo::new(),
            disk_info: platform::DisksInfo::new(),
            gpu_info: platform::GpuInfo::new(),
            processes: platform::Processes::new(),
            apps: platform::Apps::new(),
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

    message!("Gatherer::Main", "Starting...");

    message!("Gatherer::Main", "Initializing platform utilities...");
    let plat_utils = platform::PlatformUtilities::default();

    message!("Gatherer::Main", "Setting up connection to main app...");
    // Set up so that the Gatherer exists when the main app exits
    plat_utils.on_main_app_exit(Box::new(|| {
        message!("Gatherer::Main", "Parent process exited, exiting...");
        std::process::exit(0);
    }));

    message!("Gatherer::Main", "Setting up D-Bus connection...");
    let c = Connection::new_session()?;

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
        |h: OrgFreedesktopDBusNameLost, _: &Connection, _: &dbus::Message| {
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
        message!("Gatherer::Main", "Registering D-Bus methods...");

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetCpuStaticInfo`..."
        );
        builder.method(
            "GetCpuStaticInfo",
            (),
            ("static_info",),
            |ctx, sys_stats: &mut System, (): ()| {
                ctx.reply(Ok((sys_stats.cpu_info.static_info(),)));

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::CpuStaticInfo::default(),))
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetCpuDynamicInfo`..."
        );
        builder.method(
            "GetCpuDynamicInfo",
            (),
            ("static_info",),
            |ctx, sys_stats: &mut System, (): ()| {
                sys_stats
                    .cpu_info
                    .refresh_dynamic_info_cache(&sys_stats.processes);
                ctx.reply(Ok((sys_stats.cpu_info.dynamic_info(),)));

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::CpuDynamicInfo::default(),))
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetDisksInfo`..."
        );
        builder.method(
            "GetDisksInfo",
            (),
            ("info",),
            |ctx, sys_stats: &mut System, (): ()| {
                sys_stats.disk_info.refresh_cache();
                ctx.reply(Ok((sys_stats.disk_info.info(),)));

                // Make the scaffolding happy, since the reply was already set
                Ok((Vec::<platform::DiskInfo>::new(),))
            },
        );

        message!("Gatherer::Main", "Registering D-Bus method `GetGPUs`...");
        builder.method(
            "EnumerateGPUs",
            (),
            ("gpu_ids",),
            |ctx, sys_stats: &mut System, (): ()| {
                use platform::GpuInfoExt;

                sys_stats.gpu_info.refresh_gpu_list();
                ctx.reply(Ok((sys_stats.gpu_info.enumerate().collect::<Vec<_>>(),)));

                // Make the scaffolding happy, since the reply was already set
                Ok((Vec::<&str>::new(),))
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetGPUStaticInfo`..."
        );
        builder.method(
            "GetGPUStaticInfo",
            ("gpu_id",),
            ("static_info",),
            |ctx, sys_stats: &mut System, (gpu_id,): (String,)| {
                use platform::GpuInfoExt;

                sys_stats.gpu_info.refresh_static_info_cache();

                match sys_stats.gpu_info.static_info(&gpu_id) {
                    None => {
                        ctx.reply::<(platform::GpuStaticInfo,)>(Err(dbus::MethodErr::invalid_arg(
                            &format!("`{}` is not a valid GPU id", gpu_id),
                        )));
                    }
                    Some(static_info) => {
                        ctx.reply(Ok((static_info,)));
                    }
                }

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::GpuStaticInfo::default(),))
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetGPUDynamicInfo`..."
        );
        builder.method(
            "GetGPUDynamicInfo",
            ("gpu_id",),
            ("dynamic_info",),
            |ctx, sys_stats: &mut System, (gpu_id,): (String,)| {
                use platform::GpuInfoExt;

                sys_stats
                    .gpu_info
                    .refresh_dynamic_info_cache(&mut sys_stats.processes);

                match sys_stats.gpu_info.dynamic_info(&gpu_id) {
                    None => {
                        ctx.reply::<(platform::GpuDynamicInfo,)>(Err(
                            dbus::MethodErr::invalid_arg(&format!(
                                "`{}` is not a valid GPU id",
                                gpu_id
                            )),
                        ));
                    }
                    Some(dynamic_info) => {
                        ctx.reply(Ok((dynamic_info,)));
                    }
                }

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::GpuDynamicInfo::default(),))
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetProcesses`..."
        );
        builder.method(
            "GetProcesses",
            (),
            ("processes",),
            |ctx, sys_stats: &mut System, (): ()| {
                use platform::ProcessesExt;

                sys_stats.processes.refresh_cache();
                ctx.reply(Ok((&sys_stats.processes,)));

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::Processes::default(),))
            },
        );

        message!("Gatherer::Main", "Registering D-Bus method `GetApps`...");
        builder.method(
            "GetApps",
            (),
            ("apps",),
            |ctx, sys_stats: &mut System, (): ()| {
                use platform::{AppsExt, ProcessesExt};

                if sys_stats.processes.is_cache_stale() {
                    sys_stats.processes.refresh_cache();
                }

                sys_stats
                    .apps
                    .refresh_cache(sys_stats.processes.process_list());
                ctx.reply(Ok((&sys_stats.apps,)));

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::Apps::default(),))
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
            |_, sys_stats: &mut System, (pid,): (u32,)| {
                use platform::ProcessesExt;

                sys_stats.processes.terminate_process(pid);

                Ok(())
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
            |_, system: &mut System, (pid,): (u32,)| {
                use platform::ProcessesExt;

                system.processes.kill_process(pid);

                Ok(())
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

    message!("Gatherer::Main", "Instantiating System...");
    let mut system = System::new();

    message!("Gatherer::Main", "Refreshing CPU static info cache...");
    system.cpu_info.refresh_static_info_cache();

    message!("Gatherer::Main", "Inserting system into Crossroads...");
    cr.insert(
        "/io/missioncenter/MissionCenter/Gatherer",
        &[peer_itf, iface_token],
        system,
    );

    message!("Gatherer::Main", "Serving D-Bus requests...");
    cr.serve(&c)?;

    unreachable!()
}
