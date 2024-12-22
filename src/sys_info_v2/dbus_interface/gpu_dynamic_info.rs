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

use std::sync::Arc;

use dbus::{arg::*, strings::*};
use gtk::glib::g_critical;

use super::deserialize_field;

#[derive(Debug, Clone)]
pub struct GpuDynamicInfo {
    pub id: Arc<str>,
    pub temp_celsius: u32,
    pub fan_speed_percent: u32,
    pub util_percent: u32,
    pub power_draw_watts: f32,
    pub power_draw_max_watts: f32,
    pub clock_speed_mhz: u32,
    pub clock_speed_max_mhz: u32,
    pub mem_speed_mhz: u32,
    pub mem_speed_max_mhz: u32,
    pub free_memory: u64,
    pub used_memory: u64,
    pub used_gtt: u64,
    pub encoder_percent: u32,
    pub decoder_percent: u32,
}

impl Default for GpuDynamicInfo {
    fn default() -> Self {
        Self {
            id: Arc::from(""),
            temp_celsius: 0,
            fan_speed_percent: 0,
            util_percent: 0,
            power_draw_watts: 0.0,
            power_draw_max_watts: 0.0,
            clock_speed_mhz: 0,
            clock_speed_max_mhz: 0,
            mem_speed_mhz: 0,
            mem_speed_max_mhz: 0,
            free_memory: 0,
            used_memory: 0,
            used_gtt: 0,
            encoder_percent: 0,
            decoder_percent: 0,
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
                        let mut this = GpuDynamicInfo {
                            id: Arc::from(""),
                            temp_celsius: 0,
                            fan_speed_percent: 0,
                            util_percent: 0,
                            power_draw_watts: 0.0,
                            power_draw_max_watts: 0.0,
                            clock_speed_mhz: 0,
                            clock_speed_max_mhz: 0,
                            mem_speed_mhz: 0,
                            mem_speed_max_mhz: 0,
                            free_memory: 0,
                            used_memory: 0,
                            used_gtt: 0,
                            encoder_percent: 0,
                            decoder_percent: 0,
                        };

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

                        this.id = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'s' at index 0",
                            |arg| arg.as_str().map(Arc::from),
                        ) {
                            Some(n) => n,
                            None => return None,
                        };

                        this.temp_celsius = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'1: u' at index 1",
                            |arg| arg.as_u64(),
                        ) {
                            Some(tc) => tc as _,
                            None => return None,
                        };

                        this.fan_speed_percent = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'2: u' at index 2",
                            |arg| arg.as_u64(),
                        ) {
                            Some(fs) => fs as _,
                            None => return None,
                        };

                        this.util_percent = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'3: u' at index 3",
                            |arg| arg.as_u64(),
                        ) {
                            Some(up) => up as _,
                            None => return None,
                        };

                        this.power_draw_watts = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'4: d' at index 4",
                            |arg| arg.as_f64(),
                        ) {
                            Some(pd) => pd as _,
                            None => return None,
                        };

                        this.power_draw_max_watts = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'5: d' at index 5",
                            |arg| arg.as_f64(),
                        ) {
                            Some(pdm) => pdm as _,
                            None => return None,
                        };

                        this.clock_speed_mhz = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'6: u' at index 6",
                            |arg| arg.as_u64(),
                        ) {
                            Some(cs) => cs as _,
                            None => return None,
                        };

                        this.clock_speed_max_mhz = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'7: u' at index 7",
                            |arg| arg.as_u64(),
                        ) {
                            Some(csm) => csm as _,
                            None => return None,
                        };

                        this.mem_speed_mhz = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'8: u' at index 8",
                            |arg| arg.as_u64(),
                        ) {
                            Some(ms) => ms as _,
                            None => return None,
                        };

                        this.mem_speed_max_mhz = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'9: u' at index 9",
                            |arg| arg.as_u64(),
                        ) {
                            Some(msm) => msm as _,
                            None => return None,
                        };

                        this.free_memory = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'10: t' at index 10",
                            |arg| arg.as_u64(),
                        ) {
                            Some(fm) => fm as _,
                            None => return None,
                        };

                        this.used_memory = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'11: t' at index 11",
                            |arg| arg.as_u64(),
                        ) {
                            Some(um) => um as _,
                            None => return None,
                        };

                        this.used_gtt = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'12: t' at index 12",
                            |arg| arg.as_u64(),
                        ) {
                            Some(ug) => ug as _,
                            None => return None,
                        };

                        this.encoder_percent = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'13: u' at index 13",
                            |arg| arg.as_u64(),
                        ) {
                            Some(ep) => ep as _,
                            None => return None,
                        };

                        this.decoder_percent = match deserialize_field(
                            dynamic_info,
                            "GpuDynamicInfo",
                            "'14: u' at index 14",
                            |arg| arg.as_u64(),
                        ) {
                            Some(dp) => dp as _,
                            None => return None,
                        };

                        result.push(this);
                    }

                    Some(result.into())
                }
            },
        }
    }
}
