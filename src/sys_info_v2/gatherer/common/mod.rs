/* sys_info_v2/gatherer/common/mod.rs
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

pub use arrayvec::ArrayVec;

pub use apps::{AppDescriptor, AppPIDs, Apps};
pub use exit_code::ExitCode;
pub use processes::{ProcessDescriptor, ProcessState, Processes};
pub use shared_data::{SharedData, SharedDataContent};
pub use util::{to_binary, to_binary_mut};

pub mod ipc;

mod apps;
mod cpu;
mod exit_code;
mod gpu;
mod processes;
mod shared_data;
mod util;

pub type ArrayString = arrayvec::ArrayString<256>;
pub type ProcessStats = processes::Stats;
pub type AppStats = apps::Stats;
pub type CpuStaticInfo = cpu::StaticInfo;
pub type CpuDynamicInfo = cpu::DynamicInfo;
pub type LogicalCpuInfo = cpu::LogicalInfo;
pub type GpuPciIds = gpu::PciIds;
pub type GpuStaticInfo = gpu::StaticInfo;
pub type GpuDynamicInfo = gpu::DynamicInfo;
pub type GpuProcesses = gpu::Processes;
