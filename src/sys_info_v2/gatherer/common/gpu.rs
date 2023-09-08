/* sys_info_v2/gatherer/common/gpu.rs
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

use super::{ArrayString, ArrayVec};

#[derive(Debug, Default, Clone)]
pub struct StaticInfoDescriptor {
    pub id: ArrayString,
    pub device_name: ArrayString,
    pub pci_slot_name: ArrayString,
    pub dri_path: ArrayString,
    pub vendor_id: u16,
    pub device_id: u16,

    pub opengl_version: Option<(u8, u8, bool)>,
    pub vulkan_version: Option<(u16, u16, u16)>,
    pub pcie_gen: Option<u8>,
    pub pcie_lanes: Option<u8>,
}

#[derive(Debug, Default, Clone)]
pub struct DynamicInfoDescriptor {
    pub temp_celsius: u32,
    pub fan_speed_percent: u32,
    pub util_percent: u32,
    pub power_draw_watts: f32,
    pub power_draw_max_watts: f32,
    pub clock_speed_mhz: u32,
    pub clock_speed_max_mhz: u32,
    pub mem_speed_mhz: u32,
    pub mem_speed_max_mhz: u32,
    pub total_memory: u64,
    pub free_memory: u64,
    pub used_memory: u64,
    pub encoder_percent: u32,
    pub decoder_percent: u32,
}

#[derive(Debug, Default, Clone)]
pub struct GpuProcess {
    pub index: usize,
    pub pid: u32,
    pub usage: f32,
}

#[derive(Debug, Default, Clone)]
pub struct StaticInfo {
    pub desc: ArrayVec<StaticInfoDescriptor, 16>,
    pub is_complete: bool,
}

#[derive(Debug, Default, Clone)]
pub struct DynamicInfo {
    pub desc: ArrayVec<DynamicInfoDescriptor, 16>,
    pub is_complete: bool,
}

#[derive(Debug, Default, Clone)]
pub struct Processes {
    pub usage: ArrayVec<GpuProcess, 64>,
    pub is_complete: bool,
}
