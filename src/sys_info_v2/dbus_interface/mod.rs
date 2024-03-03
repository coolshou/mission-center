/* sys_info_v2/dbus-interface/mod.rs
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

use std::{collections::HashMap, sync::Arc};

use dbus::{arg::*, blocking, blocking::BlockingSender, Error, strings::*};

pub use apps::*;
use arc_str_vec::*;
pub use cpu_dynamic_info::*;
pub use cpu_static_info::*;
pub use gpu_dynamic_info::*;
pub use gpu_static_info::*;
pub use processes::{Process, ProcessUsageStats};
use processes::ProcessMap;

mod apps;
mod arc_str_vec;
mod cpu_dynamic_info;
mod cpu_static_info;
mod gpu_dynamic_info;
mod gpu_static_info;
mod processes;

fn dbus_method_call<
    'a,
    'i,
    'm,
    R: ReadAll,
    A: AppendAll,
    I: Into<Interface<'i>>,
    M: Into<Member<'m>>,
>(
    connection: &blocking::Connection,
    destination: &BusName<'a>,
    path: &Path<'a>,
    timeout: std::time::Duration,
    i: I,
    m: M,
    args: A,
) -> Result<R, dbus::Error> {
    let mut msg = dbus::Message::method_call(destination, path, &i.into(), &m.into());
    args.append(&mut IterAppend::new(&mut msg));
    let r = connection.send_with_reply_and_block(msg, timeout)?;
    Ok(R::read(&mut r.iter_init())?)
}

pub trait IoMissioncenterMissionCenterGatherer {
    fn cpu_static_info(&self) -> Result<CpuStaticInfo, dbus::Error>;
    fn cpu_dynamic_info(&self) -> Result<CpuDynamicInfo, dbus::Error>;
    fn enumerate_gpus(&self) -> Result<Vec<Arc<str>>, dbus::Error>;
    fn gpu_dynamic_info(&self, gpu_id: &str) -> Result<GpuDynamicInfo, dbus::Error>;
    fn gpu_static_info(&self, gpu_id: &str) -> Result<GpuStaticInfo, dbus::Error>;
    fn processes(&self) -> Result<HashMap<u32, Process>, dbus::Error>;
    fn apps(&self) -> Result<HashMap<Arc<str>, App>, dbus::Error>;

    fn terminate_process(&self, process_id: u32) -> Result<(), dbus::Error>;
    fn kill_process(&self, process_id: u32) -> Result<(), dbus::Error>;
}

impl<'a> IoMissioncenterMissionCenterGatherer for blocking::Proxy<'a, blocking::Connection> {
    fn cpu_static_info(&self) -> Result<CpuStaticInfo, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "GetCpuStaticInfo",
            (),
        )
            .and_then(|r: (CpuStaticInfo, )| Ok(r.0))
    }

    fn cpu_dynamic_info(&self) -> Result<CpuDynamicInfo, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "GetCpuDynamicInfo",
            (),
        )
            .and_then(|r: (CpuDynamicInfo, )| Ok(r.0))
    }

    fn enumerate_gpus(&self) -> Result<Vec<Arc<str>>, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "EnumerateGPUs",
            (),
        )
            .and_then(|r: (ArcStrVec, )| Ok(r.0.into()))
    }

    fn gpu_dynamic_info(&self, gpu_id: &str) -> Result<GpuDynamicInfo, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "GetGPUDynamicInfo",
            (gpu_id, ),
        )
            .and_then(|r: (GpuDynamicInfo, )| Ok(r.0))
    }

    fn gpu_static_info(&self, gpu_id: &str) -> Result<GpuStaticInfo, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "GetGPUStaticInfo",
            (gpu_id, ),
        )
            .and_then(|r: (GpuStaticInfo, )| Ok(r.0))
    }

    fn processes(&self) -> Result<HashMap<u32, Process>, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "GetProcesses",
            (),
        )
            .and_then(|r: (ProcessMap, )| Ok(r.0.into()))
    }

    fn apps(&self) -> Result<HashMap<Arc<str>, App>, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "GetApps",
            (),
        )
            .and_then(|r: (AppMap, )| Ok(r.0.into()))
    }

    fn terminate_process(&self, process_id: u32) -> Result<(), Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "TerminateProcess",
            (process_id, ),
        )
            .and_then(|_: ()| Ok(()))
    }

    fn kill_process(&self, process_id: u32) -> Result<(), Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "KillProcess",
            (process_id, ),
        )
            .and_then(|_: ()| Ok(()))
    }
}

pub trait OrgFreedesktopDBusIntrospectable {
    fn introspect(&self) -> Result<String, dbus::Error>;
}

impl<'a> OrgFreedesktopDBusIntrospectable for blocking::Proxy<'a, blocking::Connection> {
    fn introspect(&self) -> Result<String, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "org.freedesktop.DBus.Introspectable",
            "Introspect",
            (),
        )
            .and_then(|r: (String, )| Ok(r.0))
    }
}

pub trait OrgFreedesktopDBusPeer {
    fn get_machine_id(&self) -> Result<String, dbus::Error>;
    fn ping(&self) -> Result<(), dbus::Error>;
}

impl<'a> OrgFreedesktopDBusPeer for blocking::Proxy<'a, blocking::Connection> {
    fn get_machine_id(&self) -> Result<String, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "org.freedesktop.DBus.Peer",
            "GetMachineId",
            (),
        )
            .and_then(|r: (String, )| Ok(r.0))
    }

    fn ping(&self) -> Result<(), dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "org.freedesktop.DBus.Peer",
            "Ping",
            (),
        )
    }
}
