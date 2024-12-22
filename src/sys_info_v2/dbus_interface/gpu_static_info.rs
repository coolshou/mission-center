/* sys_info_v2/dbus_interface/gpu_static_info.rs
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

use std::fmt::Write;
use std::sync::Arc;

use arrayvec::ArrayString;
use dbus::{arg::*, strings::*};
use gtk::glib::g_critical;

use super::deserialize_field;

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum OpenGLApi {
    OpenGL,
    OpenGLES,
    Invalid = 255,
}

#[derive(Debug, Copy, Clone)]
pub struct OpenGLApiVersion {
    pub major: u8,
    pub minor: u8,
    pub api: OpenGLApi,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

#[derive(Debug, Clone)]
pub struct GpuStaticInfo {
    pub id: Arc<str>,
    pub device_name: Arc<str>,
    pub vendor_id: u16,
    pub device_id: u16,
    pub total_memory: u64,
    pub total_gtt: u64,
    pub opengl_version: Option<OpenGLApiVersion>,
    pub vulkan_version: Option<ApiVersion>,
    pub metal_version: Option<ApiVersion>,
    pub direct3d_version: Option<ApiVersion>,
    pub pcie_gen: u8,
    pub pcie_lanes: u8,
}

impl Default for GpuStaticInfo {
    fn default() -> Self {
        let empty = Arc::<str>::from("");
        GpuStaticInfo {
            id: empty.clone(),
            device_name: empty,
            vendor_id: 0,
            device_id: 0,
            total_memory: 0,
            total_gtt: 0,
            opengl_version: None,
            vulkan_version: None,
            metal_version: None,
            direct3d_version: None,
            pcie_gen: 0,
            pcie_lanes: 0,
        }
    }
}

pub struct GpuStaticInfoVec(pub Vec<GpuStaticInfo>);

impl From<GpuStaticInfoVec> for Vec<GpuStaticInfo> {
    fn from(v: GpuStaticInfoVec) -> Self {
        v.0
    }
}

impl From<Vec<GpuStaticInfo>> for GpuStaticInfoVec {
    fn from(v: Vec<GpuStaticInfo>) -> Self {
        GpuStaticInfoVec(v)
    }
}

impl Arg for GpuStaticInfoVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        dbus::Signature::from("a(ssqqt(yyy)(qqq)(qqq)(qqq)yy)")
    }
}

impl ReadAll for GpuStaticInfoVec {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for GpuStaticInfoVec {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
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
                    for static_info in arr {
                        let empty_string = Arc::<str>::from("");

                        let mut info = GpuStaticInfo {
                            id: empty_string.clone(),
                            device_name: empty_string,
                            vendor_id: 0,
                            device_id: 0,
                            total_memory: 0,
                            total_gtt: 0,
                            opengl_version: None,
                            vulkan_version: None,
                            metal_version: None,
                            direct3d_version: None,
                            pcie_gen: 0,
                            pcie_lanes: 0,
                        };

                        let mut static_info = match static_info.as_iter() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo: Expected '0: STRUCT', got None, failed to iterate over fields",
                                );
                                return None;
                            }
                            Some(i) => i,
                        };
                        let static_info = static_info.as_mut();

                        info.id = match deserialize_field(
                            static_info,
                            "GpuStaticInfo",
                            "'s' at index 0",
                            |arg| arg.as_str().map(Arc::from),
                        ) {
                            Some(id) => id,
                            None => return None,
                        };

                        info.device_name = match deserialize_field(
                            static_info,
                            "GpuStaticInfo",
                            "'s' at index 1",
                            |arg| arg.as_str().map(Arc::from),
                        ) {
                            Some(name) => name,
                            None => return None,
                        };

                        info.vendor_id = match deserialize_field(
                            static_info,
                            "GpuStaticInfo",
                            "'q' at index 2",
                            |arg| arg.as_u64(),
                        ) {
                            Some(vendor_id) => vendor_id as _,
                            None => return None,
                        };

                        info.device_id = match deserialize_field(
                            static_info,
                            "GpuStaticInfo",
                            "'3: q' at index 3",
                            |arg| arg.as_u64(),
                        ) {
                            Some(device_id) => device_id as _,
                            None => return None,
                        };

                        info.total_memory = match deserialize_field(
                            static_info,
                            "GpuStaticInfo",
                            "'t' at index 4",
                            |arg| arg.as_u64(),
                        ) {
                            Some(total_memory) => total_memory,
                            None => return None,
                        };

                        info.total_gtt = match deserialize_field(
                            static_info,
                            "GpuStaticInfo",
                            "'t' at index 5",
                            |arg| arg.as_u64(),
                        ) {
                            Some(total_gtt) => total_gtt,
                            None => return None,
                        };

                        info.opengl_version = match deserialize_field(
                            static_info,
                            "GpuStaticInfo",
                            "STRUCT at index 6",
                            |arg| arg.as_iter(),
                        ) {
                            Some(mut it) => {
                                let major = if let Some(major) = Iterator::next(it.as_mut()) {
                                    major.as_u64().unwrap_or(0)
                                } else {
                                    g_critical!(
                                            "MissionCenter::GathererDBusProxy",
                                            "Failed to get GpuStaticInfo(OpenGLVersion): Expected '6-0: y', got None",
                                        );

                                    0
                                };

                                let minor = if let Some(minor) = Iterator::next(it.as_mut()) {
                                    minor.as_u64().unwrap_or(0)
                                } else {
                                    g_critical!(
                                            "MissionCenter::GathererDBusProxy",
                                            "Failed to get GpuStaticInfo(OpenGLVersion): Expected '6-1: y', got None",
                                        );

                                    0
                                };

                                let gl_api = if let Some(minor) = Iterator::next(it.as_mut()) {
                                    match minor.as_u64().unwrap_or(OpenGLApi::Invalid as u64) {
                                        0 => OpenGLApi::OpenGL,
                                        1 => OpenGLApi::OpenGLES,
                                        _ => OpenGLApi::Invalid,
                                    }
                                } else {
                                    g_critical!(
                                            "MissionCenter::GathererDBusProxy",
                                            "Failed to get GpuStaticInfo(OpenGLVersion): Expected '6-2: y', got None",
                                        );

                                    OpenGLApi::Invalid
                                };

                                if major == 0 || minor == 0 || gl_api == OpenGLApi::Invalid {
                                    None
                                } else {
                                    Some(OpenGLApiVersion {
                                        major: major as u8,
                                        minor: minor as u8,
                                        api: gl_api,
                                    })
                                }
                            }
                            None => return None,
                        };

                        let mut api_versions = [None; 3];
                        for i in 0..3 {
                            let mut description = ArrayString::<18>::new();
                            write!(&mut description, "STRUCT at index {}", i + 7).unwrap_or_else(|_| {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo(ApiVersion): Expected 'y at index {}', failed to write description",
                                    i + 7,
                                );
                            });

                            api_versions[i] = match deserialize_field(
                                static_info,
                                "GpuStaticInfo",
                                &description,
                                |arg| arg.as_iter(),
                            ) {
                                Some(mut it) => {
                                    description.clear();
                                    write!(&mut description, "y at index {}", i + 7).unwrap_or_else(|_| {
                                        g_critical!(
                                            "MissionCenter::GathererDBusProxy",
                                            "Failed to get GpuStaticInfo(ApiVersion): Expected 'y at index {}', failed to write description",
                                            i + 7,
                                        );
                                    });

                                    let major = match deserialize_field(
                                        it.as_mut(),
                                        "GpuStaticInfo(ApiVersion)",
                                        &description,
                                        |arg| arg.as_u64(),
                                    ) {
                                        Some(major) => major as u16,
                                        None => 0,
                                    };

                                    let minor = match deserialize_field(
                                        it.as_mut(),
                                        "GpuStaticInfo(ApiVersion)",
                                        &description,
                                        |arg| arg.as_u64(),
                                    ) {
                                        Some(minor) => minor as u16,
                                        None => 0,
                                    };

                                    let patch = match deserialize_field(
                                        it.as_mut(),
                                        "GpuStaticInfo(ApiVersion)",
                                        &description,
                                        |arg| arg.as_u64(),
                                    ) {
                                        Some(patch) => patch as u16,
                                        None => 0,
                                    };

                                    if major == 0 {
                                        None
                                    } else {
                                        Some(ApiVersion {
                                            major,
                                            minor,
                                            patch,
                                        })
                                    }
                                }
                                None => return None,
                            };
                        }

                        info.vulkan_version = api_versions[0];
                        info.metal_version = api_versions[1];
                        info.direct3d_version = api_versions[2];

                        info.pcie_gen = match deserialize_field(
                            static_info,
                            "GpuStaticInfo",
                            "'y' at index 10",
                            |arg| arg.as_u64(),
                        ) {
                            Some(pcie_gen) => pcie_gen as u8,
                            None => return None,
                        };

                        info.pcie_lanes = match deserialize_field(
                            static_info,
                            "GpuStaticInfo",
                            "'y' at index 11",
                            |arg| arg.as_u64(),
                        ) {
                            Some(pcie_lanes) => pcie_lanes as u8,
                            None => return None,
                        };

                        result.push(info);
                    }

                    Some(result.into())
                }
            },
        }
    }
}
