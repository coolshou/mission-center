/* sys_info/gpu_info.rs
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

use adw::glib::g_critical;

#[allow(unused)]
mod nvtop {
    const MAX_DEVICE_NAME: usize = 128;
    const PDEV_LEN: usize = 16;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct ListHead {
        pub next: *mut ListHead,
        pub prev: *mut ListHead,
    }

    unsafe impl Send for ListHead {}

    unsafe impl Sync for ListHead {}

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUVendor {
        pub list: ListHead,

        pub init: Option<fn() -> u8>,
        pub shutdown: Option<fn()>,

        pub last_error_string: Option<extern "C" fn() -> *const i8>,

        pub get_device_handles:
            Option<extern "C" fn(devices: *mut ListHead, count: *mut u32) -> u8>,

        pub populate_static_info: Option<extern "C" fn(gpu_info: *mut GPUInfo) -> u8>,
        pub refresh_dynamic_info: Option<extern "C" fn(gpu_info: *mut GPUInfo) -> u8>,

        pub refresh_running_processes: Option<extern "C" fn(gpu_info: *mut GPUInfo) -> u8>,

        pub name: *mut i8,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub enum GPUInfoStaticInfoValid {
        DeviceNameValid = 0,
        MaxPcieGenValid,
        MaxPcieLinkWidthValid,
        TemperatureShutdownThresholdValid,
        TemperatureSlowdownThresholdValid,
        StaticInfoCount,
    }

    const GPU_INFO_STATIC_INFO_COUNT: usize = GPUInfoStaticInfoValid::StaticInfoCount as usize;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUInfoStaticInfo {
        pub device_name: [i8; MAX_DEVICE_NAME],
        pub max_pcie_gen: u32,
        pub max_pcie_link_width: u32,
        pub temperature_shutdown_threshold: u32,
        pub temperature_slowdown_threshold: u32,
        pub integrated_graphics: u8,
        pub valid: [u8; (GPU_INFO_STATIC_INFO_COUNT + 7) / 8],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub enum GPUInfoDynamicInfoValid {
        GpuClockSpeedValid = 0,
        GpuClockSpeedMaxValid,
        MemClockSpeedValid,
        MemClockSpeedMaxValid,
        GpuUtilRateValid,
        MemUtilRateValid,
        EncoderRateValid,
        DecoderRateValid,
        TotalMemoryValid,
        FreeMemoryValid,
        UsedMemoryValid,
        PcieLinkGenValid,
        PcieLinkWidthValid,
        PcieRxValid,
        PcieTxValid,
        FanSpeedValid,
        GpuTempValid,
        PowerDrawValid,
        PowerDrawMaxValid,
        DynamicInfoCount,
    }

    const GPU_INFO_DYNAMIC_INFO_COUNT: usize = GPUInfoDynamicInfoValid::DynamicInfoCount as usize;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUInfoDynamicInfo {
        pub gpu_clock_speed: u32,
        pub gpu_clock_speed_max: u32,
        pub mem_clock_speed: u32,
        pub mem_clock_speed_max: u32,
        pub gpu_util_rate: u32,
        pub mem_util_rate: u32,
        pub encoder_rate: u32,
        pub decoder_rate: u32,
        pub total_memory: u64,
        pub free_memory: u64,
        pub used_memory: u64,
        pub pcie_link_gen: u32,
        pub pcie_link_width: u32,
        pub pcie_rx: u32,
        pub pcie_tx: u32,
        pub fan_speed: u32,
        pub gpu_temp: u32,
        pub power_draw: u32,
        pub power_draw_max: u32,
        pub encode_decode_shared: u8,
        pub valid: [u8; (GPU_INFO_DYNAMIC_INFO_COUNT + 7) / 8],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub enum GPUProcessType {
        Unknown = 0,
        Graphical = 1,
        Compute = 2,
        GraphicalCompute = 3,
        Count,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub enum GPUInfoProcessInfoValid {
        CmdlineValid,
        UserNameValid,
        GfxEngineUsedValid,
        ComputeEngineUsedValid,
        EncEngineUsedValid,
        DecEngineUsedValid,
        GpuUsageValid,
        EncodeUsageValid,
        DecodeUsageValid,
        GpuMemoryUsageValid,
        GpuMemoryPercentageValid,
        CpuUsageValid,
        CpuMemoryVirtValid,
        CpuMemoryResValid,
        ProcessValidInfoCount,
    }

    const GPU_PROCESS_INFO_VALID_INFO_COUNT: usize =
        GPUInfoProcessInfoValid::ProcessValidInfoCount as usize;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUProcess {
        pub r#type: GPUProcessType,
        pub pid: i32,
        pub cmdline: *mut i8,
        pub user_name: *mut i8,
        pub gfx_engine_used: u64,
        pub compute_engine_used: u64,
        pub enc_engine_used: u64,
        pub dec_engine_used: u64,
        pub gpu_usage: u32,
        pub encode_usage: u32,
        pub decode_usage: u32,
        pub gpu_memory_usage: libc::c_ulonglong,
        pub gpu_memory_percentage: u32,
        pub cpu_usage: u32,
        pub cpu_memory_virt: libc::c_ulong,
        pub cpu_memory_res: libc::c_ulong,
        pub valid: [u8; (GPU_PROCESS_INFO_VALID_INFO_COUNT + 7) / 8],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUInfo {
        pub list: ListHead,
        pub vendor: *mut GPUVendor,
        pub static_info: GPUInfoStaticInfo,
        pub dynamic_info: GPUInfoDynamicInfo,
        pub processes_count: u32,
        pub processes: *mut GPUProcess,
        pub processes_array_size: u32,
        pub pdev: [i8; PDEV_LEN],
    }

    extern "C" {
        pub fn gpuinfo_init_info_extraction(
            monitored_dev_count: *mut u32,
            devices: *mut ListHead,
        ) -> u8;

        pub fn gpuinfo_populate_static_infos(devices: *mut ListHead) -> u8;
        pub fn gpuinfo_refresh_dynamic_info(devices: *mut ListHead) -> u8;
        pub fn gpuinfo_refresh_processes(devices: *mut ListHead) -> u8;
        pub fn gpuinfo_fix_dynamic_info_from_process_info(devices: *mut ListHead) -> u8;
    }
}

#[derive(Debug, Clone)]
pub struct GPU {
    pub device_name: String,
    pub pci_slot_name: String,
    pub dri_path: String,
    pub vendor_id: Option<u16>,
    pub device_id: Option<u16>,

    pub vulkan_version: Option<(u16, u16, u16)>,
    pub pcie_gen: Option<u8>,
    pub pcie_lanes: Option<u8>,

    pub temp_celsius: u32,
    pub fan_speed_percent: u32,
    pub util_percent: u32,
    pub power_draw_watts: f32,
    pub power_draw_max_watts: f32,
    pub clock_speed_mhz: u32,
    pub clock_speed_max_mhz: u32,
    pub mem_speed_mhz: u32,
    pub mem_speed_max_mhz: u32,
    pub total_memory: u64,
    pub free_memory: u64,
    pub used_memory: u64,
    pub encoder_percent: u32,
    pub decoder_percent: u32,
}

pub struct GPUInfo {
    gpu_list: Box<nvtop::ListHead>,
    gpus: Vec<GPU>,
}

impl GPUInfo {
    pub fn new() -> Option<Self> {
        use gtk::glib::*;

        let mut cache_dir = if let Ok(mut cache_dir) = std::env::var("XDG_CACHE_HOME") {
            cache_dir.push_str("/io.missioncenter.MissionCenter");

            cache_dir
        } else {
            let mut cache_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            cache_dir.push_str("/.cache/io.missioncenter.MissionCenter");

            cache_dir
        };

        std::fs::create_dir_all(&cache_dir).expect("Unable to create cache directory");
        cache_dir.push_str("/gpud_shm");

        let mut gpu_list = Box::new(nvtop::ListHead {
            next: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
        });
        gpu_list.next = gpu_list.as_mut();
        gpu_list.prev = gpu_list.as_mut();

        let mut gpu_count: u32 = 0;
        unsafe {
            let result = nvtop::gpuinfo_init_info_extraction(&mut gpu_count, gpu_list.as_mut());
            if result == 0 {
                g_critical!(
                    "MissionCenter::GPUInfo",
                    "Unable to initialize GPU info extraction"
                );
                return None;
            }

            let result = nvtop::gpuinfo_populate_static_infos(gpu_list.as_mut());
            if result == 0 {
                g_critical!(
                    "MissionCenter::GPUInfo",
                    "Unable to populate static GPU info"
                );
                return None;
            }
        }

        Some(Self {
            gpu_list,
            gpus: vec![],
        })
    }

    pub fn refresh(&mut self) {
        use gtk::glib::*;

        let result = unsafe { nvtop::gpuinfo_refresh_dynamic_info(self.gpu_list.as_mut()) };
        if result == 0 {
            g_critical!(
                "MissionCenter::GPUInfo",
                "Unable to refresh dynamic GPU info"
            );
            return;
        }

        let result = unsafe { nvtop::gpuinfo_refresh_processes(self.gpu_list.as_mut()) };
        if result == 0 {
            g_critical!("MissionCenter::GPUInfo", "Unable to refresh GPU processes");
            return;
        }

        let result =
            unsafe { nvtop::gpuinfo_fix_dynamic_info_from_process_info(self.gpu_list.as_mut()) };
        if result == 0 {
            g_critical!(
                "MissionCenter::GPUInfo",
                "Unable to fix dynamic GPU info from process info"
            );
            return;
        }

        let vulkan_versions = unsafe { Self::supported_vulkan_versions() };
        self.gpus.clear();

        let mut device: *mut nvtop::ListHead = self.gpu_list.next;
        while device != self.gpu_list.as_mut() {
            let dev: &nvtop::GPUInfo = unsafe { core::mem::transmute(device) };

            let pdev = unsafe { std::ffi::CStr::from_ptr(dev.pdev.as_ptr()) }.to_string_lossy();
            // Skip Intel integrated graphics, stats for them are utterly broken
            if pdev == "0000:00:02.0" {
                device = unsafe { (*device).next };
                continue;
            }

            let uevent = std::fs::read_to_string(format!("/sys/bus/pci/devices/{}/uevent", pdev));
            let (vendor_id, device_id) = if let Ok(uevent) = uevent {
                let mut vendor_id = None;
                let mut device_id = None;

                for line in uevent.lines().map(|l| l.trim()) {
                    if line.starts_with("PCI_ID=") {
                        let mut ids = line[7..].split(':');
                        vendor_id = ids.next().and_then(|id| u16::from_str_radix(id, 16).ok());
                        device_id = ids.next().and_then(|id| u16::from_str_radix(id, 16).ok());
                        break;
                    }
                }

                (vendor_id, device_id)
            } else {
                g_critical!(
                    "MissionCenter::GPUInfo",
                    "Unable to read uevent for device {}",
                    pdev
                );

                (None, None)
            };

            let vulkan_version = if vendor_id.is_some() && device_id.is_some() {
                let key = ((vendor_id.unwrap() as u32) << 16) | (device_id.unwrap() as u32);
                if let Some(vulkan_versions) = vulkan_versions.as_ref() {
                    vulkan_versions.get(&key).cloned()
                } else {
                    None
                }
            } else {
                None
            };

            self.gpus.push(GPU {
                device_name: unsafe {
                    std::ffi::CStr::from_ptr(dev.static_info.device_name.as_ptr())
                }
                .to_string_lossy()
                .to_string(),
                pci_slot_name: pdev.to_string(),
                dri_path: format!("/dev/dri/by-path/pci-{}-card", pdev),
                vendor_id,
                device_id,

                vulkan_version,
                pcie_gen: Some(dev.dynamic_info.pcie_link_gen as u8),
                pcie_lanes: Some(dev.dynamic_info.pcie_link_width as u8),

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

            device = unsafe { (*device).next };
        }
    }

    pub fn gpus(&self) -> &[GPU] {
        &self.gpus
    }

    #[allow(non_snake_case)]
    unsafe fn supported_vulkan_versions() -> Option<std::collections::HashMap<u32, (u16, u16, u16)>>
    {
        use gtk::glib::*;

        type Void = std::ffi::c_void;

        const VK_MAX_PHYSICAL_DEVICE_NAME_SIZE: usize = 256;
        const VK_UUID_SIZE: usize = 16;
        const SIZE_OF_LIMITS_STRUCT: usize = 504;
        const SIZE_OF_SPARSE_PROPERTIES_STRUCT: usize = 20;

        #[repr(C)]
        #[derive(Debug, Copy, Clone)]
        struct VkPhysicalDeviceProperties {
            apiVersion: u32,
            driverVersion: u32,
            vendorID: u32,
            deviceID: u32,
            deviceType: i32,
            deviceName: [i8; VK_MAX_PHYSICAL_DEVICE_NAME_SIZE],
            pipelineCacheUUID: [u8; VK_UUID_SIZE],
            limits: [u8; SIZE_OF_LIMITS_STRUCT],
            sparseProperties: [u8; SIZE_OF_SPARSE_PROPERTIES_STRUCT],
        }

        let lib = minidl::Library::load("libvulkan.so.1\0");
        if lib.is_err() {
            g_critical!(
                "MissionCenter::GPUInfo",
                "Failed to create Vulkan instance: Could not load 'libvulkan.so.1'; {}",
                lib.err().unwrap(),
            );
            return None;
        }
        let lib = lib.unwrap();

        let vkGetInstanceProcAddr = lib.sym::<*const Void>("vkGetInstanceProcAddr\0");
        if vkGetInstanceProcAddr.is_err() {
            g_critical!(
                "MissionCenter::GPUInfo",
                "Failed to create Vulkan instance: Could not find 'vkGetInstanceProcAddr' in 'libvulkan.so.1'; {}", vkGetInstanceProcAddr.err().unwrap(),
            );
            return None;
        }
        let vkGetInstanceProcAddr: extern "C" fn(
            vk_instance: *mut Void,
            name: *const u8,
        ) -> *mut Void = core::mem::transmute(vkGetInstanceProcAddr.unwrap());

        let vkCreateInstance =
            vkGetInstanceProcAddr(std::ptr::null_mut(), b"vkCreateInstance\0".as_ptr());
        if vkCreateInstance.is_null() {
            g_critical!(
                "MissionCenter::GPUInfo",
                "Failed to create Vulkan instance: vkCreateInstance not found",
            );
            return None;
        }

        let vkCreateInstance: extern "C" fn(
            create_info: *const i32,
            allocator: *const Void,
            instance: *mut *mut Void,
        ) -> i32 = core::mem::transmute(vkCreateInstance);

        let mut create_info = [0; 16];
        create_info[0] = 1;
        let allocator = std::ptr::null_mut();
        let mut instance = std::ptr::null_mut();
        let result = vkCreateInstance(create_info.as_ptr(), allocator, &mut instance);
        if result != 0 || instance.is_null() {
            g_critical!(
                "MissionCenter::GPUInfo",
                "Failed to create Vulkan instance: vkCreateInstance failed ({})",
                result
            );
            return None;
        }

        let vkEnumeratePhysicalDevices =
            vkGetInstanceProcAddr(instance, b"vkEnumeratePhysicalDevices\0".as_ptr());
        if vkEnumeratePhysicalDevices.is_null() {
            g_critical!(
                "MissionCenter::GPUInfo",
                "Failed to create Vulkan instance: vkEnumeratePhysicalDevices not found",
            );
            return None;
        }

        let vkEnumeratePhysicalDevices: extern "C" fn(
            instance: *mut Void,
            device_count: *mut u32,
            devices: *mut *mut Void,
        ) -> i32 = core::mem::transmute(vkEnumeratePhysicalDevices);

        let mut device_count = 0;
        let result = vkEnumeratePhysicalDevices(instance, &mut device_count, std::ptr::null_mut());
        if result != 0 || device_count == 0 {
            g_critical!(
                "MissionCenter::GPUInfo",
                "Failed to create Vulkan instance: No Vulkan capable devices found ({})",
                result
            );
            return None;
        }

        let mut devices = vec![std::ptr::null_mut(); device_count as usize];
        let result = vkEnumeratePhysicalDevices(instance, &mut device_count, devices.as_mut_ptr());
        if result != 0 || device_count == 0 {
            g_critical!(
                "MissionCenter::GPUInfo",
                "Failed to create Vulkan instance: No Vulkan capable devices found ({})",
                result
            );
            return None;
        }

        let vkGetPhysicalDeviceProperties =
            vkGetInstanceProcAddr(instance, b"vkGetPhysicalDeviceProperties\0".as_ptr());
        if vkGetPhysicalDeviceProperties.is_null() {
            g_critical!(
                "MissionCenter::GPUInfo",
                "Failed to create Vulkan instance: vkGetPhysicalDeviceProperties not found",
            );
            return None;
        }

        let vkGetPhysicalDeviceProperties: extern "C" fn(
            device: *mut Void,
            properties: *mut VkPhysicalDeviceProperties,
        ) = core::mem::transmute(vkGetPhysicalDeviceProperties);

        let mut supported_versions = std::collections::HashMap::new();
        for device in devices {
            let mut properties = core::mem::zeroed();

            vkGetPhysicalDeviceProperties(device, &mut properties);
            g_info!(
                "MissionCenter::GPUInfo",
                "Found Vulkan device: {:?}",
                std::ffi::CStr::from_ptr(properties.deviceName.as_ptr())
            );

            let version = properties.apiVersion;
            let major = (version >> 22) as u16;
            let minor = ((version >> 12) & 0x3ff) as u16;
            let patch = (version & 0xfff) as u16;

            let vendor_id = properties.vendorID & 0xffff;
            let device_id = properties.deviceID & 0xffff;

            supported_versions.insert((vendor_id << 16) | device_id, (major, minor, patch));
        }

        Some(supported_versions)
    }
}
