/* main.rs
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

include!("shm.rs");

#[allow(unused)]
mod ffi {
    use super::*;

    // const PDEV_LEN: usize = 16;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct ListHead {
        pub next: *mut ListHead,
        pub prev: *mut ListHead,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUVendor {
        pub list: ListHead,

        pub init: Option<fn() -> u8>,
        pub shutdown: Option<fn()>,

        pub last_error_string: Option<fn() -> *const i8>,

        pub get_device_handles: Option<fn(devices: *mut ListHead, count: *mut u32) -> u8>,

        pub populate_static_info: Option<fn(gpu_info: *mut GPUInfo) -> u8>,
        pub refresh_dynamic_info: Option<fn(gpu_info: *mut GPUInfo) -> u8>,

        pub refresh_running_processes: Option<fn(gpu_info: *mut GPUInfo) -> u8>,

        pub name: *mut i8,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUInfo {
        pub list: ListHead,
        pub vendor: *mut GPUVendor,
        pub static_info: GPUInfoStaticInfo,
        pub dynamic_info: GPUInfoDynamicInfo,
        pub processes_count: u32,
        // pub processes: *mut GPUProcess,
        // pub processes_array_size: u32,
        // pub pdev: [i8; PDEV_LEN],
    }

    extern "C" {
        pub fn gpuinfo_init_info_extraction(
            monitored_dev_count: *mut u32,
            devices: *mut ListHead,
        ) -> u8;

        pub fn gpuinfo_populate_static_infos(devices: *mut ListHead) -> u8;
        pub fn gpuinfo_refresh_dynamic_info(devices: *mut ListHead) -> u8;
    }
}

fn main() {
    let shm_file_link = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: gpud <shm_file_link>");
        std::process::exit(1);
    });

    std::fs::create_dir_all(std::path::Path::new(&shm_file_link).parent().unwrap())
        .expect("Unable to create cache directory");

    let shared_memory =
        SharedMemory::new(&shm_file_link, false).expect("Unable to open shared memory");

    let mut gpu_list: ffi::ListHead = ffi::ListHead {
        next: std::ptr::null_mut(),
        prev: std::ptr::null_mut(),
    };
    gpu_list.next = &mut gpu_list;
    gpu_list.prev = &mut gpu_list;

    let mut gpu_count: u32 = 0;
    unsafe {
        ffi::gpuinfo_init_info_extraction(&mut gpu_count, &mut gpu_list);
        ffi::gpuinfo_populate_static_infos(&mut gpu_list);
    }

    {
        let writer = shared_memory
            .write(raw_sync::Timeout::Infinite)
            .expect("Unable to acquire write lock");
        *writer.gpu_info.0 = gpu_count as usize;
    }

    loop {
        let refresh_interval_ms = {
            let reader = match shared_memory.read(raw_sync::Timeout::Infinite) {
                None => {
                    eprintln!("Unable to acquire read lock");
                    continue;
                }
                Some(reader) => reader,
            };

            if reader.stop_daemon {
                break;
            }

            reader.refresh_interval_ms
        };

        {
            let writer = match shared_memory.write(raw_sync::Timeout::Infinite) {
                None => {
                    eprintln!("Unable to acquire write lock");
                    continue;
                }
                Some(writer) => writer,
            };

            unsafe {
                ffi::gpuinfo_refresh_dynamic_info(&mut gpu_list);
            }

            let mut device: *mut ffi::ListHead = gpu_list.next;
            let mut i = 0;
            while device != (&mut gpu_list) {
                let dev: &ffi::GPUInfo = unsafe { core::mem::transmute(device) };

                let shared_data = &mut writer.gpu_info.1[i];
                shared_data.static_info = dev.static_info;
                shared_data.dynamic_info = dev.dynamic_info;

                device = unsafe { (*device).next };
                i += 1;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(refresh_interval_ms as _));
    }
}
