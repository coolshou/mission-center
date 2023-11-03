/* sys_info_v2/gatherer/src/platform/apps.rs
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

use dbus::{arg::*, strings::*};
use std::sync::Arc;

pub type AppUsageStats = super::processes::ProcessUsageStats;

#[derive(Debug, Clone)]
pub struct App {
    name: Arc<str>,
    icon: Option<Arc<str>>,
    id: Arc<str>,
    command: Arc<str>,
    pids: Vec<u32>,
    usage_stats: AppUsageStats,
}

impl Arg for App {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(ssssau(ddddd))")
    }
}

impl<'a> Get<'a> for App {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let empty_string = Arc::<str>::from("");

        let mut this = App {
            name: empty_string.clone(),
            icon: None,
            id: empty_string.clone(),
            command: empty_string,
            pids: vec![],
            usage_stats: Default::default(),
        };

        let app = match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get App: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(id) => id,
        };

        let mut app = match app.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get App: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return None;
            }
            Some(i) => i,
        };
        let app = app.as_mut();

        this.name = match Iterator::next(app) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get App: Expected '0: s', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get App: Expected '0: s', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(n) => Arc::from(n),
            },
        };

        this.icon = match Iterator::next(app) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get App: Expected '1: s', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get App: Expected '1: s', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(icon) => {
                    if icon.is_empty() {
                        None
                    } else {
                        Some(Arc::from(icon))
                    }
                }
            },
        };

        this.id = match Iterator::next(app) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get App: Expected '2: s', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get App: Expected '2: s', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(id) => Arc::from(id),
            },
        };

        this.command = match Iterator::next(app) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get App: Expected '3: 2', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get App: Expected '3: s', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(c) => Arc::from(c),
            },
        };

        match Iterator::next(app) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get App: Expected '4: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get App: Expected '4: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(pids) => {
                    for p in pids {
                        if let Some(p) = p.as_u64() {
                            this.pids.push(p as u32);
                        }
                    }
                }
            },
        }

        match Iterator::next(app) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get App: Expected '6: STRUCT', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get App: Expected '6: STRUCT', got {:?}",
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

        Some(this)
    }
}
