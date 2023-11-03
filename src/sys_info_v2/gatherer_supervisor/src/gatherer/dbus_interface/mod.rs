use dbus::{arg::*, blocking, blocking::BlockingSender, strings::*};

pub use apps::*;
pub use cpu_dynamic_info::*;
pub use cpu_static_info::*;
pub use gpu_dynamic_info::*;
pub use gpu_static_info::*;
pub use processes::*;

mod apps;
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
    fn enumerate_gpus(&self) -> Result<Vec<String>, dbus::Error>;
    fn gpu_dynamic_info(&self, gpu_id: &str) -> Result<GpuDynamicInfo, dbus::Error>;
    fn gpu_static_info(&self, gpu_id: &str) -> Result<GpuStaticInfo, dbus::Error>;
    fn processes(&self) -> Result<Vec<Process>, dbus::Error>;
    fn apps(&self) -> Result<Vec<App>, dbus::Error>;
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
        .and_then(|r: (CpuStaticInfo,)| Ok(r.0))
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
        .and_then(|r: (CpuDynamicInfo,)| Ok(r.0))
    }

    fn enumerate_gpus(&self) -> Result<Vec<String>, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "EnumerateGPUs",
            (),
        )
        .and_then(|r: (Vec<String>,)| Ok(r.0))
    }

    fn gpu_dynamic_info(&self, gpu_id: &str) -> Result<GpuDynamicInfo, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "GetGPUDynamicInfo",
            (gpu_id,),
        )
        .and_then(|r: (GpuDynamicInfo,)| Ok(r.0))
    }

    fn gpu_static_info(&self, gpu_id: &str) -> Result<GpuStaticInfo, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "GetGPUStaticInfo",
            (gpu_id,),
        )
        .and_then(|r: (GpuStaticInfo,)| Ok(r.0))
    }

    fn processes(&self) -> Result<Vec<Process>, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "GetProcesses",
            (),
        )
        .and_then(|r: (Vec<Process>,)| Ok(r.0))
    }

    fn apps(&self) -> Result<Vec<App>, dbus::Error> {
        dbus_method_call(
            &self.connection,
            &self.destination,
            &self.path,
            self.timeout,
            "io.missioncenter.MissionCenter.Gatherer",
            "GetApps",
            (),
        )
        .and_then(|r: (Vec<App>,)| Ok(r.0))
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
        .and_then(|r: (String,)| Ok(r.0))
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
        .and_then(|r: (String,)| Ok(r.0))
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
