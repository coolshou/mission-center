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

#[allow(unused)]
mod shm {
    include!("../../gpud/src/shm.rs");
}

#[derive(Debug, Clone)]
pub struct GPU {
    pub device_name: String,
    pub dri_path: String,
    pub temp_celsius: u32,
    pub fan_speed_percent: u32,
    pub util_percent: u32,
    pub power_draw_watts: f32,
    pub clock_speed_mhz: u32,
    pub mem_speed_mhz: u32,
    pub total_memory: u64,
    pub free_memory: u64,
    pub used_memory: u64,
    pub encoder_percent: u32,
    pub decoder_percent: u32,
}

pub struct GPUInfo {
    shm: shm::SharedMemory,
    shm_file: String,
    gpud_process: subprocess::Popen,

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

        let shm = shm::SharedMemory::new(&cache_dir, true);
        if shm.is_none() {
            g_critical!("MissionCenter::SysInfo", "Unable to open shared memory");
            return None;
        }
        let shm = shm.unwrap();

        let child = match Self::start_gpud(&cache_dir) {
            Some(child) => child,
            None => return None,
        };

        let mut this = Self {
            shm,
            shm_file: cache_dir,
            gpud_process: child,
            gpus: vec![],
        };

        // Wait a bit in case the process fails to start
        std::thread::sleep(std::time::Duration::from_millis(100));
        match this.has_gpud_exited() {
            None | Some(true) => return None,
            _ => {}
        }

        Some(this)
    }

    fn start_gpud(shm_file_link: &str) -> Option<subprocess::Popen> {
        use gtk::glib::*;

        let gpud_executable = match *super::IS_FLATPAK {
            true => "/app/gpud",
            false => "gpud",
        };

        let child = subprocess::Exec::cmd(gpud_executable)
            .arg(&shm_file_link)
            .detached()
            .popen();
        if child.is_err() {
            g_critical!(
                "MissionCenter::SysInfo",
                "Unable to spawn gpud process: {}",
                child.err().unwrap()
            );
            return None;
        }

        Some(child.unwrap())
    }

    pub fn refresh(&mut self) {
        use gtk::glib::*;
        use raw_sync::Timeout;
        use std::time::Duration;

        match self.has_gpud_exited() {
            None => {
                return;
            }
            Some(true) => {
                let child = Self::start_gpud(&self.shm_file);
                if child.is_none() {
                    return;
                }
                self.gpud_process = child.unwrap();

                // Wait a bit in case the process fails to start
                std::thread::sleep(std::time::Duration::from_millis(100));
                match self.has_gpud_exited() {
                    None | Some(true) => return,
                    _ => {}
                }
            }
            _ => {}
        }

        if let Some(reader) = self.shm.read(Timeout::Val(Duration::from_secs(1))) {
            self.gpus.clear();

            for gpu in reader.gpu_info {
                self.gpus.push(GPU {
                    device_name: unsafe {
                        std::ffi::CStr::from_ptr(gpu.static_info.device_name.as_ptr())
                    }
                    .to_string_lossy()
                    .to_string(),
                    dri_path: unsafe {
                        std::ffi::CStr::from_ptr(gpu.dri_path.as_ptr() as *const i8)
                    }
                    .to_string_lossy()
                    .to_string(),
                    temp_celsius: gpu.dynamic_info.gpu_temp,
                    fan_speed_percent: gpu.dynamic_info.fan_speed,
                    util_percent: gpu.dynamic_info.gpu_util_rate,
                    power_draw_watts: gpu.dynamic_info.power_draw as f32 / 1000.,
                    clock_speed_mhz: gpu.dynamic_info.gpu_clock_speed,
                    mem_speed_mhz: gpu.dynamic_info.mem_clock_speed,
                    total_memory: gpu.dynamic_info.total_memory,
                    free_memory: gpu.dynamic_info.free_memory,
                    used_memory: gpu.dynamic_info.used_memory,
                    encoder_percent: gpu.dynamic_info.encoder_rate,
                    decoder_percent: gpu.dynamic_info.decoder_rate,
                });
            }
        } else {
            g_warning!(
                "MissionCenter::SysInfo",
                "Unable to read shared memory: Timeout while waiting for lock"
            );
        }
    }

    pub fn gpus(&self) -> &[GPU] {
        &self.gpus
    }

    fn has_gpud_exited(&mut self) -> Option<bool> {
        use gtk::glib::*;

        match self.gpud_process.poll() {
            Some(status) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "gpud process exited with status: {:?}",
                    status
                );

                Some(true)
            }
            None => Some(false),
        }
    }
}
