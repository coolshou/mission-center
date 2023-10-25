/* sys_info_v2/gatherer/src/platform/linux/gpu_info/vulkan_info.rs
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

use crate::{critical, debug};

pub struct VulkanInfo {
    vk_instance: *mut std::ffi::c_void,
    vk_destroy_instance_fn:
        extern "C" fn(instance: *mut std::ffi::c_void, allocator: *const std::ffi::c_void),
    vk_enumerate_physical_devices_fn: extern "C" fn(
        instance: *mut std::ffi::c_void,
        device_count: *mut u32,
        devices: *mut *mut std::ffi::c_void,
    ) -> i32,
    vk_get_physical_device_properties_fn:
        extern "C" fn(device: *mut std::ffi::c_void, properties: *mut std::ffi::c_void),
}

impl Drop for VulkanInfo {
    fn drop(&mut self) {
        (self.vk_destroy_instance_fn)(self.vk_instance, std::ptr::null());
    }
}

impl VulkanInfo {
    #[allow(non_snake_case)]
    pub unsafe fn new() -> Option<Self> {
        type Void = std::ffi::c_void;

        let lib = match minidl::Library::load("libvulkan.so.1\0") {
            Err(e) => {
                critical!(
                    "Gatherer::GPU",
                    "Failed to get Vulkan information: Could not load 'libvulkan.so.1'; {}",
                    e
                );
                return None;
            }
            Ok(lib) => lib,
        };

        let vkGetInstanceProcAddr: extern "C" fn(
            vk_instance: *mut Void,
            name: *const u8,
        ) -> *mut Void = match lib.sym::<*const Void>("vkGetInstanceProcAddr\0") {
            Err(e) => {
                critical!(
                    "Gatherer::GPU",
                    "Failed to get Vulkan information: Could not find 'vkGetInstanceProcAddr' in 'libvulkan.so.1'; {}", e
                );
                return None;
            }
            Ok(vkGetInstanceProcAddr) => core::mem::transmute(vkGetInstanceProcAddr),
        };

        let vkCreateInstance =
            vkGetInstanceProcAddr(std::ptr::null_mut(), b"vkCreateInstance\0".as_ptr());
        if vkCreateInstance.is_null() {
            critical!(
                "Gatherer::GPU",
                "Failed to get Vulkan information: vkCreateInstance not found",
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
            critical!(
                "Gatherer::GPU",
                "Failed to get Vulkan information: vkCreateInstance failed ({})",
                result
            );
            return None;
        }

        let vkDestroyInstance = vkGetInstanceProcAddr(instance, b"vkDestroyInstance\0".as_ptr());
        if vkDestroyInstance.is_null() {
            critical!(
                "Gatherer::GPU",
                "Failed to get Vulkan information: vkDestroyInstance not found, leaking instance",
            );
            return None;
        }

        let vkDestroyInstance: extern "C" fn(instance: *mut Void, allocator: *const Void) =
            core::mem::transmute(vkDestroyInstance);

        let vkEnumeratePhysicalDevices =
            vkGetInstanceProcAddr(instance, b"vkEnumeratePhysicalDevices\0".as_ptr());
        if vkEnumeratePhysicalDevices.is_null() {
            critical!(
                "Gatherer::GPU",
                "Failed to get Vulkan information: vkEnumeratePhysicalDevices not found",
            );
            return None;
        }

        let vkEnumeratePhysicalDevices: extern "C" fn(
            instance: *mut Void,
            device_count: *mut u32,
            devices: *mut *mut Void,
        ) -> i32 = core::mem::transmute(vkEnumeratePhysicalDevices);

        let vkGetPhysicalDeviceProperties =
            vkGetInstanceProcAddr(instance, b"vkGetPhysicalDeviceProperties\0".as_ptr());
        if vkGetPhysicalDeviceProperties.is_null() {
            critical!(
                "Gatherer::GPU",
                "Failed to get Vulkan information: vkGetPhysicalDeviceProperties not found",
            );
            return None;
        }

        let vkGetPhysicalDeviceProperties: extern "C" fn(device: *mut Void, properties: *mut Void) =
            core::mem::transmute(vkGetPhysicalDeviceProperties);

        Some(Self {
            vk_instance: instance,
            vk_destroy_instance_fn: vkDestroyInstance,
            vk_enumerate_physical_devices_fn: vkEnumeratePhysicalDevices,
            vk_get_physical_device_properties_fn: vkGetPhysicalDeviceProperties,
        })
    }

    pub unsafe fn supported_vulkan_versions(
        &self,
    ) -> Option<std::collections::HashMap<u32, (u16, u16, u16)>> {
        return None;

        const VK_MAX_PHYSICAL_DEVICE_NAME_SIZE: usize = 256;
        const VK_UUID_SIZE: usize = 16;
        const SIZE_OF_LIMITS_STRUCT: usize = 504;
        const SIZE_OF_SPARSE_PROPERTIES_STRUCT: usize = 20;

        #[allow(non_snake_case)]
        #[repr(C)]
        #[derive(Debug, Copy, Clone)]
        struct VkPhysicalDeviceProperties {
            apiVersion: u32,
            driverVersion: u32,
            vendorID: u32,
            deviceID: u32,
            deviceType: i32,
            deviceName: [libc::c_char; VK_MAX_PHYSICAL_DEVICE_NAME_SIZE],
            pipelineCacheUUID: [u8; VK_UUID_SIZE],
            limits: [u8; SIZE_OF_LIMITS_STRUCT],
            sparseProperties: [u8; SIZE_OF_SPARSE_PROPERTIES_STRUCT],
        }

        let mut device_count = 0;
        let result = (self.vk_enumerate_physical_devices_fn)(
            self.vk_instance,
            &mut device_count,
            std::ptr::null_mut(),
        );
        if result != 0 || device_count == 0 {
            critical!(
                "Gatherer::GPU",
                "Failed to get Vulkan information: No Vulkan capable devices found ({})",
                result
            );
            return None;
        }

        let mut devices = vec![std::ptr::null_mut(); device_count as usize];
        let result = (self.vk_enumerate_physical_devices_fn)(
            self.vk_instance,
            &mut device_count,
            devices.as_mut_ptr(),
        );
        if result != 0 || device_count == 0 {
            critical!(
                "Gatherer::GPU",
                "Failed to get Vulkan information: No Vulkan capable devices found ({})",
                result
            );
            return None;
        }

        let mut supported_versions = std::collections::HashMap::new();
        for device in devices {
            let mut properties: VkPhysicalDeviceProperties = core::mem::zeroed();

            (self.vk_get_physical_device_properties_fn)(
                device,
                &mut properties as *mut VkPhysicalDeviceProperties as *mut _,
            );
            debug!(
                "Gatherer::GPU",
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
