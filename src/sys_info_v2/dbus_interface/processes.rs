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

use std::sync::Arc;

use dbus::{arg::*, strings::*};

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
    pub disk_usage: f32,
    pub network_usage: f32,
    pub gpu_usage: f32,
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
    pub task_count: usize,
}

impl Arg for Process {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(sassyuu(ddddd)t)")
    }
}

impl<'a> Get<'a> for Process {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let empty_string = Arc::<str>::from("");

        let mut this = Process {
            name: empty_string.clone(),
            cmd: vec![],
            exe: empty_string,
            state: ProcessState::Unknown,
            pid: 0,
            parent: 0,
            usage_stats: Default::default(),
            task_count: 0,
        };

        let process = match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(id) => id,
        };

        let mut process = match process.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return None;
            }
            Some(i) => i,
        };
        let process = process.as_mut();

        this.name = match Iterator::next(process) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '0: s', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Process: Expected '0: s', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(n) => Arc::from(n),
            },
        };

        match Iterator::next(process) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '1: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Process: Expected '1: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(cmds) => {
                    for c in cmds {
                        if let Some(c) = c.as_str() {
                            this.cmd.push(Arc::from(c));
                        }
                    }
                }
            },
        }

        this.exe = match Iterator::next(process) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '3: s', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Process: Expected '3: s', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(e) => Arc::from(e),
            },
        };

        this.state = match Iterator::next(process) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '4: y', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Process: Expected '4: y', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(u) => {
                    if u >= 0 && u < ProcessState::Unknown as u64 {
                        unsafe { core::mem::transmute(u as u8) }
                    } else {
                        ProcessState::Unknown
                    }
                }
            },
        };

        this.pid = match Iterator::next(process) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '5: u', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Process: Expected '5: u', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(p) => p as _,
            },
        };

        this.parent = match Iterator::next(process) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '6: u', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Process: Expected '6: u', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(p) => p as _,
            },
        };

        match Iterator::next(process) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '7: STRUCT', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Process: Expected '7: STRUCT', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(stats) => {
                    let mut values = [0_f32; 5];

                    for (i, v) in stats.enumerate() {
                        values[i] = v.as_f64().unwrap_or(0.) as f32;
                    }

                    this.usage_stats.cpu_usage = values[0];
                    this.usage_stats.memory_usage = values[1];
                    this.usage_stats.disk_usage = values[2];
                    this.usage_stats.network_usage = values[3];
                    this.usage_stats.gpu_usage = values[4];
                }
            },
        };

        this.task_count = match Iterator::next(process) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Process: Expected '14: t', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Process: Expected '14: t', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(tc) => tc as _,
            },
        };

        Some(this)
    }
}
