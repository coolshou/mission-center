use std::error::Error;

use dbus::blocking::Connection;
use dbus_crossroads::Crossroads;

use crate::platform::ProcessesExt;
#[allow(unused_imports)]
use logging::{critical, debug, error, info, message, warning};
#[allow(unused_imports)]
use utils::arraystring::ToArrayStringLossy;

// mod dbus;
mod logging;
mod platform;
mod utils;

struct SystemStatistics {
    processes: platform::Processes,
    apps: platform::Apps,
}

impl SystemStatistics {
    pub fn new() -> Self {
        Self {
            processes: platform::Processes::new(),
            apps: platform::Apps::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let c = Connection::new_session()?;
    c.request_name("io.missioncenter.MissionCenter", true, true, false)?;

    let mut cr = Crossroads::new();
    let iface_token = cr.register("io.missioncenter.MissionCenter", |builder| {
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

                sys_stats
                    .apps
                    .refresh_cache(sys_stats.processes.process_list());
                ctx.reply(Ok((&sys_stats.apps,)));

                // Make the scaffolding happy, since the reply was already set
                Ok((platform::Apps::new(),))
            },
        );
    });

    cr.insert(
        "/io/missioncenter/MissionCenter",
        &[iface_token],
        SystemStatistics::new(),
    );

    cr.serve(&c)?;
    unreachable!()
}
