/* sys_info_v2/dbus_interface/gpu_dynamic_info.rs
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
use std::num::NonZero;
use std::sync::Arc;

use dbus::{arg::*, strings::*};
use gtk::glib::g_critical;

#[derive(Debug, Clone)]
pub struct GpuDynamicInfo {
    pub id: Arc<str>,
    pub temp_celsius: Option<u32>,
    pub fan_speed_percent: Option<u32>,
    pub util_percent: Option<u32>,
    pub power_draw_watts: Option<f32>,
    pub power_draw_max_watts: Option<f32>,
    pub clock_speed_mhz: Option<u32>,
    pub clock_speed_max_mhz: Option<NonZero<u32>>,
    pub mem_speed_mhz: Option<u32>,
    pub mem_speed_max_mhz: Option<NonZero<u32>>,
    pub free_memory: Option<u64>,
    pub used_memory: Option<u64>,
    pub used_shared_memory: Option<u64>,
    pub encoder_percent: Option<u32>,
    pub decoder_percent: Option<u32>,
}

impl Default for GpuDynamicInfo {
    fn default() -> Self {
        Self {
            id: Arc::from(""),
            temp_celsius: None,
            fan_speed_percent: None,
            util_percent: None,
            power_draw_watts: None,
            power_draw_max_watts: None,
            clock_speed_mhz: None,
            clock_speed_max_mhz: None,
            mem_speed_mhz: None,
            mem_speed_max_mhz: None,
            free_memory: None,
            used_memory: None,
            used_shared_memory: None,
            encoder_percent: None,
            decoder_percent: None,
        }
    }
}

pub struct GpuDynamicInfoVec(pub Vec<GpuDynamicInfo>);

impl From<GpuDynamicInfoVec> for Vec<GpuDynamicInfo> {
    fn from(v: GpuDynamicInfoVec) -> Self {
        v.0
    }
}

impl From<Vec<GpuDynamicInfo>> for GpuDynamicInfoVec {
    fn from(v: Vec<GpuDynamicInfo>) -> Self {
        GpuDynamicInfoVec(v)
    }
}

impl Arg for GpuDynamicInfoVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        dbus::Signature::from("a(suuudduuuuttuu)")
    }
}

impl ReadAll for GpuDynamicInfoVec {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for GpuDynamicInfoVec {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        let mut result = vec![];

        match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Vec<DiskInfo>: Expected '0: ARRAY', got None",
                );
                None
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Vec<DiskInfo>: Expected '0: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    None
                }
                Some(arr) => {
                    for dynamic_info in arr {
                        let mut this = GpuDynamicInfo::default();

                        let mut dynamic_info = match dynamic_info.as_iter() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '0: STRUCT', got None, failed to iterate over fields",
                                );
                                return None;
                            }
                            Some(i) => i,
                        };
                        let dynamic_info = dynamic_info.as_mut();

                        this.id = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '0: s', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '0: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(id) => Arc::<str>::from(id),
                            },
                        };

                        this.temp_celsius = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '1: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '1: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(temp) => {
                                    let temp = temp as u32;
                                    if temp == u32::MAX {
                                        None
                                    } else {
                                        Some(temp)
                                    }
                                }
                            },
                        };

                        this.fan_speed_percent = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '2: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '2: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(fs) => {
                                    let fs = fs as u32;
                                    if fs == u32::MAX {
                                        None
                                    } else {
                                        Some(fs)
                                    }
                                }
                            },
                        };

                        this.util_percent = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '3: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '3: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(up) => {
                                    let up = up as u32;
                                    if up == u32::MAX {
                                        None
                                    } else {
                                        Some(up)
                                    }
                                }
                            },
                        };

                        this.power_draw_watts = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '4: d', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_f64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '4: d', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(pd) => {
                                    if pd == f64::INFINITY {
                                        None
                                    } else {
                                        Some(pd as _)
                                    }
                                }
                            },
                        };

                        this.power_draw_max_watts = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '5: d', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_f64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '5: d', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(pdm) => {
                                    if pdm == f64::INFINITY {
                                        None
                                    } else {
                                        Some(pdm as _)
                                    }
                                }
                            },
                        };

                        this.clock_speed_mhz = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '6: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '6: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(cs) => {
                                    let cs = cs as u32;
                                    if cs == u32::MAX {
                                        None
                                    } else {
                                        Some(cs)
                                    }
                                }
                            },
                        };

                        this.clock_speed_max_mhz = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '7: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '7: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(csm) => NonZero::new(csm as u32),
                            },
                        };

                        this.mem_speed_mhz = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '8: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '8: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(ms) => {
                                    let ms = ms as u32;
                                    if ms == u32::MAX {
                                        None
                                    } else {
                                        Some(ms)
                                    }
                                }
                            },
                        };

                        this.mem_speed_max_mhz = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '9: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '9: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(msm) => NonZero::new(msm as u32),
                            },
                        };

                        this.free_memory = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '10: t', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '10: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(fm) => {
                                    if fm == u64::MAX {
                                        None
                                    } else {
                                        Some(fm)
                                    }
                                }
                            },
                        };

                        this.used_memory = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '11: t', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '11: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(um) => {
                                    if um == u64::MAX {
                                        None
                                    } else {
                                        Some(um)
                                    }
                                }
                            },
                        };

                        this.used_shared_memory = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '12: t', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '12: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(sm) => {
                                    if sm == u64::MAX {
                                        None
                                    } else {
                                        Some(sm)
                                    }
                                }
                            },
                        };

                        this.encoder_percent = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '13: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '13: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(ep) => {
                                    let ep = ep as u32;
                                    if ep == u32::MAX {
                                        None
                                    } else {
                                        Some(ep)
                                    }
                                }
                            },
                        };

                        this.decoder_percent = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '14: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '14: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(dp) => {
                                    let dp = dp as u32;
                                    if dp == u32::MAX {
                                        None
                                    } else {
                                        Some(dp)
                                    }
                                }
                            },
                        };

                        result.push(this);
                    }

                    Some(result.into())
                }
            },
        }
    }
}
