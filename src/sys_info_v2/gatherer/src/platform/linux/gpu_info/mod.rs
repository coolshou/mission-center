/* sys_info_v2/gatherer/src/platform/linux/gpu_info/mod.rs
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

use super::gpu;

mod nvtop;
mod vulkan_info;

pub struct GpuInfo {}

impl super::GpuInfoExt for GpuInfo {
    fn new() -> Self {
        Self {}
    }

    fn static_info(&mut self) -> gpu::StaticInfo {
        gpu::StaticInfo::default()
    }

    fn dynamic_info(&mut self) -> gpu::DynamicInfo {
        gpu::DynamicInfo::default()
    }

    fn processes(&mut self) -> gpu::Processes {
        gpu::Processes::default()
    }
}
