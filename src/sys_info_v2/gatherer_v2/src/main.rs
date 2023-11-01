use std::error::Error;

use dbus::blocking::Connection;
use dbus_crossroads::Crossroads;

use crate::platform::GpuInfoExt;
#[allow(unused_imports)]
use logging::{critical, debug, error, info, message, warning};
use platform::CpuInfoExt;

// mod dbus;
mod logging;
mod platform;
mod utils;

struct SystemStatistics {
    cpu_info: platform::CpuInfo,
    gpu_info: platform::GpuInfo,
    processes: platform::Processes,
    apps: platform::Apps,
}

impl SystemStatistics {
    pub fn new() -> Self {
        Self {
            cpu_info: platform::CpuInfo::new(),
            gpu_info: platform::GpuInfo::new(),
            processes: platform::Processes::new(),
            apps: platform::Apps::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let c = Connection::new_session()?;
    c.request_name("io.missioncenter.MissionCenter.Gatherer", true, true, false)?;

    let mut cr = Crossroads::new();
    let iface_token = cr.register("io.missioncenter.MissionCenter.Gatherer", |builder| {
        builder.method(
            "GetCpuStaticInfo",
            (),
            ("static_info",),
            |ctx, sys_stats: &mut SystemStatistics, (): ()| {
                ctx.reply(Ok((sys_stats.cpu_info.static_info(),)));

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::CpuStaticInfo::new(),))
            },
        );

        builder.method(
            "GetCpuDynamicInfo",
            (),
            ("static_info",),
            |ctx, sys_stats: &mut SystemStatistics, (): ()| {
                sys_stats
                    .cpu_info
                    .refresh_dynamic_info_cache(&sys_stats.processes);
                ctx.reply(Ok((sys_stats.cpu_info.dynamic_info(),)));

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::CpuDynamicInfo::new(),))
            },
        );

        builder.method(
            "EnumerateGPUs",
            (),
            ("gpu_ids",),
            |ctx, sys_stats: &mut SystemStatistics, (): ()| {
                sys_stats.gpu_info.refresh_gpu_list();
                ctx.reply(Ok((sys_stats.gpu_info.enumerate().collect::<Vec<_>>(),)));

                // Make the scaffolding happy, since the reply was already set
                Ok((Vec::<&str>::new(),))
            },
        );

        builder.method(
            "GetProcesses",
            (),
            ("processes",),
            |ctx, sys_stats: &mut SystemStatistics, (): ()| {
                use platform::ProcessesExt;

                sys_stats.processes.refresh_cache();
                ctx.reply(Ok((&sys_stats.processes,)));

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::Processes::new(),))
            },
        );

        builder.method(
            "GetApps",
            (),
            ("apps",),
            |ctx, sys_stats: &mut SystemStatistics, (): ()| {
                use platform::{AppsExt, ProcessesExt};

                if sys_stats.processes.is_cache_stale() {
                    sys_stats.processes.refresh_cache();
                }

                sys_stats
                    .apps
                    .refresh_cache(sys_stats.processes.process_list());
                ctx.reply(Ok((&sys_stats.apps,)));

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::Apps::new(),))
            },
        );
    });

    let peer_itf = cr.register("org.freedesktop.DBus.Peer", |builder| {
        builder.method("GetMachineId", (), ("machine_uuid",), |_, _, (): ()| {
            Ok((std::fs::read_to_string("/var/lib/dbus/machine-id")
                .map_or("UNKNOWN".into(), |s| s.trim().to_owned()),))
        });
        builder.method("Ping", (), (), |_, _, (): ()| Ok(()));
    });

    let mut sys_stats = SystemStatistics::new();
    sys_stats.cpu_info.refresh_static_info_cache();

    cr.insert(
        "/io/missioncenter/MissionCenter/Gatherer",
        &[peer_itf, iface_token],
        sys_stats,
    );

    cr.serve(&c)?;
    unreachable!()
}
