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

use std::{collections::HashMap, sync::Arc};

use dbus::{arg::*, strings::*};
use gtk::glib::g_critical;

use super::{deser_array, deser_str, deser_struct, deser_u32, deser_u64, deser_usize};

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum ProcessState {
    Running = 0,
    Sleeping = 1,
    SleepingUninterruptible = 2,
    Zombie = 3,
    Stopped = 4,
    Tracing = 5,
    Dead = 6,
    WakeKill = 7,
    Waking = 8,
    Parked = 9,
    Unknown = 10, // Keep this last and increase it
}

#[derive(Debug, Default, Copy, Clone)]
pub struct ProcessUsageStats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub memory_shared: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    pub gpu_usage: f32,
    pub gpu_memory_usage: f32,
}

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

#[derive(Debug, Clone)]
pub struct Process {
    pub name: Arc<str>,
    pub cmd: Vec<Arc<str>>,
    pub exe: Arc<str>,
    pub state: ProcessState,
    pub pid: u32,
    pub parent: u32,
    pub usage_stats: ProcessUsageStats,
    pub merged_usage_stats: ProcessUsageStats,
    pub task_count: usize,
    pub children: HashMap<u32, Process>,
}

impl Default for Process {
    fn default() -> Self {
        let empty_string = Arc::<str>::from("");

        Self {
            name: empty_string.clone(),
            cmd: vec![],
            exe: empty_string,
            state: ProcessState::Unknown,
            pid: 0,
            parent: 0,
            usage_stats: Default::default(),
            merged_usage_stats: Default::default(),
            task_count: 0,
            children: HashMap::new(),
        }
    }
}

impl From<&dyn RefArg> for Process {
    fn from(value: &dyn RefArg) -> Self {
        let mut this = Self::default();

        let mut process = match value.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected 'STRUCT', got None, failed to iterate over fields",
                );
                return this;
            }
            Some(i) => i,
        };

        let process = process.as_mut();

        this.name = match deser_str(process, "Process", 0) {
            Some(name) => name,
            None => return this,
        };

        if let Some(cmds) = deser_array(process, "Process", 1) {
            for c in cmds {
                if let Some(c) = c.as_str() {
                    this.cmd.push(Arc::from(c));
                }
            }
        } else {
            return this;
        }

        this.exe = match deser_str(process, "Process", 3) {
            Some(exe) => exe,
            None => return this,
        };

        this.state = match deser_u64(process, "Process", 4) {
            Some(u) => {
                if u < ProcessState::Unknown as u64 {
                    unsafe { core::mem::transmute(u as u8) }
                } else {
                    ProcessState::Unknown
                }
            }
            None => return this,
        };

        this.pid = match deser_u32(process, "Process", 5) {
            Some(p) => p,
            None => return this,
        };

        this.parent = match deser_u32(process, "Process", 6) {
            Some(p) => p,
            None => return this,
        };

        match deser_struct(process, "Process", 7) {
            Some(arg) => {
                let mut values = [0_f32; 7];

                for (i, v) in arg.enumerate() {
                    values[i] = v.as_f64().unwrap_or(0.) as f32;
                }

                this.usage_stats.cpu_usage = values[0];
                this.usage_stats.memory_usage = values[1];
                this.usage_stats.memory_shared = values[2];
                this.usage_stats.disk_usage = values[3];
                this.usage_stats.network_usage = values[4];
                this.usage_stats.gpu_usage = values[5];
                this.usage_stats.gpu_memory_usage = values[6];

                this.merged_usage_stats = this.usage_stats;
            }
            None => return this,
        };

        this.task_count = match deser_usize(process, "Process", 14) {
            Some(tc) => tc,
            None => return this,
        };

        this
    }
}

pub struct ProcessMap(HashMap<u32, Process>);

impl From<HashMap<u32, Process>> for ProcessMap {
    fn from(value: HashMap<u32, Process>) -> Self {
        Self(value)
    }
}

impl From<ProcessMap> for HashMap<u32, Process> {
    fn from(value: ProcessMap) -> Self {
        value.0
    }
}

impl Arg for ProcessMap {
    const ARG_TYPE: ArgType = ArgType::Array;

    fn signature() -> Signature<'static> {
        Signature::from("a(sassyuu(dddddd)t)")
    }
}

impl ReadAll for ProcessMap {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for ProcessMap {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut this = HashMap::new();

        match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get HashMap<Pid, Process>: Expected '0: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get HashMap<Pid, Process>: Expected '0: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    for p in arr {
                        let p = Process::from(p);
                        if p.pid == 0 {
                            continue;
                        }
                        this.insert(p.pid, p.clone());
                    }
                }
            },
        }

        Some(this.into())
    }
}
