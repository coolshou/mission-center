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
