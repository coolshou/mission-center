/* sys_info_v2/gatherer/src/platform/cpu_ext.rs
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

pub mod cpu {
    include!("../../common/cpu.rs");
}

/// Trait that provides an interface for gathering CPU information.
pub trait CpuInfoExt {
    /// Creates a new instance of a struct that implements the `CpuInfo` trait.
    fn new() -> Self;

    /// Returns the static information for the CPU.
    fn static_info(&mut self) -> cpu::StaticInfo;

    /// Returns the dynamic information for the CPU.
    fn dynamic_info(&mut self) -> cpu::DynamicInfo;

    /// Returns dynamic information for each logical CPU present in the system.
    ///
    /// Should be called multiple times until the `cpu::LogicalInfo::is_complete` filed is true.
    fn logical_cpu_info(&mut self) -> cpu::LogicalInfo;
}
