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

use dbus::arg::{Append, Arg};

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
}

/// The public interface that describes how the list of running processes is obtained
pub trait ProcessesExt<'a>: Append + Arg {
    type P: ProcessExt<'a>;

    /// Refreshes the internal process cache
    ///
    /// It is expected that implementors of this trait cache the process list once obtained from
    /// the underlying OS
    fn refresh_cache(&mut self);

    /// Implementation specific understanding of whether the cache is too old to be relevant
    fn is_cache_stale(&self) -> bool;

    /// Return the (cached) list of processes
    fn process_list(&'a self) -> &'a std::collections::HashMap<u32, Self::P>;
}

impl Arg for crate::platform::Processes {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Array;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("a(sassyuu(ddddd)t)")
    }
}

impl Append for crate::platform::Processes {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append(
            self.process_list()
                .iter()
                .map(|(_, p)| {
                    (
                        p.name(),
                        p.cmd().clone().collect::<Vec<_>>(),
                        p.exe(),
                        p.state() as u8,
                        p.pid(),
                        p.parent(),
                        (
                            p.usage_stats().cpu_usage as f64,
                            p.usage_stats().memory_usage as f64,
                            p.usage_stats().disk_usage as f64,
                            p.usage_stats().network_usage as f64,
                            p.usage_stats().gpu_usage as f64,
                        ),
                        p.task_count() as u64,
                    )
                })
                .collect::<Vec<_>>(),
        );
    }
}
