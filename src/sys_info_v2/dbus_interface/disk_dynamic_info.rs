/* sys_info_v2/dbus_interface/disk_dynamic_info.rs
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

use std::sync::Arc;

use dbus::{arg::*, strings::*};

#[derive(Debug, Clone, PartialEq)]
pub struct DiskDynamicInfo {
    pub id: Arc<str>,
    pub busy_percent: f32,
    pub response_time_ms: f32,
    pub read_speed: u64,
    pub write_speed: u64,
}

impl Default for DiskDynamicInfo {
    fn default() -> Self {
        Self {
            id: Arc::from(""),
            busy_percent: 0.0,
            response_time_ms: 0.0,
            read_speed: 0,
            write_speed: 0,
        }
    }
}

pub struct DiskDynamicInfoVec(pub Vec<DiskDynamicInfo>);

impl From<DiskDynamicInfoVec> for Vec<DiskDynamicInfo> {
    fn from(v: DiskDynamicInfoVec) -> Self {
        v.0
    }
}

impl Arg for DiskDynamicInfoVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("a(sddtt)")
    }
}

impl<'a> Get<'a> for DiskDynamicInfoVec {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut result = vec![];

        match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Vec<DiskDynamicInfo>: Expected '0: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Vec<DiskDynamicInfo>: Expected '0: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    for i in arr {
                        let mut this = DiskDynamicInfo::default();

                        let mut i = match i.as_iter() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskDynamicInfo: Expected '0: STRUCT', got None",
                                );
                                continue;
                            }
                            Some(i) => i,
                        };
                        let disk_info = i.as_mut();

                        this.id = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskDynamicInfo: Expected '0: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskDynamicInfo: Expected '0: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(id) => Arc::from(id),
                            },
                        };

                        this.busy_percent = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskDynamicInfo: Expected '0: d', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_f64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskDynamicInfo: Expected '0: d', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(bp) => bp as _,
                            },
                        };

                        this.response_time_ms = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskDynamicInfo: Expected '1: d', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_f64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskDynamicInfo: Expected '1: d', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(rt) => rt as _,
                            },
                        };

                        this.read_speed = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskDynamicInfo: Expected '2: t', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskDynamicInfo: Expected '2: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(rs) => rs,
                            },
                        };

                        this.write_speed = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskDynamicInfo: Expected '3: t', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskDynamicInfo: Expected '3: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(ws) => ws,
                            },
                        };
                        result.push(this);
                    }
                }
            },
        }

        Some(DiskDynamicInfoVec(result))
    }
}
