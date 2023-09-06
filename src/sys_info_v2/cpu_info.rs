/* sys_info_v2/cpu_info.rs
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

use super::GathererSupervisor;

pub type StaticInfo = super::gatherer::CpuStaticInfo;

impl GathererSupervisor {
    pub fn cpu_static_info(&mut self) -> StaticInfo {
        use super::gatherer::SharedDataContent;
        use gtk::glib::*;

        let mut result = Default::default();

        self.execute(
            super::gatherer::Message::GetCpuStaticInfo,
            |gatherer, _| {
                let shared_memory = match gatherer.shared_memory() {
                    Ok(shm) => shm,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::CpuInfo",
                            "Unable to to access shared memory: {}",
                            e
                        );
                        return false;
                    }
                };

                match shared_memory.content {
                    SharedDataContent::CpuStaticInfo(ref static_info) => {
                        result = static_info.clone();
                        true
                    }
                    _ => {
                        g_critical!(
                            "MissionCenter::CpuInfo",
                            "Shared data content is {:?} instead of CpuStaticInfo; encountered when reading installed apps from gatherer",
                            shared_memory.content
                        );
                        false
                    }
                }
            },
        );

        result
    }
}

#[derive(Debug, Default, Clone)]
pub struct DynamicInfo {
    pub utilization_percent: f32,
    pub kernel_utilization_percent: f32,
    pub utilization_percent_per_core: Vec<f32>,
    pub kernel_utilization_percent_per_core: Vec<f32>,
    pub current_frequency_mhz: u64,
    pub temperature: Option<f32>,
    pub process_count: u32,
    pub thread_count: u32,
    pub handle_count: u32,
    pub uptime_seconds: u64,
}

impl GathererSupervisor {
    pub fn cpu_dynamic_info(&mut self) -> DynamicInfo {
        use super::gatherer::SharedDataContent;
        use gtk::glib::*;

        let mut result = DynamicInfo::default();

        self.execute(
            super::gatherer::Message::GetCpuDynamicInfo,
            |gatherer, _| {
                let shared_memory = match gatherer.shared_memory() {
                    Ok(shm) => shm,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::CpuInfo",
                            "Unable to to access shared memory: {}",
                            e
                        );
                        return false;
                    }
                };

                match shared_memory.content {
                    SharedDataContent::CpuDynamicInfo(ref dynamic_info) => {
                        result.utilization_percent = dynamic_info.utilization_percent;
                        result.kernel_utilization_percent = dynamic_info.kernel_utilization_percent;
                        result.current_frequency_mhz = dynamic_info.current_frequency_mhz;
                        result.temperature = dynamic_info.temperature;
                        result.process_count = dynamic_info.process_count;
                        result.thread_count = dynamic_info.thread_count;
                        result.handle_count = dynamic_info.handle_count;
                        result.uptime_seconds = dynamic_info.uptime_seconds;

                        true
                    }
                    _ => {
                        g_critical!(
                            "MissionCenter::CpuInfo",
                            "Shared data content is {:?} instead of CpuDynamicInfo; encountered when reading installed apps from gatherer",
                            shared_memory.content
                        );
                        false
                    }
                }
            },
        );

        result.utilization_percent_per_core.clear();
        result.kernel_utilization_percent_per_core.clear();
        self.execute(
            super::gatherer::Message::GetLogicalCpuInfo,
            |gatherer, _| {
                let shared_memory = match gatherer.shared_memory() {
                    Ok(shm) => shm,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::CpuInfo",
                            "Unable to to access shared memory: {}",
                            e
                        );
                        return false;
                    }
                };

                match shared_memory.content {
                    SharedDataContent::LogicalCpuInfo(ref logical_cpu_info) => {
                        result.utilization_percent_per_core.extend_from_slice(logical_cpu_info.utilization_percent.as_slice());
                        result.kernel_utilization_percent_per_core.extend_from_slice(logical_cpu_info.kernel_utilization_percent.as_slice());
                        return logical_cpu_info.is_complete;
                    }
                    _ => {
                        g_critical!(
                            "MissionCenter::CpuInfo",
                            "Shared data content is {:?} instead of CpuDynamicInfo; encountered when reading installed apps from gatherer",
                            shared_memory.content
                        );
                        false
                    }
                }
            },
        );
        result
            .utilization_percent_per_core
            .resize(num_cpus::get(), 0.);
        result
            .kernel_utilization_percent_per_core
            .resize(num_cpus::get(), 0.);

        result
    }
}
