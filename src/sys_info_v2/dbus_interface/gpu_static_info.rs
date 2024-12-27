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

use std::sync::Arc;

use dbus::{arg::*, strings::*};
use gtk::glib::g_critical;

use super::{deser_str, deser_struct, deser_u16, deser_u64, deser_u8};

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

                        info.id = match deser_str(static_info, "GpuStaticInfo", 0) {
                            Some(id) => id,
                            None => return None,
                        };

                        info.device_name = match deser_str(static_info, "GpuStaticInfo", 1) {
                            Some(name) => name,
                            None => return None,
                        };

                        info.vendor_id = match deser_u16(static_info, "GpuStaticInfo", 2) {
                            Some(vendor_id) => vendor_id,
                            None => return None,
                        };

                        info.device_id = match deser_u16(static_info, "GpuStaticInfo", 3) {
                            Some(device_id) => device_id,
                            None => return None,
                        };

                        info.total_memory = match deser_u64(static_info, "GpuStaticInfo", 4) {
                            Some(total_memory) => total_memory,
                            None => return None,
                        };

                        info.total_gtt = match deser_u64(static_info, "GpuStaticInfo", 5) {
                            Some(total_gtt) => total_gtt,
                            None => return None,
                        };

                        info.opengl_version = match deser_struct(static_info, "GpuStaticInfo", 6) {
                            Some(mut it) => {
                                let it = it.as_mut();
                                let major = if let Some(major) = it.next() {
                                    major.as_u64().unwrap_or(0)
                                } else {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo(OpenGLVersion): Expected '6-0: y', got None",
                                    );

                                    0
                                };

                                let minor = if let Some(minor) = it.next() {
                                    minor.as_u64().unwrap_or(0)
                                } else {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo(OpenGLVersion): Expected '6-1: y', got None",
                                    );

                                    0
                                };

                                let gl_api = if let Some(minor) = it.next() {
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
                            api_versions[i] =
                                match deser_struct(static_info, "GpuStaticInfo", i + 7) {
                                    Some(mut it) => {
                                        let major = deser_u16(
                                            it.as_mut(),
                                            "GpuStaticInfo(ApiVersion)",
                                            i + 9,
                                        )
                                        .unwrap_or(0);

                                        let minor = deser_u16(
                                            it.as_mut(),
                                            "GpuStaticInfo(ApiVersion)",
                                            i + 10,
                                        )
                                        .unwrap_or(0);

                                        let patch = deser_u16(
                                            it.as_mut(),
                                            "GpuStaticInfo(ApiVersion)",
                                            i + 11,
                                        )
                                        .unwrap_or(0);

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

                        info.pcie_gen = match deser_u8(static_info, "GpuStaticInfo", 10) {
                            Some(pcie_gen) => pcie_gen,
                            None => return None,
                        };

                        info.pcie_lanes = match deser_u8(static_info, "GpuStaticInfo", 11) {
                            Some(pcie_lanes) => pcie_lanes,
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
