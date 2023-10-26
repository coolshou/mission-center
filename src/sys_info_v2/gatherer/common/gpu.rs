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

use arrayvec::{ArrayString, ArrayVec};

/// Describes the static information of a GPU.
///
/// This struct is used to describe the information about a GPU that does not change during the
/// lifetime of said GPU.
#[derive(Debug, Default, Clone)]
pub struct StaticInfoDescriptor {
    pub pci_id: ArrayString<16>,
    pub device_name: ArrayString<128>,
    pub vendor_id: u16,
    pub device_id: u16,

    pub total_memory: u64,

    pub opengl_version: Option<(u8, u8, bool)>,
    pub vulkan_version: Option<(u16, u16, u16)>,
    pub pcie_gen: u8,
    pub pcie_lanes: u8,
}

/// Describes the dynamic information of a GPU.
///
///  This struct is used to describe the information about a GPU that changes during the lifetime
/// of said GPU.
#[derive(Debug, Default, Clone)]
pub struct DynamicInfoDescriptor {
    pub pci_id: ArrayString<16>,
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
    pub encoder_percent: u32,
    pub decoder_percent: u32,
}

/// The PCI IDs of all GPUs present in the system.
///
/// Since the maximum number of elements is limited the `is_complete` field is used to indicate
/// whether or not all the GPUs have been described. If `is_complete` is `false` then the
/// `id` field contains only a subset of all the PCI IDs of the GPUs present in the system,
/// and the providing function should be called again to get the rest of the IDs.
#[derive(Debug, Default, Clone)]
pub struct PciIds {
    pub ids: ArrayVec<ArrayString<16>, 16>,
    pub is_complete: bool,
}

/// Describes the static information of all GPUs present in the system.
///
/// Since the maximum number of elements is limited the `is_complete` field is used to indicate
/// whether or not all the GPUs have been described. If `is_complete` is `false` then the
/// `desc` field contains only a subset of all the GPUs present in the system, and the providing
/// function should be called again to get the rest of the GPUs.
#[derive(Debug, Default, Clone)]
pub struct StaticInfo {
    pub desc: ArrayVec<StaticInfoDescriptor, 16>,
    pub is_complete: bool,
}

/// Describes the dynamic information of all GPUs present in the system.
///
/// Since the maximum number of elements is limited the `is_complete` field is used to indicate
/// whether or not all the GPUs have been described. If `is_complete` is `false` then the
/// `desc` field contains only a subset of all the GPUs present in the system, and the providing
/// function should be called again to get the rest of the GPUs.
#[derive(Debug, Default, Clone)]
pub struct DynamicInfo {
    pub desc: ArrayVec<DynamicInfoDescriptor, 16>,
    pub is_complete: bool,
}
