/* sys_info_v2/dbus_interface/disk_static_info.rs
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

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DiskType {
    Unknown = 0,
    HDD,
    SSD,
    NVMe,
    eMMC,
    iSCSI,
    Optical,
}

impl Default for DiskType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DiskStaticInfo {
    pub id: Arc<str>,
    pub model: Arc<str>,
    pub r#type: DiskType,
    pub capacity: u64,
    pub formatted: u64,
    pub system_disk: bool,
}

impl Default for DiskStaticInfo {
    fn default() -> Self {
        Self {
            id: Arc::from(""),
            model: Arc::from(""),
            r#type: DiskType::default(),
            capacity: 0,
            formatted: 0,
            system_disk: false,
        }
    }
}

pub struct DiskStaticInfoVec(pub Vec<DiskStaticInfo>);

impl From<DiskStaticInfoVec> for Vec<DiskStaticInfo> {
    fn from(v: DiskStaticInfoVec) -> Self {
        v.0
    }
}

impl Arg for DiskStaticInfoVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("a(ssyttb)")
    }
}

impl<'a> Get<'a> for DiskStaticInfoVec {
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
                        let mut this = DiskStaticInfo::default();

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
                                    "Failed to get DiskStaticInfo: Expected '0: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskStaticInfo: Expected '0: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(n) => Arc::<str>::from(n),
                            },
                        };

                        this.model = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskStaticInfo: Expected '1: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskStaticInfo: Expected '1: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(m) => Arc::<str>::from(m),
                            },
                        };

                        this.r#type = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskStaticInfo: Expected '2: y', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskStaticInfo: Expected '2: y', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(t) => match t {
                                    1 => DiskType::HDD,
                                    2 => DiskType::SSD,
                                    3 => DiskType::NVMe,
                                    4 => DiskType::eMMC,
                                    5 => DiskType::iSCSI,
                                    6 => DiskType::Optical,
                                    _ => DiskType::Unknown,
                                },
                            },
                        };

                        this.capacity = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskStaticInfo: Expected '3: t', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskStaticInfo: Expected '3: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(c) => c,
                            },
                        };

                        this.formatted = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskStaticInfo: Expected '4: t', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskStaticInfo: Expected '4: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(f) => f,
                            },
                        };

                        this.system_disk = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskStaticInfo: Expected '5: b', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskStaticInfo: Expected '5: b', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(ivm) => match ivm {
                                    1 => true,
                                    _ => false,
                                },
                            },
                        };
                        result.push(this);
                    }
                }
            },
        }

        Some(DiskStaticInfoVec(result))
    }
}
