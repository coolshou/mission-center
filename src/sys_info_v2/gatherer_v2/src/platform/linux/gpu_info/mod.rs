/* sys_info_v2/gatherer/src/platform/linux/gpu_info/mod.rs
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

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::platform::{
    ApiVersion, GpuDynamicInfoExt, GpuInfoExt, GpuStaticInfoExt, OpenGLApiVersion,
};

#[allow(unused)]
mod nvtop;
mod vulkan_info;

pub struct LinuxGpuStaticInfo {
    id: Arc<str>,
    device_name: Arc<str>,
    vendor_id: u16,
    device_id: u16,
    total_memory: u64,
    opengl_version: Option<OpenGLApiVersion>,
    vulkan_version: Option<ApiVersion>,
    pcie_gen: u8,
    pcie_lanes: u8,
}

impl Default for LinuxGpuStaticInfo {
    fn default() -> Self {
        Self {
            id: Arc::from(""),
            device_name: Arc::from(""),
            vendor_id: 0,
            device_id: 0,
            total_memory: 0,
            opengl_version: None,
            vulkan_version: None,
            pcie_gen: 0,
            pcie_lanes: 0,
        }
    }
}

impl GpuStaticInfoExt for LinuxGpuStaticInfo {
    fn id(&self) -> &str {
        self.id.as_ref()
    }

    fn device_name(&self) -> &str {
        self.device_name.as_ref()
    }

    fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    fn device_id(&self) -> u16 {
        self.device_id
    }

    fn total_memory(&self) -> u64 {
        self.total_memory
    }

    fn opengl_version(&self) -> Option<&OpenGLApiVersion> {
        self.opengl_version.as_ref()
    }

    fn vulkan_version(&self) -> Option<&ApiVersion> {
        self.vulkan_version.as_ref()
    }

    fn metal_version(&self) -> Option<&ApiVersion> {
        None
    }

    fn direct3d_version(&self) -> Option<&ApiVersion> {
        None
    }

    fn pcie_gen(&self) -> u8 {
        self.pcie_gen
    }

    fn pcie_lanes(&self) -> u8 {
        self.pcie_lanes
    }
}

pub struct LinuxGpuDynamicInfo {}

impl LinuxGpuDynamicInfo {
    pub fn new() -> Self {
        Self {}
    }
}

impl GpuDynamicInfoExt for LinuxGpuDynamicInfo {
    fn id(&self) -> &str {
        todo!()
    }

    fn temp_celsius(&self) -> u32 {
        todo!()
    }

    fn fan_speed_percent(&self) -> u32 {
        todo!()
    }

    fn util_percent(&self) -> u32 {
        todo!()
    }

    fn power_draw_watts(&self) -> f32 {
        todo!()
    }

    fn power_draw_max_watts(&self) -> f32 {
        todo!()
    }

    fn clock_speed_mhz(&self) -> u32 {
        todo!()
    }

    fn clock_speed_max_mhz(&self) -> u32 {
        todo!()
    }

    fn mem_speed_mhz(&self) -> u32 {
        todo!()
    }

    fn mem_speed_max_mhz(&self) -> u32 {
        todo!()
    }

    fn free_memory(&self) -> u64 {
        todo!()
    }

    fn used_memory(&self) -> u64 {
        todo!()
    }

    fn encoder_percent(&self) -> u32 {
        todo!()
    }

    fn decoder_percent(&self) -> u32 {
        todo!()
    }
}

pub struct LinuxGpuInfo {
    vk_info: Option<vulkan_info::VulkanInfo>,

    gpu_list: Arc<RwLock<nvtop::ListHead>>,
    static_info: HashMap<arrayvec::ArrayString<16>, LinuxGpuStaticInfo>,
    dynamic_info: HashMap<arrayvec::ArrayString<16>, LinuxGpuDynamicInfo>,

    gpu_list_refreshed: bool,
}

impl Drop for LinuxGpuInfo {
    fn drop(&mut self) {
        use std::ops::DerefMut;

        let mut gl = self.gpu_list.write().unwrap();
        unsafe {
            nvtop::gpuinfo_shutdown_info_extraction(gl.deref_mut());
        }
    }
}

impl LinuxGpuInfo {
    pub fn new() -> Self {
        use std::ops::DerefMut;

        unsafe {
            // nvtop::init_extract_gpuinfo_intel();
            nvtop::init_extract_gpuinfo_amdgpu();
            nvtop::init_extract_gpuinfo_nvidia();
        }

        let gpu_list = Arc::new(RwLock::new(nvtop::ListHead {
            next: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
        }));
        {
            let mut gl = gpu_list.write().unwrap();
            gl.next = gl.deref_mut();
            gl.prev = gl.deref_mut();
        }

        Self {
            vk_info: vulkan_info::VulkanInfo::new(),

            gpu_list,

            static_info: HashMap::new(),
            dynamic_info: HashMap::new(),

            gpu_list_refreshed: false,
        }
    }

    #[allow(non_snake_case)]
    unsafe fn supported_opengl_version(dri_path: &str) -> Option<OpenGLApiVersion> {
        use crate::{error, platform::OpenGLApi};
        use gbm::AsRaw;
        use std::os::fd::*;

        type Void = std::ffi::c_void;

        pub struct DrmDevice(std::fs::File);

        impl AsFd for DrmDevice {
            fn as_fd(&self) -> BorrowedFd<'_> {
                self.0.as_fd()
            }
        }

        impl DrmDevice {
            pub fn open(path: &str) -> std::io::Result<Self> {
                let mut options = std::fs::OpenOptions::new();
                options.read(true);
                options.write(true);

                Ok(Self(options.open(path)?))
            }
        }

        impl drm::Device for DrmDevice {}

        let drm_device = match DrmDevice::open(dri_path) {
            Err(e) => {
                error!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: {}", e
                );
                return None;
            }
            Ok(drm_device) => drm_device,
        };

        let gbm_device = match gbm::Device::new(drm_device) {
            Err(e) => {
                error!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: {}", e
                );
                return None;
            }
            Ok(gbm_device) => gbm_device,
        };

        const EGL_CONTEXT_MAJOR_VERSION_KHR: egl::EGLint = 0x3098;
        const EGL_CONTEXT_MINOR_VERSION_KHR: egl::EGLint = 0x30FB;
        const EGL_PLATFORM_GBM_KHR: egl::EGLenum = 0x31D7;
        const EGL_OPENGL_ES3_BIT: egl::EGLint = 0x0040;

        let eglGetPlatformDisplayEXT =
            egl::get_proc_address("eglGetPlatformDisplayEXT") as *const Void;
        let egl_display = if !eglGetPlatformDisplayEXT.is_null() {
            let eglGetPlatformDisplayEXT: extern "C" fn(
                egl::EGLenum,
                *mut Void,
                *const egl::EGLint,
            ) -> egl::EGLDisplay = std::mem::transmute(eglGetPlatformDisplayEXT);
            eglGetPlatformDisplayEXT(
                EGL_PLATFORM_GBM_KHR,
                gbm_device.as_raw() as *mut Void,
                std::ptr::null(),
            )
        } else {
            let eglGetPlatformDisplay =
                egl::get_proc_address("eglGetPlatformDisplay") as *const Void;
            if !eglGetPlatformDisplay.is_null() {
                let eglGetPlatformDisplay: extern "C" fn(
                    egl::EGLenum,
                    *mut Void,
                    *const egl::EGLint,
                ) -> egl::EGLDisplay = std::mem::transmute(eglGetPlatformDisplay);
                eglGetPlatformDisplay(
                    EGL_PLATFORM_GBM_KHR,
                    gbm_device.as_raw() as *mut Void,
                    std::ptr::null(),
                )
            } else {
                egl::get_display(gbm_device.as_raw() as *mut Void)
                    .map_or(std::ptr::null_mut(), |d| d)
            }
        };
        if egl_display.is_null() {
            error!(
                "Gatherer::GpuInfo",
                "Failed to get OpenGL information: Failed to initialize an EGL display ({:X})",
                egl::get_error()
            );
            return None;
        }

        let mut egl_major = 0;
        let mut egl_minor = 0;
        if !egl::initialize(egl_display, &mut egl_major, &mut egl_minor) {
            error!(
                "Gathereer::GpuInfo",
                "Failed to get OpenGL information: Failed to initialize an EGL display ({:X})",
                egl::get_error()
            );
            return None;
        }

        if egl_major < 1 || (egl_major == 1 && egl_minor < 4) {
            error!(
                "Gatherer::GpuInfo",
                "Failed to get OpenGL information: EGL version 1.4 or higher is required to test OpenGL support"
            );
            return None;
        }

        let mut gl_api = egl::EGL_OPENGL_API;
        if !egl::bind_api(gl_api) {
            gl_api = egl::EGL_OPENGL_ES_API;
            if !egl::bind_api(gl_api) {
                error!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: Failed to bind an EGL API ({:X})",
                    egl::get_error()
                );
                return None;
            }
        }

        let egl_config = if gl_api == egl::EGL_OPENGL_ES_API {
            let mut config_attribs = [
                egl::EGL_SURFACE_TYPE,
                egl::EGL_WINDOW_BIT,
                egl::EGL_RENDERABLE_TYPE,
                EGL_OPENGL_ES3_BIT,
                egl::EGL_NONE,
            ];

            let mut egl_config = egl::choose_config(egl_display, &config_attribs, 1);
            if egl_config.is_some() {
                egl_config
            } else {
                config_attribs[3] = egl::EGL_OPENGL_ES2_BIT;
                egl_config = egl::choose_config(egl_display, &config_attribs, 1);
                if egl_config.is_some() {
                    egl_config
                } else {
                    config_attribs[3] = egl::EGL_OPENGL_ES_BIT;
                    egl::choose_config(egl_display, &config_attribs, 1)
                }
            }
        } else {
            let config_attribs = [
                egl::EGL_SURFACE_TYPE,
                egl::EGL_WINDOW_BIT,
                egl::EGL_RENDERABLE_TYPE,
                egl::EGL_OPENGL_BIT,
                egl::EGL_NONE,
            ];

            egl::choose_config(egl_display, &config_attribs, 1)
        };

        if egl_config.is_none() {
            return None;
        }
        let egl_config = match egl_config {
            Some(ec) => ec,
            None => {
                error!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: Failed to choose an EGL config ({:X})",
                    egl::get_error()
                );
                return None;
            }
        };

        let mut ver_major = if gl_api == egl::EGL_OPENGL_API { 4 } else { 3 };
        let mut ver_minor = if gl_api == egl::EGL_OPENGL_API { 6 } else { 0 };

        let mut context_attribs = [
            EGL_CONTEXT_MAJOR_VERSION_KHR,
            ver_major,
            EGL_CONTEXT_MINOR_VERSION_KHR,
            ver_minor,
            egl::EGL_NONE,
        ];

        let mut egl_context;
        loop {
            egl_context = egl::create_context(
                egl_display,
                egl_config,
                egl::EGL_NO_CONTEXT,
                &context_attribs,
            );

            if egl_context.is_some() || (ver_major == 1 && ver_minor == 0) {
                break;
            }

            if ver_minor > 0 {
                ver_minor -= 1;
            } else {
                ver_major -= 1;
                ver_minor = 9;
            }

            context_attribs[1] = ver_major;
            context_attribs[3] = ver_minor;
        }

        match egl_context {
            Some(ec) => egl::destroy_context(egl_display, ec),
            None => {
                error!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: Failed to create an EGL context ({:X})",
                    egl::get_error()
                );
                return None;
            }
        };

        Some(OpenGLApiVersion {
            major: ver_major as u8,
            minor: ver_minor as u8,
            api: if gl_api != egl::EGL_OPENGL_API {
                OpenGLApi::OpenGLES
            } else {
                OpenGLApi::OpenGL
            },
        })
    }
}

impl<'a> GpuInfoExt<'a> for LinuxGpuInfo {
    type S = LinuxGpuStaticInfo;
    type D = LinuxGpuDynamicInfo;
    type P = crate::platform::Processes;
    type Iter = std::iter::Map<
        std::collections::hash_map::Keys<'a, arrayvec::ArrayString<16>, LinuxGpuStaticInfo>,
        fn(&arrayvec::ArrayString<16>) -> &str,
    >;

    fn refresh_gpu_list(&mut self) {
        use crate::{critical, warning};
        use arrayvec::ArrayString;
        use std::{io::Read, ops::DerefMut};

        if self.gpu_list_refreshed {
            return;
        }

        self.gpu_list_refreshed = true;

        let mut gpu_list = self.gpu_list.write().unwrap();
        let gpu_list = gpu_list.deref_mut();

        let mut gpu_count: u32 = 0;
        let nvt_result = unsafe { nvtop::gpuinfo_init_info_extraction(&mut gpu_count, gpu_list) };
        if nvt_result == 0 {
            critical!(
                "Gatherer::GpuInfo",
                "Unable to initialize GPU info extraction"
            );
            return;
        }

        let nvt_result = unsafe { nvtop::gpuinfo_populate_static_infos(gpu_list) };
        if nvt_result == 0 {
            unsafe { nvtop::gpuinfo_shutdown_info_extraction(gpu_list) };

            critical!("Gatherer::GPUInfo", "Unable to populate static GPU info");
            return;
        }

        let result = unsafe { nvtop::gpuinfo_refresh_dynamic_info(gpu_list) };
        if result == 0 {
            critical!("Gatherer::GpuInfo", "Unable to refresh dynamic GPU info");
            return;
        }

        self.static_info.clear();
        self.dynamic_info.clear();

        let mut buffer = String::new();

        let mut device = gpu_list.next;
        while device != gpu_list {
            use std::fmt::Write;

            let dev: &nvtop::GPUInfo = unsafe { core::mem::transmute(device) };
            device = unsafe { (*device).next };

            let pdev = unsafe { std::ffi::CStr::from_ptr(dev.pdev.as_ptr()) };
            let pdev = match pdev.to_str() {
                Ok(pd) => pd,
                Err(_) => {
                    warning!(
                        "Gatherer::GpuInfo",
                        "Unable to convert PCI ID to string: {:?}",
                        pdev
                    );
                    continue;
                }
            };
            let mut pci_bus_id = ArrayString::<16>::new();
            match write!(pci_bus_id, "{}", pdev) {
                Ok(_) => {}
                Err(_) => {
                    warning!(
                        "Gatherer::GpuInfo",
                        "PCI ID exceeds 16 characters: {}",
                        pdev
                    );
                    continue;
                }
            }

            let device_name =
                unsafe { std::ffi::CStr::from_ptr(dev.static_info.device_name.as_ptr()) };
            let device_name = match device_name.to_str() {
                Ok(dn) => dn,
                Err(_) => "Unknown",
            };

            let mut uevent_path = ArrayString::<64>::new();
            let _ = write!(uevent_path, "/sys/bus/pci/devices/{}/uevent", pdev);
            let uevent_file = match std::fs::OpenOptions::new()
                .read(true)
                .open(uevent_path.as_str())
            {
                Ok(f) => Some(f),
                Err(e) => {
                    uevent_path.clear();
                    let _ = write!(
                        uevent_path,
                        "/sys/bus/pci/devices/{}/uevent",
                        pdev.to_lowercase()
                    );
                    match std::fs::OpenOptions::new()
                        .read(true)
                        .open(uevent_path.as_str())
                    {
                        Ok(f) => Some(f),
                        Err(e) => {
                            warning!(
                                "Gatherer::GPUInfo",
                                "Unable to open `uevent` file for device {}",
                                pdev
                            );
                            None
                        }
                    }
                }
            };

            let ven_dev_id = if let Some(mut f) = uevent_file {
                buffer.clear();
                match f.read_to_string(&mut buffer) {
                    Ok(_) => {
                        let mut vendor_id = 0;
                        let mut device_id = 0;

                        for line in buffer.lines().map(|l| l.trim()) {
                            if line.starts_with("PCI_ID=") {
                                let mut ids = line[7..].split(':');
                                vendor_id = ids
                                    .next()
                                    .and_then(|id| u16::from_str_radix(id, 16).ok())
                                    .unwrap_or(0);
                                device_id = ids
                                    .next()
                                    .and_then(|id| u16::from_str_radix(id, 16).ok())
                                    .unwrap_or(0);
                                break;
                            }
                        }

                        (vendor_id, device_id)
                    }
                    Err(e) => {
                        warning!(
                            "Gatherer::GPUInfo",
                            "Unable to read `uevent` file content for device {}",
                            pdev
                        );
                        (0, 0)
                    }
                }
            } else {
                (0, 0)
            };

            let static_info = LinuxGpuStaticInfo {
                id: Arc::from(pdev),
                device_name: Arc::from(device_name),
                vendor_id: ven_dev_id.0,
                device_id: ven_dev_id.1,

                total_memory: dev.dynamic_info.total_memory,

                pcie_gen: dev.dynamic_info.pcie_link_gen as _,
                pcie_lanes: dev.dynamic_info.pcie_link_width as _,

                // Leave the rest for when static info is actually requested
                ..Default::default()
            };

            self.static_info.insert(pci_bus_id.clone(), static_info);
            self.dynamic_info
                .insert(pci_bus_id.clone(), LinuxGpuDynamicInfo::new());
        }
    }

    fn refresh_static_info_cache(&mut self) {
        use arrayvec::ArrayString;
        use std::fmt::Write;

        let vulkan_versions = if let Some(vulkan_info) = &self.vk_info {
            unsafe { vulkan_info.supported_vulkan_versions() }.unwrap_or(HashMap::new())
        } else {
            HashMap::new()
        };

        let mut dri_path = ArrayString::<64>::new_const();
        for (pci_id, static_info) in &mut self.static_info {
            let _ = write!(dri_path, "/dev/dri/by-path/pci-{}-card", pci_id);
            if !std::path::Path::new(dri_path.as_str()).exists() {
                let _ = write!(
                    dri_path,
                    "/dev/dri/by-path/pci-{}-card",
                    pci_id.to_lowercase()
                );
            }
            static_info.opengl_version =
                unsafe { Self::supported_opengl_version(dri_path.as_str()) };

            let device_id = ((static_info.vendor_id as u32) << 16) | static_info.device_id as u32;
            if let Some(vulkan_version) = vulkan_versions.get(&device_id) {
                static_info.vulkan_version = Some(*vulkan_version);
            }
        }
    }

    fn refresh_dynamic_info_cache(&mut self, processes: &Self::P) {}

    fn enumerate(&'a self) -> Self::Iter {
        self.static_info.keys().map(|k| k.as_str())
    }

    fn static_info(&self, id: &str) -> Option<&Self::S> {
        use arrayvec::ArrayString;

        self.static_info
            .get(&ArrayString::<16>::from(id).unwrap_or_default())
    }

    fn dynamic_info(&self, id: &str) -> Option<&Self::D> {
        use arrayvec::ArrayString;

        self.dynamic_info
            .get(&ArrayString::<16>::from(id).unwrap_or_default())
    }
}
