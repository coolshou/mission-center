/* sys_info_v2/gatherer/src/platform/processes.rs
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

/// State of a running process
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum ProcessState {
    Running,
    Sleeping,
    SleepingUninterruptible,
    Zombie,
    Stopped,
    Tracing,
    Dead,
    WakeKill,
    Waking,
    Parked,
    Unknown,
}

/// Statistics associated with a process
#[derive(Debug, Default, Copy, Clone)]
pub struct ProcessUsageStats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    pub gpu_usage: f32,
}

impl ProcessUsageStats {
    pub fn merge(&mut self, other: &Self) {
        self.cpu_usage += other.cpu_usage;
        self.memory_usage += other.memory_usage;
        self.disk_usage += other.disk_usage;
        self.network_usage += other.network_usage;
        self.gpu_usage += other.gpu_usage;
    }
}

/// High-level description of a process
pub trait ProcessExt<'a> {
    type Iter: Iterator<Item = &'a str>;

    fn name(&self) -> &str;
    fn cmd(&'a self) -> Self::Iter;
    fn exe(&self) -> &str;
    fn state(&self) -> ProcessState;
    fn pid(&self) -> u32;
    fn parent(&self) -> u32;
    fn usage_stats(&self) -> &ProcessUsageStats;
    fn task_count(&self) -> usize;
    fn as_bus_process(&'a self) -> crate::dbus::Process<'a>;
}

/// The public interface that describes how the list of running processes is obtained
pub trait ProcessesExt<'a> {
    type P: ProcessExt<'a>;

    /// Refreshes the internal process cache
    ///
    /// It is expected that implementors of this trait cache the process list once obtained from
    /// the underlying OS
    fn refresh_cache(&mut self);

    /// Return the (cached) list of processes
    fn process_list(&self) -> &std::collections::HashMap<u32, Self::P>;
}
