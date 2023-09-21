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

use std::collections::HashMap;

use arrayvec::ArrayString;
use lazy_static::lazy_static;

use super::gpu;
use super::GpuInfoExt;

lazy_static! {
    static ref INIT_NVTOP: () = unsafe {
        nvtop::init_extract_gpuinfo_amdgpu();
        nvtop::init_extract_gpuinfo_nvidia();
    };
}

#[allow(unused)]
mod nvtop;
mod vulkan_info;

pub struct GpuInfo {
    vk_info: Option<vulkan_info::VulkanInfo>,

    gpu_list: Box<nvtop::ListHead>,

    static_info: HashMap<ArrayString<16>, gpu::StaticInfoDescriptor>,

    pci_ids_cache: Vec<ArrayString<16>>,
    static_info_cache: Vec<gpu::StaticInfoDescriptor>,
    dynamic_info_cache: Vec<gpu::DynamicInfoDescriptor>,
    processes_cache: Vec<gpu::Process>,
}

impl Drop for GpuInfo {
    fn drop(&mut self) {
        unsafe {
            nvtop::gpuinfo_shutdown_info_extraction(self.gpu_list.as_mut());
        }
    }
}

impl GpuInfoExt for GpuInfo {
    fn new() -> Self {
        #[allow(unused_variables)]
        let init = *INIT_NVTOP;

        let mut gpu_list = Box::new(nvtop::ListHead {
            next: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
        });
        gpu_list.next = gpu_list.as_mut();
        gpu_list.prev = gpu_list.as_mut();

        Self {
            vk_info: unsafe { vulkan_info::VulkanInfo::new() },

            gpu_list,

            static_info: HashMap::new(),

            pci_ids_cache: Vec::new(),
            static_info_cache: Vec::new(),
            dynamic_info_cache: Vec::new(),
            processes_cache: Vec::new(),
        }
    }

    fn enumerate(&mut self) -> gpu::PciIds {
        let mut result = gpu::PciIds::default();

        if self.pci_ids_cache.is_empty() {
            self.load_pci_ids();
        }

        let drop_count = self
            .pci_ids_cache
            .chunks(result.ids.capacity())
            .next()
            .unwrap_or(&[])
            .len();

        let it = self.pci_ids_cache.drain(0..drop_count);
        result.ids.extend(it);
        result.is_complete = self.pci_ids_cache.is_empty();

        result
    }

    fn static_info(&mut self) -> gpu::StaticInfo {
        let mut result = gpu::StaticInfo::default();

        if self.static_info_cache.is_empty() {
            self.load_static_info();
        }

        let drop_count = self
            .static_info_cache
            .chunks(result.desc.capacity())
            .next()
            .unwrap_or(&[])
            .len();

        let it = self.static_info_cache.drain(0..drop_count);
        result.desc.extend(it);
        result.is_complete = self.static_info_cache.is_empty();

        result
    }

    fn dynamic_info(&mut self) -> gpu::DynamicInfo {
        let mut result = gpu::DynamicInfo::default();

        if self.dynamic_info_cache.is_empty() {
            self.load_dynamic_info();
        }

        let drop_count = self
            .dynamic_info_cache
            .chunks(result.desc.capacity())
            .next()
            .unwrap_or(&[])
            .len();

        let it = self.dynamic_info_cache.drain(0..drop_count);
        result.desc.extend(it);
        result.is_complete = self.dynamic_info_cache.is_empty();

        result
    }

    fn processes(&mut self) -> gpu::Processes {
        let mut result = gpu::Processes::default();

        if self.processes_cache.is_empty() {
            self.load_dynamic_info();
        }

        let drop_count = self
            .processes_cache
            .chunks(result.usage.capacity())
            .next()
            .unwrap_or(&[])
            .len();

        let it = self.processes_cache.drain(0..drop_count);
        result.usage.extend(it);
        result.is_complete = self.processes_cache.is_empty();

        result
    }
}

