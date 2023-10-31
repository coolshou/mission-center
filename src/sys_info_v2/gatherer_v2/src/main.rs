use std::error::Error;

use dbus::blocking::Connection;
use dbus_crossroads::Crossroads;

#[allow(unused_imports)]
use logging::{critical, debug, error, info, message, warning};

// mod dbus;
mod logging;
mod platform;
mod utils;

struct SystemStatistics {
    cpu_info: platform::CpuInfo,
    processes: platform::Processes,
    apps: platform::Apps,
}

impl SystemStatistics {
    pub fn new() -> Self {
        Self {
            cpu_info: platform::CpuInfo::new(),
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
                use platform::{CpuInfoExt, CpuStaticInfoExt};

                sys_stats.cpu_info.refresh_static_info_cache();
                ctx.reply(Ok((sys_stats.cpu_info.static_info(),)));

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::CpuStaticInfo::new(),))
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

    cr.insert(
        "/io/missioncenter/MissionCenter/Gatherer",
        &[peer_itf, iface_token],
        SystemStatistics::new(),
    );

    cr.serve(&c)?;
    unreachable!()
}
