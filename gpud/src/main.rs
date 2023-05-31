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

    const PDEV_LEN: usize = 16;

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
        // Process ID
        pub cmdline: *mut i8,
        // Process User Name
        pub user_name: *mut i8,
        // Process User Name
        pub gfx_engine_used: u64,
        // Time in nanoseconds this process spent using the GPU gfx
        pub compute_engine_used: u64,
        // Time in nanoseconds this process spent using the GPU compute
        pub enc_engine_used: u64,
        // Time in nanoseconds this process spent using the GPU encoder
        pub dec_engine_used: u64,
        // Time in nanoseconds this process spent using the GPU decoder
        pub gpu_usage: u32,
        // Percentage of GPU used by the process
        pub encode_usage: u32,
        // Percentage of GPU encoder used by the process
        pub decode_usage: u32,
        // Percentage of GPU decoder used by the process
        pub gpu_memory_usage: libc::c_ulonglong,
        // Memory used by the process
        pub gpu_memory_percentage: u32,
        // Percentage of the total device memory
        // consumed by the process
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

fn main() {
    let parent_pid = unsafe { libc::getppid() };

    let shm_file_link = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: gpud <shm_file_link>");
        std::process::exit(1);
    });

    std::fs::create_dir_all(std::path::Path::new(&shm_file_link).parent().unwrap())
        .expect("Unable to create cache directory");

    dbg!(&shm_file_link);

    let shared_memory =
        SharedMemory::new(&shm_file_link, false).expect("Unable to open shared memory");
    dbg!("Shared memory opened");

    let mut gpu_list: ffi::ListHead = ffi::ListHead {
        next: std::ptr::null_mut(),
        prev: std::ptr::null_mut(),
    };
    gpu_list.next = &mut gpu_list;
    gpu_list.prev = &mut gpu_list;

    let mut gpu_count: u32 = 0;
    unsafe {
        let result = ffi::gpuinfo_init_info_extraction(&mut gpu_count, &mut gpu_list);
        if result == 0 {
            eprintln!("Unable to initialize GPU info extraction");
            std::process::exit(1);
        }

        let result = ffi::gpuinfo_populate_static_infos(&mut gpu_list);
        if result == 0 {
            eprintln!("Unable to populate static GPU info");
            std::process::exit(1);
        }
    }

    dbg!(gpu_count);

    {
        let writer = shared_memory
            .write(raw_sync::Timeout::Infinite)
            .expect("Unable to acquire write lock");
        *writer.gpu_info.0 = gpu_count as usize;
    }
    dbg!("written gpu count");

    loop {
        // If the parent process is no longer running, i.e. the parent PID changes, exit.
        if unsafe { libc::getppid() } != parent_pid {
            eprintln!("Parent process no longer running, exiting");
            break;
        }

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
        dbg!(refresh_interval_ms);

        {
            let writer = match shared_memory.write(raw_sync::Timeout::Infinite) {
                None => {
                    eprintln!("Unable to acquire write lock");
                    continue;
                }
                Some(writer) => writer,
            };
            dbg!("got writer lock");

            let result = unsafe { ffi::gpuinfo_refresh_dynamic_info(&mut gpu_list) };
            if result == 0 {
                eprintln!("Unable to refresh dynamic GPU info");
                continue;
            }

            let result = unsafe { ffi::gpuinfo_refresh_processes(&mut gpu_list) };
            if result == 0 {
                eprintln!("Unable to refresh GPU processes");
                continue;
            }

            let result = unsafe { ffi::gpuinfo_fix_dynamic_info_from_process_info(&mut gpu_list) };
            if result == 0 {
                eprintln!("Unable to fix dynamic GPU info from process info");
                continue;
            }

            let mut device: *mut ffi::ListHead = gpu_list.next;
            let mut i = 0;
            while device != (&mut gpu_list) {
                let dev: &ffi::GPUInfo = unsafe { core::mem::transmute(device) };
                dbg!(dev);

                let shared_data = &mut writer.gpu_info.1[i];
                shared_data.static_info = dev.static_info;
                shared_data.dynamic_info = dev.dynamic_info;

                let dri_path_prefix = b"/dev/dri/by-path/pci-";
                let dri_device_name =
                    unsafe { std::ffi::CStr::from_ptr(dev.pdev.as_ptr()).to_bytes() };
                let dri_path_suffix = b"-card";

                let total_len =
                    dri_path_prefix.len() + dri_device_name.len() + dri_path_suffix.len();
                if total_len < shared_data.dri_path.len() {
                    shared_data.dri_path = unsafe { core::mem::zeroed() };

                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            dri_path_prefix.as_ptr(),
                            shared_data.dri_path.as_mut_ptr(),
                            dri_path_prefix.len(),
                        );

                        std::ptr::copy_nonoverlapping(
                            dri_device_name.as_ptr(),
                            shared_data.dri_path.as_mut_ptr().add(dri_path_prefix.len()),
                            dri_device_name.len(),
                        );

                        std::ptr::copy_nonoverlapping(
                            dri_path_suffix.as_ptr(),
                            shared_data
                                .dri_path
                                .as_mut_ptr()
                                .add(dri_path_prefix.len() + dri_device_name.len()),
                            dri_path_suffix.len(),
                        );
                    }
                } else {
                    eprintln!("DRI path too long");
                }

                device = unsafe { (*device).next };
                i += 1;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(refresh_interval_ms as _));
    }
}
