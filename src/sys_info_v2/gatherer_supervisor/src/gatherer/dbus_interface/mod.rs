use dbus::{arg::*, blocking, blocking::BlockingSender, strings::*};

pub use gpu_dynamic_info::*;
pub use gpu_static_info::*;

mod gpu_dynamic_info;
mod gpu_static_info;

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
    fn enumerate_gpus(&self) -> Result<Vec<String>, dbus::Error>;
    // fn get_apps(&self) -> Result<Vec<(String, String, String, String, Vec<u32>, (f64, f64, f64, f64, f64))>, dbus::Error>;
    // fn get_cpu_dynamic_info(&self) -> Result<(f64, f64, Vec<f64>, Vec<f64>, u64, f64, u64, u64, u64, u64), dbus::Error>;
    // fn get_cpu_static_info(&self) -> Result<(String, u32, u8, u64, u8, u8, u64, u64, u64, u64), dbus::Error>;
    fn gpu_dynamic_info(&self, gpu_id: &str) -> Result<GpuDynamicInfo, dbus::Error>;
    fn gpu_static_info(&self, gpu_id: &str) -> Result<GpuStaticInfo, dbus::Error>;
    // fn get_processes(&self) -> Result<Vec<(String, Vec<String>, String, u8, u32, u32, (f64, f64, f64, f64, f64), u64)>, dbus::Error>;
}

impl<'a> IoMissioncenterMissionCenterGatherer for blocking::Proxy<'a, blocking::Connection> {
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

    // fn get_apps(&self) -> Result<Vec<(String, String, String, String, Vec<u32>, (f64, f64, f64, f64, f64))>, dbus::Error> {
    //     self.method_call("io.missioncenter.MissionCenter.Gatherer", "GetApps", ())
    //         .and_then(|r: (Vec<(String, String, String, String, Vec<u32>, (f64, f64, f64, f64, f64))>, )| Ok(r.0, ))
    // }
    //
    // fn get_cpu_dynamic_info(&self) -> Result<(f64, f64, Vec<f64>, Vec<f64>, u64, f64, u64, u64, u64, u64), dbus::Error> {
    //     self.method_call("io.missioncenter.MissionCenter.Gatherer", "GetCpuDynamicInfo", ())
    //         .and_then(|r: ((f64, f64, Vec<f64>, Vec<f64>, u64, f64, u64, u64, u64, u64), )| Ok(r.0, ))
    // }
    //
    // fn get_cpu_static_info(&self) -> Result<(String, u32, u8, u64, u8, u8, u64, u64, u64, u64), dbus::Error> {
    //     self.method_call("io.missioncenter.MissionCenter.Gatherer", "GetCpuStaticInfo", ())
    //         .and_then(|r: ((String, u32, u8, u64, u8, u8, u64, u64, u64, u64), )| Ok(r.0, ))
    // }
    //
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
    //
    // fn get_processes(&self) -> Result<Vec<(String, Vec<String>, String, u8, u32, u32, (f64, f64, f64, f64, f64), u64)>, dbus::Error> {
    //     self.method_call("io.missioncenter.MissionCenter.Gatherer", "GetProcesses", ())
    //         .and_then(|r: (Vec<(String, Vec<String>, String, u8, u32, u32, (f64, f64, f64, f64, f64), u64)>, )| Ok(r.0, ))
    // }
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
