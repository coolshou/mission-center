/* sys_info_v2/gpu_info.rs
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

use std::rc::Rc;

use super::GathererSupervisor;

pub type StaticInfo = super::gatherer::GpuStaticInfoDescriptor;

#[derive(Debug, Clone)]
pub struct DynamicInfo {
    desc: super::gatherer::GpuDynamicInfoDescriptor,
}

impl DynamicInfo {
    fn new(desc: super::gatherer::GpuDynamicInfoDescriptor) -> Self {
        Self { desc }
    }

    #[inline]
    pub fn pci_id(&self) -> &str {
        self.desc.pci_id.as_str()
    }

    #[inline]
    pub fn temp_celsius(&self) -> u32 {
        self.desc.temp_celsius
    }

    #[inline]
    pub fn fan_speed_percent(&self) -> u32 {
        self.desc.fan_speed_percent
    }

    #[inline]
    pub fn util_percent(&self) -> u32 {
        self.desc.util_percent
    }

    #[inline]
    pub fn power_draw_watts(&self) -> f32 {
        self.desc.power_draw_watts
    }

    #[inline]
    pub fn power_draw_max_watts(&self) -> f32 {
        self.desc.power_draw_max_watts
    }

    #[inline]
    pub fn clock_speed_mhz(&self) -> u32 {
        self.desc.clock_speed_mhz
    }

    #[inline]
    pub fn clock_speed_max_mhz(&self) -> u32 {
        self.desc.clock_speed_max_mhz
    }

    #[inline]
    pub fn mem_speed_mhz(&self) -> u32 {
        self.desc.mem_speed_mhz
    }

    #[inline]
    pub fn mem_speed_max_mhz(&self) -> u32 {
        self.desc.mem_speed_max_mhz
    }

    #[inline]
    pub fn free_memory(&self) -> u64 {
        self.desc.free_memory
    }

    #[inline]
    pub fn used_memory(&self) -> u64 {
        self.desc.used_memory
    }

    #[inline]
    pub fn encoder_percent(&self) -> u32 {
        self.desc.encoder_percent
    }

    #[inline]
    pub fn decoder_percent(&self) -> u32 {
        self.desc.decoder_percent
    }
}

impl GathererSupervisor {
    pub fn enumerate_gpus(&mut self) -> Vec<Rc<str>> {
        use super::gatherer::SharedDataContent;
        use gtk::glib::*;

        let mut result = vec![];

        self.execute(
            super::gatherer::Message::EnumerateGpus,
            |gatherer, _| {
                let shared_memory = match gatherer.shared_memory() {
                    Ok(shm) => shm,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::GpuInfo",
                            "Unable to to access shared memory: {}",
                            e
                        );
                        return false;
                    }
                };

                match shared_memory.content {
                    SharedDataContent::GpuPciIds(ref pci_ids) => {
                        for pci_id in &pci_ids.ids {
                            result.push(Rc::from(pci_id.as_str()));
                        }
                        return pci_ids.is_complete;
                    }
                    _ => {
                        g_critical!(
                            "MissionCenter::GpuInfo",
                            "Shared data content is {:?} instead of GpuPciIds; encountered when enumerating GPUs",
                            shared_memory.content
                        );
                        false
                    }
                }
            },
        );

        result
    }

    pub fn gpu_static_info(&mut self) -> Vec<StaticInfo> {
        use super::gatherer::SharedDataContent;
        use gtk::glib::*;

        let mut result = vec![];

        self.execute(
            super::gatherer::Message::GetGpuStaticInfo,
            |gatherer, _| {
                let shared_memory = match gatherer.shared_memory() {
                    Ok(shm) => shm,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::GpuInfo",
                            "Unable to to access shared memory: {}",
                            e
                        );
                        return false;
                    }
                };

                match shared_memory.content {
                    SharedDataContent::GpuStaticInfo(ref static_info) => {
                        for static_info in &static_info.desc {
                            result.push(static_info.clone());
                        }
                        return static_info.is_complete;
                    }
                    _ => {
                        g_critical!(
                            "MissionCenter::GpuInfo",
                            "Shared data content is {:?} instead of GpuStaticInfo; encountered when reading response from gatherer",
                            shared_memory.content
                        );
                        false
                    }
                }
            },
        );

        result
    }

    pub fn gpu_dynamic_info(&mut self) -> Vec<DynamicInfo> {
        use super::gatherer::SharedDataContent;
        use gtk::glib::*;

        let mut result = vec![];

        self.execute(
            super::gatherer::Message::GetGpuDynamicInfo,
            |gatherer, _| {
                let shared_memory = match gatherer.shared_memory() {
                    Ok(shm) => shm,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::GpuInfo",
                            "Unable to to access shared memory: {}",
                            e
                        );
                        return false;
                    }
                };

                match shared_memory.content {
                    SharedDataContent::GpuDynamicInfo(ref dynamic_info) => {
                        for dynamic_info in &dynamic_info.desc {
                            result.push(DynamicInfo::new(dynamic_info.clone()));
                        }
                        return dynamic_info.is_complete;
                    }
                    _ => {
                        g_critical!(
                            "MissionCenter::GpuInfo",
                            "Shared data content is {:?} instead of GpuDynamicInfo; encountered when reading response from gatherer",
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
