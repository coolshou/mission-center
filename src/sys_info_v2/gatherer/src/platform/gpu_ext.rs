/* sys_info_v2/gatherer/src/platform/gpu_trait.rs
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

pub mod gpu {
    include!("../../common/gpu.rs");
}

/// Trait that provides an interface for gathering GPU information.
pub trait GpuInfoExt {
    /// Creates a new instance of a struct that implements the `GpuInfo` trait.
    fn new() -> Self;

    fn enumerate(&mut self) -> gpu::PciIds;

    /// Returns the static information for all GPUs present in the system.
    ///
    /// Should be called multiple times until the `GpuStaticInfo::is_complete` filed is true.
    fn static_info(&mut self) -> gpu::StaticInfo;

    /// Returns the dynamic information for all GPUs present in the system.
    ///
    /// Should be called multiple times until the `GpuDynamicInfo::is_complete` filed is true.
    fn dynamic_info(&mut self) -> gpu::DynamicInfo;

    /// Returns the processes that are currently using the GPUs in the system.
    ///
    /// Should be called multiple times until the `GpuProcesses::is_complete` filed is true.
    fn processes(&mut self) -> gpu::Processes;
}
