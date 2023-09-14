/* sys_info_v2/gatherer/common/shared_data.rs
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

use super::*;

#[allow(dead_code)]
pub enum SharedDataContent {
    Monostate,
    Processes(Processes),
    Apps(Apps),
    AppPIDs(AppPIDs),
    CpuStaticInfo(CpuStaticInfo),
    CpuDynamicInfo(CpuDynamicInfo),
    LogicalCpuInfo(LogicalCpuInfo),
    GpuPciIds(GpuPciIds),
    GpuStaticInfo(GpuStaticInfo),
    GpuDynamicInfo(GpuDynamicInfo),
    GpuProcesses(GpuProcesses),
}

impl std::fmt::Debug for SharedDataContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            SharedDataContent::Monostate => f.write_str("Monostate"),
            SharedDataContent::Processes(_) => f.write_str("Processes"),
            SharedDataContent::Apps(_) => f.write_str("Apps"),
            SharedDataContent::AppPIDs(_) => f.write_str("AppPIDs"),
            SharedDataContent::CpuStaticInfo(_) => f.write_str("CpuStaticInfo"),
            SharedDataContent::CpuDynamicInfo(_) => f.write_str("CpuDynamicInfo"),
            SharedDataContent::LogicalCpuInfo(_) => f.write_str("LogicalCpuInfo"),
            SharedDataContent::GpuPciIds(_) => f.write_str("GpuPciIds"),
            SharedDataContent::GpuStaticInfo(_) => f.write_str("GpuStaticInfo"),
            SharedDataContent::GpuDynamicInfo(_) => f.write_str("GpuDynamicInfo"),
            SharedDataContent::GpuProcesses(_) => f.write_str("GpuProcesses"),
        }
    }
}

#[derive(Debug)]
pub struct SharedData {
    pub content: SharedDataContent,
}

#[allow(dead_code)]
impl SharedData {
    pub fn clear(&mut self) {
        self.content = SharedDataContent::Monostate;
    }
}
