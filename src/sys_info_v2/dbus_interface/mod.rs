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
    mem::{align_of, size_of},
    num::NonZeroU32,
    rc::Rc,
    sync::Arc,
};

use dbus::{arg::ArgType, blocking::{LocalConnection, Proxy}, Signature};
use dbus::arg::{Append, Arg, Get, Iter, IterAppend, ReadAll, RefArg};
use static_assertions::const_assert;

pub use apps::{App, AppMap};
pub use arc_str_vec::ArcStrVec;
pub use cpu_dynamic_info::CpuDynamicInfo;
pub use cpu_static_info::CpuStaticInfo;
pub use disk_info::{DiskInfo, DiskInfoVec, DiskType, DiskSmartInterface};
pub use fan_info::{FanInfo, FanInfoVec};
pub use gpu_dynamic_info::{GpuDynamicInfo, GpuDynamicInfoVec};
pub use gpu_static_info::{GpuStaticInfo, GpuStaticInfoVec, OpenGLApi};
pub use processes::{Process, ProcessMap, ProcessUsageStats};
pub use service::{Service, ServiceMap};
use crate::sys_info_v2::dbus_interface::processes::ProcessState;

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

pub trait Gatherer {
    fn get_cpu_static_info(&self) -> Result<CpuStaticInfo, dbus::Error>;
    fn get_cpu_dynamic_info(&self) -> Result<CpuDynamicInfo, dbus::Error>;
    fn get_disks_info(&self) -> Result<Vec<DiskInfo>, dbus::Error>;
    fn eject_disk(&self, disk_id: &str, use_force: bool) -> Result<EjectResult, dbus::Error>;
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

    fn eject_disk(&self, disk_id: &str, use_force: bool) -> Result<EjectResult, dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "EjectDisk", (disk_id, use_force))
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

#[derive(Debug)]
pub struct EjectResult {
    pub success: bool,

    pub blocking_processes: Vec<(u32, Vec<String>, Vec<String>)>,
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
        ia.append((
            self.success,
            self.blocking_processes.clone(),
        ));
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

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get boolean: Expected '0: boolean', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get boolean: Expected '0: boolean', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.success = arr != 0
                }
            },
        }

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
                    let mut block_list = block_list.as_mut();
                    while true {
                        match Iterator::next(block_list) {
                            None => { break; }
                            Some(block_tuple) => {
                                match block_tuple.as_iter() {
                                    None => {
                                        println!("f");
                                        return None;
                                    }
                                    Some(mut tuple_iter) => {
                                        let mut new_item = (u32::MAX, vec![], vec![]);

                                        let mut tuple_iter = tuple_iter.as_mut();
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
                                                            new_item.1.push(strink.as_str().unwrap_or("").to_string());
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
                                                            new_item.2.push(strink.as_str().unwrap_or("").to_string());
                                                        }
                                                    }
                                                };
                                            }
                                        }

                                        this.blocking_processes.push(new_item);
                                    }
                                }
                            }
                        }
                    }
/*                    for s in block_list {
                        let s = match s.as_iter() {
                            None => {continue;}
                            Some(i) => i,
                        };
                        let mut s = s.as_mut();
                        match Iterator::next(s) {
                            None => {}
                            Some(arg) => {
                                println!("0 {:?}", arg);
                            }
                        }
                        match Iterator::next(s) {
                            None => {}
                            Some(arg) => {
                                println!("1 {:?}", arg);
                            }
                        }
                        match Iterator::next(s) {
                            None => {}
                            Some(arg) => {
                                println!("2 {:?}", arg);
                            }
                        }
                        continue;
/*                        let mut new_item = (u32::MAX, vec![], vec![]);
                        if let Some(s) = s.as_u64() {
                            new_item.0 = s as u32;
                        } else {
                            println!("WTF");
                        }

                        if let Some(s) = s.as_iter() {
                            for s in s {
                                if let Some(s) = s.as_str() {
                                    new_item.1.push(s.to_string());
                                } else {
                                    println!("WTFF {:?}", s);
                                }
                            }
                        } else {
                            println!("WTFY");
                        }

                        if let Some(s) = s.as_iter() {
                            for s in s {
                                if let Some(s) = s.as_str() {
                                    new_item.2.push(s.to_string());
                                } else {
                                    println!("WTFF");
                                }
                            }
                        } else {
                            println!("WTFY");
                        }

                        this.blocking_processes.push(new_item);*/
                    }
*/                }
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

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get boolean: Expected '0: u8', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u8: Expected '0: u8', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.id = arr as u8
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get boolean: Expected '1: String', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get String: Expected '1: String', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.name = arr.to_string();
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u16: Expected '2: u16', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u16: Expected '2: String', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.flags = arr as u16;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get i32: Expected '3: i32', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_i64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get i32: Expected '3: i32', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.value = arr as i32;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get i32: Expected '4: i32', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_i64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get i32: Expected '4: i32', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.worst = arr as i32;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get i32: Expected '5: i32', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_i64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get i32: Expected '5: i32', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.threshold = arr as i32;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get i64: Expected '6: i64', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_i64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get i64: Expected '6: i64', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.pretty = arr;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get i32: Expected '7: i32', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_i64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get i32: Expected '7: i32', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.pretty_unit = arr as i32;
                }
            },
        }

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

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get boolean: Expected '0: u8', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u8: Expected '0: u8', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.id = arr as u8
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get boolean: Expected '1: String', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get String: Expected '1: String', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.name = arr.to_string();
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u16: Expected '2: u16', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u16: Expected '2: String', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.flags = arr as u16;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get i32: Expected '3: i32', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_i64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get i32: Expected '3: i32', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.value = arr as i32;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get i32: Expected '4: i32', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_i64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get i32: Expected '4: i32', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.worst = arr as i32;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get i32: Expected '5: i32', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_i64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get i32: Expected '5: i32', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.threshold = arr as i32;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get i64: Expected '6: i64', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_i64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get i64: Expected '6: i64', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.pretty = arr;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get i32: Expected '7: i32', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_i64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get i32: Expected '7: i32', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.pretty_unit = arr as i32;
                }
            },
        }

        Some(this)
    }
}

