/* sys_info_v2/dbus_interface/mod.rs
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

use std::{
    collections::HashMap,
    fmt::Write,
    mem::{align_of, size_of},
    num::NonZeroU32,
    rc::Rc,
    sync::Arc,
};

use adw::glib::g_critical;
use arrayvec::ArrayString;
use dbus::arg::{Append, Arg, Get, Iter, IterAppend, ReadAll, RefArg};
use dbus::{
    arg::ArgType,
    blocking::{LocalConnection, Proxy},
    Signature,
};
use static_assertions::const_assert;

pub use apps::{App, AppMap};
pub use arc_str_vec::ArcStrVec;
pub use cpu_dynamic_info::CpuDynamicInfo;
pub use cpu_static_info::CpuStaticInfo;
pub use disk_info::{DiskInfo, DiskInfoVec, DiskSmartInterface, DiskType};
pub use fan_info::{FanInfo, FanInfoVec};
pub use gpu_dynamic_info::{GpuDynamicInfo, GpuDynamicInfoVec};
pub use gpu_static_info::{GpuStaticInfo, GpuStaticInfoVec, OpenGLApi};
pub use processes::{Process, ProcessMap, ProcessUsageStats};
pub use service::{Service, ServiceMap};

mod apps;
mod arc_str_vec;
mod cpu_dynamic_info;
mod cpu_static_info;
mod disk_info;
mod fan_info;
mod gpu_dynamic_info;
mod gpu_static_info;
mod processes;
mod service;

pub const MC_GATHERER_OBJECT_PATH: &str = "/io/missioncenter/MissionCenter/Gatherer";
pub const MC_GATHERER_INTERFACE_NAME: &str = "io.missioncenter.MissionCenter.Gatherer";

// I don't know how to create one of these, so I just copy the one from the `dbus` crate.
#[allow(unused)]
struct TypeMismatchError {
    pub expected: ArgType,
    pub found: ArgType,
    pub position: u32,
}

impl TypeMismatchError {
    pub fn new(expected: ArgType, found: ArgType, position: u32) -> dbus::arg::TypeMismatchError {
        unsafe {
            std::mem::transmute(Self {
                expected,
                found,
                position,
            })
        }
    }
}

const_assert!(size_of::<TypeMismatchError>() == size_of::<dbus::arg::TypeMismatchError>());
const_assert!(align_of::<TypeMismatchError>() == align_of::<dbus::arg::TypeMismatchError>());

#[allow(unused)]
fn deser<'a, T, E>(
    iter: &mut dyn Iterator<Item = &'a dyn RefArg>,
    context: &str,
    expected_desc: &str,
    extractor: E,
) -> Option<T>
where
    E: FnOnce(&'a dyn RefArg) -> Option<T>,
{
    match iter.next() {
        None => {
            g_critical!(
                "MissionCenter::GathererDBusProxy",
                "Failed to read DBus data for {context}: Expected {expected_desc}, got None",
            );
            None
        }
        Some(arg) => match extractor(arg) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to read DBus data for {context}: Expected {expected_desc}, got {:?}",
                    arg.arg_type(),
                );
                None
            }
            Some(v) => Some(v),
        },
    }
}

#[allow(unused)]
fn deser_struct<'a>(
    iter: &mut dyn Iterator<Item = &'a dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<Box<dyn Iterator<Item = &'a dyn RefArg> + 'a>> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "STRUCT at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| arg.as_iter())
}

#[allow(unused)]
fn deser_array<'a>(
    iter: &mut dyn Iterator<Item = &'a dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<Box<dyn Iterator<Item = &'a dyn RefArg> + 'a>> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "STRUCT at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| arg.as_iter())
}

#[allow(unused)]
fn deser_u64(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<u64> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'t' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| arg.as_u64())
}

#[allow(unused)]
fn deser_u32(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<u32> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'u' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| {
        arg.as_u64().map(|v| v as u32)
    })
}

#[allow(unused)]
fn deser_u16(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<u16> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'q' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| {
        arg.as_u64().map(|v| v as u16)
    })
}

#[allow(unused)]
fn deser_u8(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<u8> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'y' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| {
        arg.as_u64().map(|v| v as u8)
    })
}

#[allow(unused)]
fn deser_usize(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<usize> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'t' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| {
        arg.as_u64().map(|v| v as usize)
    })
}

#[allow(unused)]
fn deser_i64(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<i64> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'x' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| arg.as_i64())
}

#[allow(unused)]
fn deser_i32(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<i32> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'i' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| {
        arg.as_i64().map(|i| i as i32)
    })
}

#[allow(unused)]
fn deser_i16(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<i16> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'n' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| {
        arg.as_i64().map(|i| i as i16)
    })
}

#[allow(unused)]
fn deser_bool(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<bool> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'b' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| {
        arg.as_u64().map(|v| match v {
            0 => false,
            _ => true,
        })
    })
}

#[allow(unused)]
fn deser_f64(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<f64> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'d' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| arg.as_f64())
}

#[allow(unused)]
fn deser_f32(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<f32> {
    // they both use 'd' because reasons
    deser_f64(iter, context, index).map(|v| v as f32)
}

#[allow(unused)]
fn deser_str(
    iter: &mut dyn Iterator<Item = &dyn RefArg>,
    context: &str,
    index: usize,
) -> Option<Arc<str>> {
    let mut description = ArrayString::<30>::new();
    write!(&mut description, "'s' at index {}", index).expect("Failed to write to ArrayString");
    deser(iter, context, &description, |arg| {
        arg.as_str().map(Arc::from)
    })
}

pub trait Gatherer {
    fn get_cpu_static_info(&self) -> Result<CpuStaticInfo, dbus::Error>;
    fn get_cpu_dynamic_info(&self) -> Result<CpuDynamicInfo, dbus::Error>;
    fn get_disks_info(&self) -> Result<Vec<DiskInfo>, dbus::Error>;
    fn eject_disk(
        &self,
        disk_id: &str,
        killall: bool,
        kill_pid: u32,
    ) -> Result<EjectResult, dbus::Error>;
    fn sata_smart_info(&self, disk_id: &str) -> Result<SataSmartResult, dbus::Error>;
    fn nvme_smart_info(&self, disk_id: &str) -> Result<NVMeSmartResult, dbus::Error>;
    fn get_fans_info(&self) -> Result<Vec<FanInfo>, dbus::Error>;
    fn get_gpu_list(&self) -> Result<Vec<Arc<str>>, dbus::Error>;
    fn get_gpu_static_info(&self) -> Result<Vec<GpuStaticInfo>, dbus::Error>;
    fn get_gpu_dynamic_info(&self) -> Result<Vec<GpuDynamicInfo>, dbus::Error>;
    fn get_apps(&self) -> Result<HashMap<Arc<str>, App>, dbus::Error>;
    fn get_processes(&self) -> Result<HashMap<u32, Process>, dbus::Error>;
    fn get_services(&self) -> Result<HashMap<Arc<str>, Service>, dbus::Error>;
    fn terminate_process(&self, process_id: u32) -> Result<(), dbus::Error>;
    fn kill_process(&self, process_id: u32) -> Result<(), dbus::Error>;
    fn enable_service(&self, service_name: &str) -> Result<(), dbus::Error>;
    fn disable_service(&self, service_name: &str) -> Result<(), dbus::Error>;
    fn start_service(&self, service_name: &str) -> Result<(), dbus::Error>;
    fn stop_service(&self, service_name: &str) -> Result<(), dbus::Error>;
    fn restart_service(&self, service_name: &str) -> Result<(), dbus::Error>;
    fn get_service_logs(
        &self,
        service_name: &str,
        pid: Option<NonZeroU32>,
    ) -> Result<Arc<str>, dbus::Error>;
}

impl<'a> Gatherer for Proxy<'a, Rc<LocalConnection>> {
    fn get_cpu_static_info(&self) -> Result<CpuStaticInfo, dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "GetCPUStaticInfo", ())
    }

    fn get_cpu_dynamic_info(&self) -> Result<CpuDynamicInfo, dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "GetCPUDynamicInfo", ())
    }

    fn get_disks_info(&self) -> Result<Vec<DiskInfo>, dbus::Error> {
        let res: Result<DiskInfoVec, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetDisksInfo", ());
        res.map(|v| v.into())
    }

    fn eject_disk(
        &self,
        disk_id: &str,
        killall: bool,
        kill_pid: u32,
    ) -> Result<EjectResult, dbus::Error> {
        self.method_call(
            MC_GATHERER_INTERFACE_NAME,
            "EjectDisk",
            (disk_id, killall, kill_pid),
        )
    }

    fn sata_smart_info(&self, disk_id: &str) -> Result<SataSmartResult, dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "SataSmartInfo", (disk_id,))
    }

    fn nvme_smart_info(&self, disk_id: &str) -> Result<NVMeSmartResult, dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "NVMeSmartInfo", (disk_id,))
    }

    fn get_fans_info(&self) -> Result<Vec<FanInfo>, dbus::Error> {
        let res: Result<FanInfoVec, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetFansInfo", ());
        res.map(|v| v.into())
    }

    fn get_gpu_list(&self) -> Result<Vec<Arc<str>>, dbus::Error> {
        let res: Result<ArcStrVec, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetGPUList", ());
        res.map(|v| v.into())
    }

    fn get_gpu_static_info(&self) -> Result<Vec<GpuStaticInfo>, dbus::Error> {
        let res: Result<GpuStaticInfoVec, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetGPUStaticInfo", ());
        res.map(|v| v.into())
    }

    fn get_gpu_dynamic_info(&self) -> Result<Vec<GpuDynamicInfo>, dbus::Error> {
        let res: Result<GpuDynamicInfoVec, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetGPUDynamicInfo", ());
        res.map(|v| v.into())
    }

    fn get_apps(&self) -> Result<HashMap<Arc<str>, App>, dbus::Error> {
        let res: Result<AppMap, _> = self.method_call(MC_GATHERER_INTERFACE_NAME, "GetApps", ());
        res.map(|v| v.into())
    }

    fn get_processes(&self) -> Result<HashMap<u32, Process>, dbus::Error> {
        let res: Result<ProcessMap, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetProcesses", ());
        res.map(|v| v.into())
    }

    fn get_services(&self) -> Result<HashMap<Arc<str>, Service>, dbus::Error> {
        let res: Result<ServiceMap, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetServices", ());
        res.map(|v| v.into())
    }

    fn terminate_process(&self, process_id: u32) -> Result<(), dbus::Error> {
        self.method_call(
            MC_GATHERER_INTERFACE_NAME,
            "TerminateProcess",
            (process_id,),
        )
    }

    fn kill_process(&self, process_id: u32) -> Result<(), dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "KillProcess", (process_id,))
    }

    fn enable_service(&self, service_name: &str) -> Result<(), dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "EnableService", (service_name,))
    }

    fn disable_service(&self, service_name: &str) -> Result<(), dbus::Error> {
        self.method_call(
            MC_GATHERER_INTERFACE_NAME,
            "DisableService",
            (service_name,),
        )
    }

    fn start_service(&self, service_name: &str) -> Result<(), dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "StartService", (service_name,))
    }

    fn stop_service(&self, service_name: &str) -> Result<(), dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "StopService", (service_name,))
    }

    fn restart_service(&self, service_name: &str) -> Result<(), dbus::Error> {
        self.method_call(
            MC_GATHERER_INTERFACE_NAME,
            "RestartService",
            (service_name,),
        )
    }

    fn get_service_logs(
        &self,
        service_name: &str,
        pid: Option<NonZeroU32>,
    ) -> Result<Arc<str>, dbus::Error> {
        let res: Result<(String,), _> = self.method_call(
            MC_GATHERER_INTERFACE_NAME,
            "GetServiceLogs",
            (service_name, pid.map(|v| v.get()).unwrap_or(0)),
        );
        res.map(|v| Arc::<str>::from(v.0))
    }
}

#[derive(Default, Debug)]
pub struct EjectResult {
    pub success: bool,

    pub blocking_processes: Vec<(u32, Vec<String>, Vec<String>)>,
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

impl ReadAll for EjectResult {
    fn read(i: &mut Iter) -> Result<Self, dbus::arg::TypeMismatchError> {
        i.get().ok_or(TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for EjectResult {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut this = Self::default();

        let dynamic_info = match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(id) => id,
        };

        let mut dynamic_info = match dynamic_info.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return None;
            }
            Some(i) => i,
        };
        let dynamic_info = dynamic_info.as_mut();

        this.success = match deser_bool(dynamic_info, "GathererDBusProxy", 0) {
            None => return None,
            Some(i) => i,
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get boolean: Expected '1: Vec<>', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Vec<Arc<str>>: Expected '1: Vec<>', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(mut block_list) => {
                    let block_list = block_list.as_mut();
                    loop {
                        match Iterator::next(block_list) {
                            None => {
                                break;
                            }
                            Some(block_tuple) => match block_tuple.as_iter() {
                                None => {
                                    return None;
                                }
                                Some(mut tuple_iter) => {
                                    let mut new_item = (u32::MAX, vec![], vec![]);

                                    let tuple_iter = tuple_iter.as_mut();
                                    match Iterator::next(tuple_iter) {
                                        None => {}
                                        Some(arg) => {
                                            match arg.as_u64() {
                                                None => {
                                                    return None;
                                                }
                                                Some(arr) => {
                                                    new_item.0 = arr as u32;
                                                }
                                            };
                                        }
                                    }
                                    match Iterator::next(tuple_iter) {
                                        None => {}
                                        Some(arg) => {
                                            match arg.as_iter() {
                                                None => {
                                                    return None;
                                                }
                                                Some(arr) => {
                                                    for strink in arr {
                                                        new_item.1.push(
                                                            strink
                                                                .as_str()
                                                                .unwrap_or("")
                                                                .to_string(),
                                                        );
                                                    }
                                                }
                                            };
                                        }
                                    }
                                    match Iterator::next(tuple_iter) {
                                        None => {}
                                        Some(arg) => {
                                            match arg.as_iter() {
                                                None => {
                                                    return None;
                                                }
                                                Some(arr) => {
                                                    for strink in arr {
                                                        new_item.2.push(
                                                            strink
                                                                .as_str()
                                                                .unwrap_or("")
                                                                .to_string(),
                                                        );
                                                    }
                                                }
                                            };
                                        }
                                    }

                                    this.blocking_processes.push(new_item);
                                }
                            },
                        }
                    }
                }
            },
        }

        Some(this.into())
    }
}

#[derive(Debug, Clone)]
pub struct SataSmartEntry {
    pub id: u8,
    pub name: String,
    pub flags: u16,
    pub value: i32,
    pub worst: i32,
    pub threshold: i32,
    pub pretty: i64,
    pub pretty_unit: i32,
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
        Signature::from("(ysqiiixa)")
    }
}

impl ReadAll for SataSmartEntry {
    fn read(i: &mut Iter) -> Result<Self, dbus::arg::TypeMismatchError> {
        i.get().ok_or(TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl From<&dyn RefArg> for SataSmartEntry {
    fn from(value: &dyn RefArg) -> Self {
        use gtk::glib::g_critical;

        let mut this = Self::default();

        let mut dynamic_info = match value.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return this;
            }
            Some(i) => i,
        };
        let dynamic_info = dynamic_info.as_mut();

        this.id = match deser_u8(dynamic_info, "GathererDBusProxy", 0) {
            Some(i) => i,
            None => return this,
        };

        this.name = match deser_str(dynamic_info, "GathererDBusProxy", 1) {
            Some(i) => i.to_string(),
            None => return this,
        };

        this.flags = match deser_u16(dynamic_info, "GathererDBusProxyFlags", 2) {
            Some(i) => i,
            None => return this,
        };

        this.value = match deser_i32(dynamic_info, "GathererDBusProxyValue", 3) {
            Some(i) => i,
            None => return this,
        };

        this.worst = match deser_i32(dynamic_info, "GathererDBusProxyWork", 4) {
            Some(i) => i,
            None => return this,
        };

        this.threshold = match deser_i32(dynamic_info, "GathererDBusProxyThreshold", 5) {
            Some(i) => i,
            None => return this,
        };

        this.pretty = match deser_i64(dynamic_info, "GathererDBusProxyPretty", 6) {
            Some(i) => i,
            None => return this,
        };

        this.pretty_unit = match deser_i32(dynamic_info, "GathererDBusProxyPrettyUnit", 7) {
            Some(i) => i,
            None => return this,
        };

        this
    }
}

impl<'a> Get<'a> for SataSmartEntry {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut this = Self::default();

        let dynamic_info = match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(id) => id,
        };

        let mut dynamic_info = match dynamic_info.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return None;
            }
            Some(i) => i,
        };
        let dynamic_info = dynamic_info.as_mut();

        this.id = match deser_u8(dynamic_info, "SataSmartEntry", 0) {
            Some(v) => v,
            None => return None,
        };

        this.name = match deser_str(dynamic_info, "SataSmartEntry", 1) {
            Some(v) => v.to_string(),
            None => return None,
        };

        this.flags = match deser_u16(dynamic_info, "SataSmartEntry", 2) {
            Some(v) => v,
            None => return None,
        };

        this.value = match deser_i32(dynamic_info, "SataSmartEntry", 3) {
            Some(v) => v,
            None => return None,
        };

        this.worst = match deser_i32(dynamic_info, "SataSmartEntry", 4) {
            Some(v) => v,
            None => return None,
        };

        this.threshold = match deser_i32(dynamic_info, "SataSmartEntry", 5) {
            Some(v) => v,
            None => return None,
        };

        this.pretty = match deser_i64(dynamic_info, "SataSmartEntry", 6) {
            Some(v) => v,
            None => return None,
        };

        this.pretty_unit = match deser_i32(dynamic_info, "SataSmartEntry", 7) {
            Some(i) => i,
            None => return None,
        };

        Some(this)
    }
}

#[allow(non_camel_case_types)]
#[derive(Default, Copy, Clone, Debug, Eq, PartialEq)]
pub enum SmartTestResult {
    #[default]
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

impl From<u64> for SmartTestResult {
    fn from(value: u64) -> Self {
        match value {
            1 => SmartTestResult::Success,
            2 => SmartTestResult::Aborted,
            3 => SmartTestResult::FatalError,
            4 => SmartTestResult::InProgress,
            5 => SmartTestResult::Interrupted,
            6 => SmartTestResult::ErrorUnknown,
            7 => SmartTestResult::ErrorElectrical,
            8 => SmartTestResult::ErrorServo,
            9 => SmartTestResult::ErrorRead,
            10 => SmartTestResult::ErrorHandling,
            11 => SmartTestResult::CtrlReset,
            12 => SmartTestResult::NsRemoved,
            13 => SmartTestResult::AbortedFormat,
            14 => SmartTestResult::UnknownSegmentFailed,
            15 => SmartTestResult::KnownSegmentFailed,
            16 => SmartTestResult::AbortedUnknown,
            17 => SmartTestResult::AbortedSanitize,
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

impl From<&dyn RefArg> for CommonSmartResult {
    fn from(value: &dyn RefArg) -> Self {
        use gtk::glib::g_critical;

        let mut this = Self::default();

        let mut dynamic_info = match value.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return this;
            }
            Some(i) => i,
        };
        let dynamic_info = dynamic_info.as_mut();

        this.success = match deser_bool(dynamic_info, "GathererDBusProxy", 0) {
            Some(s) => s,
            None => return this,
        };

        this.powered_on_seconds = match deser_u64(dynamic_info, "GathererDBusProxy", 1) {
            Some(s) => s,
            None => return this,
        };

        this.last_update_time = match deser_u64(dynamic_info, "GathererDBusProxy", 2) {
            Some(s) => s,
            None => return this,
        };

        this.test_result = match deser_u64(dynamic_info, "GathererDBusProxy", 3) {
            Some(s) => SmartTestResult::from(s),
            None => return this,
        };

        this
    }
}

#[derive(Debug)]
pub struct SataSmartResult {
    pub common_smart_result: CommonSmartResult,

    pub blocking_processes: Vec<SataSmartEntry>,
}

impl Default for SataSmartResult {
    fn default() -> Self {
        Self {
            common_smart_result: Default::default(),
            blocking_processes: vec![],
        }
    }
}

impl Append for SataSmartResult {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append((
            self.common_smart_result.clone(),
            self.blocking_processes.clone(),
        ));
    }
}

impl Arg for SataSmartResult {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(btsa(ysqiiixi))")
    }
}

impl ReadAll for SataSmartResult {
    fn read(i: &mut Iter) -> Result<Self, dbus::arg::TypeMismatchError> {
        i.get().ok_or(TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for SataSmartResult {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut this = Self::default();

        let dynamic_info = match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(id) => id,
        };

        let mut dynamic_info = match dynamic_info.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return None;
            }
            Some(i) => i,
        };
        let dynamic_info = dynamic_info.as_mut();

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get STRUCT: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(arg) => this.common_smart_result = CommonSmartResult::from(arg),
        }

        match deser_array(dynamic_info, "GathererDBusProxy", 1) {
            Some(arr) => {
                for block in arr {
                    let entry = SataSmartEntry::from(block);
                    this.blocking_processes.push(entry);
                }
            }
            None => return None,
        }

        Some(this)
    }
}

#[derive(Debug)]
pub struct NVMeSmartResult {
    pub common_smart_result: CommonSmartResult,

    pub avail_spare: u8,
    pub spare_thresh: u8,
    pub percent_used: u8,
    pub total_data_read: u64,
    pub total_data_written: u64,
    pub ctrl_busy_minutes: u64,
    pub power_cycles: u64,
    pub unsafe_shutdowns: u64,
    pub media_errors: u64,
    pub num_err_log_entries: u64,
    pub temp_sensors: Vec<u16>,
    pub wctemp: u16,
    pub cctemp: u16,
    pub warning_temp_time: u32,
    pub critical_temp_time: u32,
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
        Signature::from("(btsyyyttttttta(q)qquu)")
    }
}

impl ReadAll for NVMeSmartResult {
    fn read(i: &mut Iter) -> Result<Self, dbus::arg::TypeMismatchError> {
        i.get().ok_or(TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for NVMeSmartResult {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut this = Self::default();

        let dynamic_info = match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(id) => id,
        };

        let mut dynamic_info = match dynamic_info.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return None;
            }
            Some(i) => i,
        };
        let dynamic_info = dynamic_info.as_mut();

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get STRUCT: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(arg) => this.common_smart_result = CommonSmartResult::from(arg),
        }

        this.avail_spare = match deser_u8(dynamic_info, "GathererDBusProxy", 1) {
            Some(i) => i,
            None => return None,
        };

        this.spare_thresh = match deser_u8(dynamic_info, "GathererDBusProxy", 2) {
            Some(i) => i,
            None => return None,
        };

        this.percent_used = match deser_u8(dynamic_info, "GathererDBusProxy", 3) {
            Some(i) => i,
            None => return None,
        };

        this.total_data_read = match deser_u64(dynamic_info, "GathererDBusProxy", 4) {
            Some(i) => i,
            None => return None,
        };

        this.total_data_written = match deser_u64(dynamic_info, "GathererDBusProxy", 5) {
            Some(i) => i,
            None => return None,
        };

        this.ctrl_busy_minutes = match deser_u64(dynamic_info, "GathererDBusProxy", 6) {
            Some(i) => i,
            None => return None,
        };

        this.power_cycles = match deser_u64(dynamic_info, "GathererDBusProxy", 7) {
            Some(i) => i,
            None => return None,
        };

        this.unsafe_shutdowns = match deser_u64(dynamic_info, "GathererDBusProxy", 8) {
            Some(i) => i,
            None => return None,
        };

        this.media_errors = match deser_u64(dynamic_info, "GathererDBusProxy", 9) {
            Some(i) => i,
            None => return None,
        };

        this.num_err_log_entries = match deser_u64(dynamic_info, "GathererDBusProxy", 10) {
            Some(i) => i,
            None => return None,
        };

        match deser_array(dynamic_info, "GathererDBusProxy", 11) {
            Some(i) => {
                for temp in i {
                    this.temp_sensors
                        .push(temp.as_u64().unwrap_or_default() as u16);
                }
            }
            None => return None,
        }

        this.wctemp = match deser_u16(dynamic_info, "GathererDBusProxy", 12) {
            Some(i) => i,
            None => return None,
        };

        this.cctemp = match deser_u16(dynamic_info, "GathererDBusProxy", 13) {
            Some(i) => i,
            None => return None,
        };

        this.warning_temp_time = match deser_u32(dynamic_info, "GathererDBusProxy", 14) {
            Some(i) => i,
            None => return None,
        };

        this.critical_temp_time = match deser_u32(dynamic_info, "GathererDBusProxy", 15) {
            Some(i) => i,
            None => return None,
        };

        Some(this)
    }
}
