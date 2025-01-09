/* sys_info_v2/dbus_interface/processes.rs
 *
 * Copyright 2024 Romeo Calota
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
use std::collections::HashMap;

pub type ProcessState = crate::sys_info_v2::gatherer::processes::ProcessState;
pub type ProcessUsageStats = crate::sys_info_v2::gatherer::processes::ProcessUsageStats;

impl ProcessUsageStats {
    pub fn merge(&mut self, other: &Self) {
        self.cpu_usage += other.cpu_usage;
        self.memory_usage += other.memory_usage;
        self.disk_usage += other.disk_usage;
        self.network_usage += other.network_usage;
        self.gpu_usage += other.gpu_usage;
        self.gpu_memory_usage += other.gpu_memory_usage;
    }
}

pub type Process = crate::sys_info_v2::gatherer::processes::Process;

impl Process {
    pub fn merged_usage_stats(&self, processes: &HashMap<u32, Process>) -> ProcessUsageStats {
        let mut usage_stats = ProcessUsageStats::default();
        for child_pid in &self.children {
            if let Some(child) = processes.get(child_pid) {
                let child_usage_stats = child.merged_usage_stats(processes);
                usage_stats.merge(&child_usage_stats);
            }
        }
        usage_stats.merge(&self.usage_stats.unwrap_or_default());
        usage_stats
    }
}
