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

lazy_static! {
    static ref INIT_NVTOP: () = unsafe {
        nvtop::init_extract_gpuinfo_amdgpu();
        nvtop::init_extract_gpuinfo_nvidia();
    };
}

mod nvtop;
mod vulkan_info;

pub struct GpuInfo {
    vk_info: Option<vulkan_info::VulkanInfo>,

    gpu_list: Box<nvtop::ListHead>,

    pci_ids_cache: Vec<ArrayString<16>>,
    static_info_cache: HashMap<ArrayString<16>, gpu::StaticInfo>,
    dynamic_info_cache: HashMap<ArrayString<16>, gpu::DynamicInfo>,
    processes_cache: HashMap<ArrayString<16>, gpu::Processes>,
}

impl super::GpuInfoExt for GpuInfo {
    fn new() -> Self {
        let mut gpu_list = Box::new(nvtop::ListHead {
            next: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
        });
        gpu_list.next = gpu_list.as_mut();
        gpu_list.prev = gpu_list.as_mut();

        Self {
            vk_info: unsafe { vulkan_info::VulkanInfo::new() },

            gpu_list,

            pci_ids_cache: Vec::new(),
            static_info_cache: HashMap::new(),
            dynamic_info_cache: HashMap::new(),
            processes_cache: HashMap::new(),
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

    fn static_info(&mut self, pci_id: &str) -> gpu::StaticInfo {
        todo!()
    }

    fn dynamic_info(&mut self, pci_id: &str) -> gpu::DynamicInfo {
        todo!()
    }

    fn processes(&mut self, pci_id: &str) -> gpu::Processes {
        todo!()
    }
}

impl GpuInfo {
    fn load_pci_ids(&mut self) {
        use crate::critical;

        let _ = INIT_NVTOP;

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
                        "Gatherer::GPUInfo",
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
                        "Gatherer::GPUInfo",
                        "PCI ID exceeds 16 characters: {}",
                        pdev
                    );
                    continue;
                }
            }

            self.pci_ids_cache.push(pci_id);
            self.static_info_cache.insert(pci_id, Default::default());
            self.dynamic_info_cache.insert(pci_id, Default::default());
            self.processes_cache.insert(pci_id, Default::default());
        }
    }
}