impl GpuInfo {
    fn load_pci_ids(&mut self) {
        use crate::{critical, ToArrayStringLossy};

        let mut gpu_count: u32 = 0;
        let nvt_result =
            unsafe { nvtop::gpuinfo_init_info_extraction(&mut gpu_count, self.gpu_list.as_mut()) };
        if nvt_result == 0 {
            critical!(
                "Gatherer::GpuInfo",
                "Unable to initialize GPU info extraction"
            );
            return;
        }

        let nvt_result = unsafe { nvtop::gpuinfo_populate_static_infos(self.gpu_list.as_mut()) };
        if nvt_result == 0 {
            unsafe { nvtop::gpuinfo_shutdown_info_extraction(self.gpu_list.as_mut()) };

            critical!("Gatherer::GPUInfo", "Unable to populate static GPU info");
            return;
        }

        let result = unsafe { nvtop::gpuinfo_refresh_dynamic_info(self.gpu_list.as_mut()) };
        if result == 0 {
            critical!("Gatherer::GpuInfo", "Unable to refresh dynamic GPU info");
            return;
        }

        self.static_info.clear();
        self.static_info_cache.clear();
        self.dynamic_info_cache.clear();
        self.processes_cache.clear();

        let mut device: *mut nvtop::ListHead = self.gpu_list.next;
        while device != self.gpu_list.as_mut() {
            use std::fmt::Write;

            let dev: &nvtop::GPUInfo = unsafe { core::mem::transmute(device) };
            device = unsafe { (*device).next };

            let pdev = unsafe { std::ffi::CStr::from_ptr(dev.pdev.as_ptr()) };
            let pdev = match pdev.to_str() {
                Ok(pd) => pd,
                Err(_) => {
                    critical!(
                        "Gatherer::GpuInfo",
                        "Unable to convert PCI ID to string: {:?}",
                        pdev
                    );
                    continue;
                }
            };
            let mut pci_id = ArrayString::<16>::new();
            match write!(pci_id, "{}", pdev) {
                Ok(_) => {}
                Err(_) => {
                    critical!(
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
            let mut uevent = std::fs::read_to_string(uevent_path.as_str());
            if uevent.is_err() {
                uevent_path.clear();
                let _ = write!(
                    uevent_path,
                    "/sys/bus/pci/devices/{}/uevent",
                    pdev.to_lowercase()
                );
                uevent = std::fs::read_to_string(uevent_path.as_str());
            }
            let ven_dev_id = if let Ok(uevent) = uevent {
                let mut vendor_id = 0;
                let mut device_id = 0;

                for line in uevent.lines().map(|l| l.trim()) {
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
            } else {
                critical!(
                    "Gatherer::GPUInfo",
                    "Unable to read uevent for device {}",
                    pdev
                );

                (0, 0)
            };

            let static_info = gpu::StaticInfoDescriptor {
                pci_id: pci_id.clone(),
                device_name: device_name.to_array_string_lossy(),
                vendor_id: ven_dev_id.0,
                device_id: ven_dev_id.1,

                pcie_gen: dev.dynamic_info.pcie_link_gen as _,
                pcie_lanes: dev.dynamic_info.pcie_link_width as _,

                // Leave the rest for when static info is actually requested
                ..Default::default()
            };

            self.pci_ids_cache.push(pci_id);
            self.static_info.insert(pci_id, static_info);
        }
    }

    fn load_static_info(&mut self) {
        use std::fmt::Write;

        let vulkan_versions = if let Some(vulkan_info) = &self.vk_info {
            unsafe { vulkan_info.supported_vulkan_versions() }.unwrap_or(HashMap::new())
        } else {
            HashMap::new()
        };

        self.static_info_cache.clear();
        for (pci_id, static_info) in &mut self.static_info {
            let mut dri_path = ArrayString::<64>::new();
            let _ = write!(dri_path, "/dev/dri/by-path/pci-{}-card", pci_id);
            let mut opengl_version = unsafe { Self::supported_opengl_version(dri_path.as_str()) };
            if opengl_version.is_none() {
                dri_path.clear();
                let _ = write!(
                    dri_path,
                    "/dev/dri/by-path/pci-{}-card",
                    pci_id.to_lowercase()
                );
                opengl_version = unsafe { Self::supported_opengl_version(dri_path.as_str()) };
            }
            static_info.opengl_version = opengl_version;

            let device_id = ((static_info.vendor_id as u32) << 16) | static_info.device_id as u32;
            if let Some(vulkan_version) = vulkan_versions.get(&device_id) {
                static_info.vulkan_version = Some(*vulkan_version);
            }

            self.static_info_cache.push(static_info.clone());
        }

        self.static_info_cache
            .sort_by(|dev1, dev2| dev1.pci_id.cmp(&dev2.pci_id))
    }

    fn load_dynamic_info(&mut self) {
        use crate::critical;
        use std::fmt::Write;

        self.dynamic_info_cache.clear();
        self.processes_cache.clear();

        let result = unsafe { nvtop::gpuinfo_refresh_dynamic_info(self.gpu_list.as_mut()) };
        if result == 0 {
            critical!("Gatherer::GpuInfo", "Unable to refresh dynamic GPU info");
            return;
        }

        let result = unsafe { nvtop::gpuinfo_refresh_processes(self.gpu_list.as_mut()) };
        if result == 0 {
            critical!("Gatherer::GpuInfo", "Unable to refresh GPU processes");
            return;
        }

        let result =
            unsafe { nvtop::gpuinfo_fix_dynamic_info_from_process_info(self.gpu_list.as_mut()) };
        if result == 0 {
            critical!(
                "Gatherer::GpuInfo",
                "Unable to fix dynamic GPU info from process info"
            );
            return;
        }

        let mut device: *mut nvtop::ListHead = self.gpu_list.next;
        while device != self.gpu_list.as_mut() {
            let dev: &nvtop::GPUInfo = unsafe { core::mem::transmute(device) };
            device = unsafe { (*device).next };

            let pdev = unsafe { std::ffi::CStr::from_ptr(dev.pdev.as_ptr()) };
            let pdev = match pdev.to_str() {
                Ok(pd) => pd,
                Err(_) => {
                    critical!(
                        "Gatherer::GpuInfo",
                        "Unable to convert PCI ID to string: {:?}",
                        pdev
                    );
                    continue;
                }
            };
            let mut pci_id = ArrayString::<16>::new();
            match write!(pci_id, "{}", pdev) {
                Ok(_) => {}
                Err(_) => {
                    critical!(
                        "Gatherer::GpuInfo",
                        "PCI ID exceeds 16 characters: {}",
                        pdev
                    );
                    continue;
                }
            }

            self.dynamic_info_cache.push(gpu::DynamicInfoDescriptor {
                pci_id: pci_id.clone(),
                temp_celsius: dev.dynamic_info.gpu_temp,
                fan_speed_percent: dev.dynamic_info.fan_speed,
                util_percent: dev.dynamic_info.gpu_util_rate,
                power_draw_watts: dev.dynamic_info.power_draw as f32 / 1000.,
                power_draw_max_watts: dev.dynamic_info.power_draw_max as f32 / 1000.,
                clock_speed_mhz: dev.dynamic_info.gpu_clock_speed,
                clock_speed_max_mhz: dev.dynamic_info.gpu_clock_speed_max,
                mem_speed_mhz: dev.dynamic_info.mem_clock_speed,
                mem_speed_max_mhz: dev.dynamic_info.mem_clock_speed_max,
                total_memory: dev.dynamic_info.total_memory,
                free_memory: dev.dynamic_info.free_memory,
                used_memory: dev.dynamic_info.used_memory,
                encoder_percent: dev.dynamic_info.encoder_rate,
                decoder_percent: dev.dynamic_info.decoder_rate,
            });

            for i in 0..dev.processes_count as usize {
                let process = unsafe { &*dev.processes.add(i) };
                self.processes_cache.push(gpu::Process {
                    pci_id: pci_id.clone(),
                    pid: process.pid as _,
                    usage: process.gpu_usage as f32,
                });
            }
        }

        self.dynamic_info_cache
            .sort_by(|dev1, dev2| dev1.pci_id.cmp(&dev2.pci_id));
        self.processes_cache
            .sort_by(|p1, p2| p1.pci_id.cmp(&p2.pci_id));
    }

    #[allow(non_snake_case)]
    unsafe fn supported_opengl_version(dri_path: &str) -> Option<(u8, u8, bool)> {
        use crate::critical;
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
                critical!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: {}",
                    e
                );
                return None;
            }
            Ok(drm_device) => drm_device,
        };

        let gbm_device = match gbm::Device::new(drm_device) {
            Err(e) => {
                critical!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: {}",
                    e
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
            critical!(
                "Gatherer::GpuInfo",
                "Failed to get OpenGL information: Failed to initialize an EGL display ({:X})",
                egl::get_error()
            );
            return None;
        }

        let mut egl_major = 0;
        let mut egl_minor = 0;
        if !egl::initialize(egl_display, &mut egl_major, &mut egl_minor) {
            critical!(
                "Gathereer::GpuInfo",
                "Failed to get OpenGL information: Failed to initialize an EGL display ({:X})",
                egl::get_error()
            );
            return None;
        }

        if egl_major < 1 || (egl_major == 1 && egl_minor < 4) {
            critical!(
                "Gatherer::GpuInfo",
                "Failed to get OpenGL information: EGL version 1.4 or higher is required to test OpenGL support"
            );
            return None;
        }

        let mut gl_api = egl::EGL_OPENGL_API;
        if !egl::bind_api(gl_api) {
            gl_api = egl::EGL_OPENGL_ES_API;
            if !egl::bind_api(gl_api) {
                critical!(
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
                critical!(
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
                critical!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: Failed to create an EGL context ({:X})",
                    egl::get_error()
                );
                return None;
            }
        };

        Some((
            ver_major as u8,
            ver_minor as u8,
            gl_api != egl::EGL_OPENGL_API,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_gpu_info() {
        let mut gpu_info = GpuInfo::new();
        let pci_ids = gpu_info.enumerate();
        assert!(!pci_ids.ids.is_empty());
        assert!(pci_ids.is_complete);
        dbg!(&pci_ids);

        let static_info = gpu_info.static_info();
        assert!(!static_info.desc.is_empty());
        assert!(static_info.is_complete);
        dbg!(&static_info);

        let dynamic_info = gpu_info.dynamic_info();
        assert!(!dynamic_info.desc.is_empty());
        assert!(dynamic_info.is_complete);

        let dynamic_info = gpu_info.dynamic_info();
        assert!(!dynamic_info.desc.is_empty());
        assert!(dynamic_info.is_complete);
        dbg!(&dynamic_info);

        let processes = gpu_info.processes();
        dbg!(&processes);
    }
}