#[derive(Clone, Debug)]
pub struct CommonSmartResult {
    pub success: bool,

    pub powered_on_seconds: u64,
    pub status: Arc<str>,
}

impl Default for CommonSmartResult {
    fn default() -> Self {
        Self {
            success: false,
            powered_on_seconds: 0,
            status: Arc::from(""),
        }
    }
}

impl Append for CommonSmartResult {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append_struct(|ia| {
            ia.append(self.success);
            ia.append(self.powered_on_seconds);
            ia.append(self.status.as_ref());
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

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get boolean: Expected '0: boolean', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get boolean: Expected '0: boolean', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.success = arr != 0
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u64: Expected '1: u64', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u64: Expected '1: u64', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.powered_on_seconds = arr;
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get String: Expected '2: String', got None",
                );
                return this;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get String: Expected '2: String', got {:?}",
                        arg.arg_type(),
                    );
                    return this;
                }
                Some(arr) => {
                    this.status = Arc::from(arr);
                }
            },
        }

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
            Some(arg) => {
                this.common_smart_result = CommonSmartResult::from(arg)
            }
        }

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
                Some(block_list) => {
                    for block in block_list {
                        let entry = SataSmartEntry::from(block);
                        this.blocking_processes.push(entry);
                    }
                }
            }
        };

        Some(this)
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

/*        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get boolean: Expected '0: boolean', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get boolean: Expected '0: boolean', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(mut arr) => {
                    let dynamic_info = arr.as_mut();
                    match Iterator::next(dynamic_info) {
                        None => {
                            g_critical!(
                                "MissionCenter::GathererDBusProxy",
                                "Failed to get boolean: Expected '0: boolean', got None",
                            );
                            return None;
                        }
                        Some(arg) => match arg.as_u64() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get boolean: Expected '0: boolean', got {:?}",
                                    arg.arg_type(),
                                );
                                return None;
                            }
                            Some(arr) => {
                                this.common_smart_result.success = arr != 0
                            }
                        },
                    }

                    match Iterator::next(dynamic_info) {
                        None => {
                            g_critical!(
                                "MissionCenter::GathererDBusProxy",
                                "Failed to get u64: Expected '1: u64', got None",
                            );
                            return None;
                        }
                        Some(arg) => match arg.as_u64() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get u64: Expected '1: u64', got {:?}",
                                    arg.arg_type(),
                                );
                                return None;
                            }
                            Some(arr) => {
                                this.common_smart_result.powered_on_seconds = arr;
                            }
                        },
                    }

                    match Iterator::next(dynamic_info) {
                        None => {
                            g_critical!(
                                "MissionCenter::GathererDBusProxy",
                                "Failed to get String: Expected '2: String', got None",
                            );
                            return None;
                        }
                        Some(arg) => match arg.as_str() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get String: Expected '2: String', got {:?}",
                                    arg.arg_type(),
                                );
                                return None;
                            }
                            Some(arr) => {
                                this.common_smart_result.status = arr.to_string();
                            }
                        },
                    }

                    // this.common_smart_result = CommonSmartResult::from(arr.as_mut());
                }
            },
        };
*/
        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get STRUCT: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(arg) => {
                this.common_smart_result = CommonSmartResult::from(arg)
            }
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u8: Expected '1: u8', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u8: Expected '1: u8', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.avail_spare = arr as u8
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u8: Expected '2: u8', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u8: Expected '2: u8', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.spare_thresh = arr as u8
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u8: Expected '3: u8', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u8: Expected '3: u8', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.percent_used = arr as u8
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u64: Expected '4: u64', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u64: Expected '4: u64', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.total_data_read = arr
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u64: Expected '5: u64', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u64: Expected '5: u64', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.total_data_written = arr
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u64: Expected '6: u64', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u64: Expected '6: u64', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.ctrl_busy_minutes = arr
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u64: Expected '7: u64', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u64: Expected '7: u64', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.power_cycles = arr
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u64: Expected '8: u64', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u64: Expected '8: u64', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.unsafe_shutdowns = arr
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u64: Expected '9: u64', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u64: Expected '9: u64', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.media_errors = arr
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u64: Expected '10: u64', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u64: Expected '10: u64', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.num_err_log_entries = arr
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Arr: Expected '11: Arr', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Arr: Expected '11: Arr', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(mut arr) => {
                    /*let arr = arr.as_mut();

                    for j in 0..8 {
                        this.temp_sensors.push(
                            Iterator::next(arr).unwrap().as_u64().unwrap() as u16,
                        )
                    }*/
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u16: Expected '12: u16', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u16: Expected '12: u16', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.wctemp = arr as u16
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u16: Expected '13: u16', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u16: Expected '13: u16', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.cctemp = arr as u16
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u32: Expected '14: u32', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u32: Expected '14: u32', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.warning_temp_time = arr as u32
                }
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get u32: Expected '15: u32', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get u32: Expected '15: u32', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    this.critical_temp_time = arr as u32
                }
            },
        };

        Some(this)
    }
}
