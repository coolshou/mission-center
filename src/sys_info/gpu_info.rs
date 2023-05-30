use std::sync::{Arc, Mutex};

#[allow(unused)]
mod shm {
    include!("../../gpud/src/shm.rs");
}

pub struct GPUInfo {
    shm: shm::SharedMemory,
    shm_file: String,
    gpud_process: Arc<Mutex<std::process::Child>>,
}

impl Drop for GPUInfo {
    fn drop(&mut self) {
        if let Ok(mut gpud_process) = self.gpud_process.lock() {
            let _ = gpud_process.kill();
            gpud_process
                .wait()
                .expect("Unable to wait for gpud process rto exit");
        }
    }
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

        let gpud_executable = match *super::IS_FLATPAK {
            true => "/app/gpud",
            false => "gpud",
        };

        let child = std::process::Command::new(gpud_executable)
            .arg(&cache_dir)
            .spawn();
        if child.is_err() {
            g_critical!(
                "MissionCenter::SysInfo",
                "Unable to spawn gpud process: {}",
                child.err().unwrap()
            );
            return None;
        }

        let this = Self {
            shm,
            shm_file: cache_dir,
            gpud_process: Arc::new(Mutex::new(child.unwrap())),
        };

        // Wait a bit in case the process fails to start
        std::thread::sleep(std::time::Duration::from_millis(100));
        match this.has_gpud_exited() {
            None | Some(true) => return None,
            _ => {}
        }

        Some(this)
    }

    pub fn print_gpu_info(&self) -> Option<()> {
        use gtk::glib::*;

        match self.has_gpud_exited() {
            None => {
                return None;
            }
            Some(true) => {
                let gpud_executable = match *super::IS_FLATPAK {
                    true => "/app/gpud",
                    false => "gpud",
                };

                let child = std::process::Command::new(gpud_executable)
                    .arg(&self.shm_file)
                    .spawn();

                if child.is_err() {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "Unable to respawn gpud process: {}",
                        child.err().unwrap()
                    );
                    return None;
                }
                let child = child.unwrap();

                if let Ok(mut gpud_process) = self.gpud_process.lock() {
                    *gpud_process = child;
                } else {
                    g_critical!("MissionCenter::SysInfo", "Unable to lock gpud process");
                    return None;
                }
            }
            _ => {}
        }

        let reader = self.shm.read(raw_sync::Timeout::Infinite).unwrap();

        for gpu in reader.gpu_info {
            println!(
                "GPU: {}",
                unsafe { std::ffi::CStr::from_ptr(gpu.static_info.device_name.as_ptr()) }
                    .to_string_lossy()
            );
            println!("    Temperature: {}°C", gpu.dynamic_info.gpu_temp);
            println!("    Fan speed: {}%", gpu.dynamic_info.fan_speed);
            println!(
                "    Power usage: {}W",
                gpu.dynamic_info.power_draw as f32 / 1000.
            );
            println!(
                "    Memory usage: {}",
                crate::to_human_readable(gpu.dynamic_info.used_memory as f32, 1024.).0
            );
            println!("    Utilization: {}%", gpu.dynamic_info.gpu_util_rate);
        }

        Some(())
    }

    pub fn stop_gpud(&self) {
        use gtk::glib::*;

        if let Ok(mut gpud_process) = self.gpud_process.lock() {
            let _ = gpud_process.kill();
            gpud_process
                .wait()
                .expect("Unable to wait for gpud process to exit");
        } else {
            g_critical!("MissionCenter::SysInfo", "Unable to lock gpud process");
        }
    }

    fn has_gpud_exited(&self) -> Option<bool> {
        use gtk::glib::*;

        if let Ok(mut gpud_process) = self.gpud_process.lock() {
            match gpud_process.try_wait() {
                Ok(Some(status)) => {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "gpud process exited with status: {}",
                        status
                    );

                    Some(true)
                }
                Err(e) => {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "Unable to wait for gpud process: {}",
                        e
                    );

                    Some(true)
                }
                Ok(None) => Some(false),
            }
        } else {
            g_critical!("MissionCenter::SysInfo", "Unable to lock gpud process");
            None
        }
    }
}
