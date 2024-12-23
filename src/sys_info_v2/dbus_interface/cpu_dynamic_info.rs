/* sys_info_v2/dbus_interface/cpu_dynamic_info.rs
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

use std::sync::Arc;

use dbus::{arg::*, strings::*};

use super::{deser_f32, deser_iter, deser_str, deser_u64};

#[derive(Debug, Default, Clone)]
pub struct CpuDynamicInfo {
    pub overall_utilization_percent: f32,
    pub overall_kernel_utilization_percent: f32,
    pub per_logical_cpu_utilization_percent: Vec<f32>,
    pub per_logical_cpu_kernel_utilization_percent: Vec<f32>,
    pub current_frequency_mhz: u64,
    pub temperature: Option<f32>,
    pub process_count: u64,
    pub thread_count: u64,
    pub handle_count: u64,
    pub uptime_seconds: u64,
    pub cpufreq_driver: Option<Arc<str>>,
    pub cpufreq_governor: Option<Arc<str>>,
    pub energy_performance_preference: Option<Arc<str>>,
}

impl Arg for CpuDynamicInfo {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(ddadadtdtttt)")
    }
}

impl ReadAll for CpuDynamicInfo {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for CpuDynamicInfo {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut this = CpuDynamicInfo {
            overall_utilization_percent: 0.0,
            overall_kernel_utilization_percent: 0.0,
            per_logical_cpu_utilization_percent: vec![],
            per_logical_cpu_kernel_utilization_percent: vec![],
            current_frequency_mhz: 0,
            temperature: None,
            process_count: 0,
            thread_count: 0,
            handle_count: 0,
            uptime_seconds: 0,
            cpufreq_driver: None,
            cpufreq_governor: None,
            energy_performance_preference: None,
        };

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

        this.overall_utilization_percent =
            match deser_f32(dynamic_info, "CpuDynamicInfo", "'d' at index 0") {
                Some(u) => u,
                None => return None,
            };

        this.overall_kernel_utilization_percent =
            match deser_f32(dynamic_info, "CpuDynamicInfo", "'d' at index 1") {
                Some(u) => u,
                None => return None,
            };

        match deser_iter(dynamic_info, "CpuDynamicInfo", "ARRAY at index 2") {
            Some(iter) => {
                for v in iter {
                    this.per_logical_cpu_utilization_percent
                        .push(v.as_f64().unwrap_or(0.) as f32);
                }
            }
            None => return None,
        }

        match deser_iter(dynamic_info, "CpuDynamicInfo", "ARRAY at index 4") {
            Some(iter) => {
                for v in iter {
                    this.per_logical_cpu_kernel_utilization_percent
                        .push(v.as_f64().unwrap_or(0.) as f32);
                }
            }
            None => return None,
        }

        this.current_frequency_mhz =
            match deser_u64(dynamic_info, "CpuDynamicInfo", "'t' at index 6") {
                Some(u) => u,
                None => return None,
            };

        this.temperature = match deser_f32(dynamic_info, "CpuDynamicInfo", "'d' at index 7") {
            Some(u) => {
                if u == 0. {
                    None
                } else {
                    Some(u)
                }
            }
            None => return None,
        };

        this.process_count = match deser_u64(dynamic_info, "CpuDynamicInfo", "'t' at index 8") {
            Some(u) => u,
            None => return None,
        };

        this.thread_count = match deser_u64(dynamic_info, "CpuDynamicInfo", "'t' at index 9") {
            Some(u) => u,
            None => return None,
        };

        this.handle_count = match deser_u64(dynamic_info, "CpuDynamicInfo", "'t' at index 10") {
            Some(u) => u,
            None => return None,
        };

        this.uptime_seconds = match deser_u64(dynamic_info, "CpuDynamicInfo", "'t' at index 11") {
            Some(u) => u,
            None => return None,
        };

        this.cpufreq_driver = match deser_str(dynamic_info, "CpuDynamicInfo", "'s' at index 12") {
            Some(s) => {
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            }
            None => return None,
        };

        this.cpufreq_governor = match deser_str(dynamic_info, "CpuDynamicInfo", "'s' at index 13") {
            Some(s) => {
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            }
            None => return None,
        };

        this.energy_performance_preference =
            match deser_str(dynamic_info, "CpuDynamicInfo", "'s' at index 14") {
                Some(s) => {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                }
                None => return None,
            };

        Some(this)
    }
}
