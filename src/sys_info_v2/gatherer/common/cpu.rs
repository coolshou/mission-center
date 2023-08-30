/* sys_info_v2/gatherer/common/cpu.rs
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
pub struct StaticInfo {
    pub name: ArrayString,
    pub logical_cpu_count: u32,
    pub socket_count: Option<u8>,
    pub base_frequency_khz: Option<u64>,
    pub virtualization: Option<bool>,
    pub virtual_machine: Option<bool>,
    pub l1_cache: Option<usize>,
    pub l2_cache: Option<usize>,
    pub l3_cache: Option<usize>,
    pub l4_cache: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct DynamicInfo {
    pub overall_utilization_percent: f32,
    pub current_frequency_mhz: u64,
    pub temperature: Option<f32>,
    pub process_count: u32,
    pub thread_count: u32,
    pub handle_count: u32,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct LogicalCpuInfo {
    pub utilization_percent: ArrayVec<f32, 128>,
    pub is_complete: bool,
}
