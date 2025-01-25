/* sys_info_v2/dbus_interface/disk_info.rs
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

use super::{deser_bool, deser_f32, deser_str, deser_u32, deser_u64, deser_u8};

#[allow(non_camel_case_types)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DiskType {
    #[default]
    Unknown = 0,
    HDD,
    SSD,
    NVMe,
    eMMC,
    SD,
    Floppy,
    Optical,
}

impl TryFrom<u8> for DiskType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(DiskType::Unknown),
            1 => Ok(DiskType::HDD),
            2 => Ok(DiskType::SSD),
            3 => Ok(DiskType::NVMe),
            4 => Ok(DiskType::eMMC),
            5 => Ok(DiskType::SD),
            6 => Ok(DiskType::Floppy),
            7 => Ok(DiskType::Optical),
            _ => Err(format!("Unknown DiskType value: {}", value)),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DiskSmartInterface {
    #[default]
    Dumb = 0,
    Ata,
    NVMe,
}

impl TryFrom<u8> for DiskSmartInterface {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(DiskSmartInterface::Dumb),
            1 => Ok(DiskSmartInterface::Ata),
            2 => Ok(DiskSmartInterface::NVMe),
            _ => Err(format!("Unknown Smart Interface value: {}", value)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub id: Arc<str>,
    pub model: Arc<str>,
    pub r#type: DiskType,
    pub smart_interface: DiskSmartInterface,
    pub capacity: u64,
    pub formatted: u64,
    pub system_disk: bool,

    pub busy_percent: f32,
    pub response_time_ms: f32,
    pub read_speed: u64,
    pub total_read: u64,
    pub write_speed: u64,
    pub total_write: u64,
    pub ejectable: bool,
    pub drive_temperature: u32,
}

impl Default for DiskInfo {
    fn default() -> Self {
        Self {
            id: Arc::from(""),
            model: Arc::from(""),
            r#type: DiskType::default(),
            smart_interface: DiskSmartInterface::default(),
            capacity: 0,
            formatted: 0,
            system_disk: false,

            busy_percent: 0.,
            response_time_ms: 0.,
            read_speed: 0,
            total_read: 0,
            write_speed: 0,
            total_write: 0,
            ejectable: false,

            drive_temperature: 0,
        }
    }
}

impl Eq for DiskInfo {}

impl PartialEq<Self> for DiskInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id.as_ref() == other.id.as_ref()
    }
}

impl PartialOrd<Self> for DiskInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.id.as_ref().cmp(other.id.as_ref()))
    }
}

impl Ord for DiskInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.as_ref().cmp(other.id.as_ref())
    }
}

pub struct DiskInfoVec(pub Vec<DiskInfo>);

impl From<DiskInfoVec> for Vec<DiskInfo> {
    fn from(v: DiskInfoVec) -> Self {
        v.0
    }
}

impl Arg for DiskInfoVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("a(ssyyttbddttttbu)")
    }
}

impl ReadAll for DiskInfoVec {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for DiskInfoVec {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut result = vec![];

        match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Vec<DiskInfo>: Expected '0: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Vec<DiskInfo>: Expected '0: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    for i in arr {
                        let mut this = DiskInfo::default();

                        let mut i = match i.as_iter() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '0: STRUCT', got None",
                                );
                                continue;
                            }
                            Some(i) => i,
                        };
                        let disk_info = i.as_mut();

                        this.id = match deser_str(disk_info, "DiskInfo", 0) {
                            Some(n) => n,
                            None => continue,
                        };

                        this.model = match deser_str(disk_info, "DiskInfo", 1) {
                            Some(m) => m,
                            None => continue,
                        };

                        this.r#type = match deser_u8(disk_info, "DiskInfo", 2) {
                            Some(v) => v.try_into().unwrap_or_default(),
                            None => continue,
                        };

                        this.smart_interface = match deser_u8(disk_info, "DiskInfo", 3) {
                            Some(i) => i.try_into().unwrap_or_default(),
                            None => continue,
                        };

                        this.capacity = match deser_u64(disk_info, "DiskInfo", 4) {
                            Some(c) => c,
                            None => continue,
                        };

                        this.formatted = match deser_u64(disk_info, "DiskInfo", 5) {
                            Some(f) => f,
                            None => continue,
                        };

                        this.system_disk = match deser_bool(disk_info, "DiskInfo", 6) {
                            Some(sd) => sd,
                            None => continue,
                        };

                        this.busy_percent = match deser_f32(disk_info, "DiskInfo", 7) {
                            Some(u) => u,
                            None => continue,
                        };

                        this.response_time_ms = match deser_f32(disk_info, "DiskInfo", 8) {
                            Some(u) => u,
                            None => continue,
                        };

                        this.read_speed = match deser_u64(disk_info, "DiskInfo", 9) {
                            Some(rs) => rs,
                            None => continue,
                        };

                        this.total_read = match deser_u64(disk_info, "DiskInfo", 10) {
                            Some(tr) => tr,
                            None => continue,
                        };

                        this.write_speed = match deser_u64(disk_info, "DiskInfo", 11) {
                            Some(ws) => ws,
                            None => continue,
                        };

                        this.total_write = match deser_u64(disk_info, "DiskInfo", 12) {
                            Some(tw) => tw,
                            None => continue,
                        };

                        this.ejectable = match deser_bool(disk_info, "DiskInfo", 13) {
                            Some(sd) => sd,
                            None => continue,
                        };

                        this.drive_temperature = match deser_u32(disk_info, "DiskInfo", 14) {
                            Some(sd) => sd,
                            None => continue,
                        };

                        result.push(this);
                    }
                }
            },
        }

        Some(DiskInfoVec(result))
    }
}
