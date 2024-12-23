/* sys_info_v2/dbus_interface/cpu_static_info.rs
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

use super::{deser_str, deser_u32, deser_u64, deser_u8};

#[derive(Debug, Clone)]
pub struct CpuStaticInfo {
    pub name: Arc<str>,
    pub logical_cpu_count: u32,
    pub socket_count: Option<u8>,
    pub base_frequency_khz: Option<u64>,
    pub virtualization_technology: Option<Arc<str>>,
    pub is_virtual_machine: Option<bool>,
    pub l1_combined_cache: Option<u64>,
    pub l2_cache: Option<u64>,
    pub l3_cache: Option<u64>,
    pub l4_cache: Option<u64>,
}

impl Default for CpuStaticInfo {
    fn default() -> Self {
        Self {
            name: Arc::from(""),
            logical_cpu_count: 0,
            socket_count: None,
            base_frequency_khz: None,
            virtualization_technology: None,
            is_virtual_machine: None,
            l1_combined_cache: None,
            l2_cache: None,
            l3_cache: None,
            l4_cache: None,
        }
    }
}

impl Arg for CpuStaticInfo {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(suytyytttt)")
    }
}

impl ReadAll for CpuStaticInfo {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for CpuStaticInfo {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut this = Self::default();

        let static_info = match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuStaticInfo: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(id) => id,
        };

        let mut static_info = match static_info.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuStaticInfo: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return None;
            }
            Some(i) => i,
        };
        let static_info = static_info.as_mut();

        this.name = match deser_str(static_info, "CpuStaticInfo", "'s' at index 0") {
            Some(n) => n,
            None => return None,
        };

        this.logical_cpu_count = match deser_u32(static_info, "CpuStaticInfo", "'1: u' at index 1")
        {
            Some(lcc) => lcc,
            None => return None,
        };

        this.socket_count = match deser_u8(static_info, "CpuStaticInfo", "'2: y' at index 2") {
            Some(sc) => {
                if sc == 0 {
                    None
                } else {
                    Some(sc)
                }
            }
            None => return None,
        };

        this.base_frequency_khz = match deser_u64(static_info, "CpuStaticInfo", "'3: t' at index 3")
        {
            Some(bf) => {
                if bf == 0 {
                    None
                } else {
                    Some(bf)
                }
            }
            None => return None,
        };

        this.virtualization_technology =
            match deser_str(static_info, "CpuStaticInfo", "'4: s' at index 4") {
                Some(ivs) => {
                    if ivs.is_empty() {
                        None
                    } else {
                        Some(ivs)
                    }
                }
                None => return None,
            };

        this.is_virtual_machine = match deser_u64(static_info, "CpuStaticInfo", "'5: y' at index 5")
        {
            Some(ivm) => match ivm {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            },
            None => return None,
        };

        this.l1_combined_cache = match deser_u64(static_info, "CpuStaticInfo", "'6: t' at index 6")
        {
            Some(l1) => {
                if l1 == 0 {
                    None
                } else {
                    Some(l1)
                }
            }
            None => return None,
        };

        this.l2_cache = match deser_u64(static_info, "CpuStaticInfo", "'7: t' at index 7") {
            Some(l2) => {
                if l2 == 0 {
                    None
                } else {
                    Some(l2)
                }
            }
            None => return None,
        };

        this.l3_cache = match deser_u64(static_info, "CpuStaticInfo", "'8: t' at index 8") {
            Some(l3) => {
                if l3 == 0 {
                    None
                } else {
                    Some(l3)
                }
            }
            None => return None,
        };

        this.l4_cache = match deser_u64(static_info, "CpuStaticInfo", "'9: t' at index 9") {
            Some(l4) => {
                if l4 == 0 {
                    None
                } else {
                    Some(l4)
                }
            }
            None => return None,
        };

        Some(this)
    }
}
