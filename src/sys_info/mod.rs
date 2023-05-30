/* sys_info/mod.rs
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

use lazy_static::lazy_static;
use sysinfo::{NetworkExt, System, SystemExt};

pub use disk_info::*;
pub use gpu_info::*;
pub use mem_info::*;
pub use net_info::*;

mod disk_info;
mod gpu_info;
mod mem_info;
mod net_info;

const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

lazy_static! {
    static ref IS_FLATPAK: bool = std::path::Path::new("/.flatpak-info").exists();
}

pub struct SysInfo {
    system: System,

    mem_info: MemInfo,

    disk_info: DiskInfo,

    net_info: Option<NetInfo>,
    net_devices: std::collections::HashMap<String, NetworkDevice>,

    gpu_info: Option<GPUInfo>,

    cpu_base_frequency: Option<usize>,
    cpu_socket_count: Option<u8>,
    cpu_caches: std::collections::HashMap<u8, usize>,
    is_vm: Option<bool>,

    file_nr: Option<std::fs::File>,
    proc_count: usize,
    thread_count: usize,
    handle_count: usize,
}

impl SysInfo {
    pub fn new() -> Self {
        use std::collections::HashMap;

        let is_flatpak = *IS_FLATPAK;

        let file_nr = if is_flatpak {
            None
        } else {
            std::fs::OpenOptions::new()
                .read(true)
                .open("/proc/sys/fs/file-nr")
                .ok()
        };

        let cpu_base_frequency = if let Ok(content) =
            std::fs::read("/sys/devices/system/cpu/cpu0/cpufreq/base_frequency")
        {
            if let Ok(content) = std::str::from_utf8(&content) {
                if let Ok(cpu_base_frequency) = content.trim().parse() {
                    Some(cpu_base_frequency)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Self {
            system: System::new_all(),

            mem_info: MemInfo::new(),

            disk_info: DiskInfo::new(),

            net_info: NetInfo::new().ok(),
            net_devices: HashMap::new(),

            gpu_info: GPUInfo::new(),

            cpu_base_frequency,
            cpu_socket_count: Self::load_cpu_socket_count(),
            is_vm: unsafe { Self::load_is_vm() },
            cpu_caches: Self::load_cpu_cache_info(),

            file_nr,
            proc_count: 0,
            thread_count: 0,
            handle_count: 0,
        }
    }

    pub fn system(&self) -> &System {
        &self.system
    }

    pub fn cpu_base_frequency(&self) -> Option<usize> {
        self.cpu_base_frequency
    }

    pub fn cpu_socket_count(&self) -> Option<u8> {
        self.cpu_socket_count
    }

    pub fn virtualization(&self) -> Option<bool> {
        let mut signature_reg = [0u32; 3];
        let res = Self::cpuid_ex::<1, 4>(0, &mut signature_reg);
        if res.is_none() {
            return None;
        }

        let mut features = [0_u32];
        Self::cpuid_ex::<2, 3>(1, &mut features);

        //Is intel? Check bit5
        if signature_reg[0] == 0x756e6547
            && signature_reg[1] == 0x6c65746e
            && signature_reg[2] == 0x49656e69
        {
            return Some((features[0] & 0x20) > 0);
        }

        //Is AMD? check bit2
        if signature_reg[0] == 0x68747541
            && signature_reg[1] == 0x69746e65
            && signature_reg[2] == 0x444d4163
        {
            return Some((features[0] & 0x04) > 0);
        }

        None
    }

    pub fn is_vm(&self) -> Option<bool> {
        self.is_vm
    }

    pub fn cpu_cache_size(&self, level: u8) -> Option<usize> {
        self.cpu_caches.get(&level).copied()
    }

    pub fn process_count(&self) -> usize {
        self.proc_count
    }

    pub fn thread_count(&self) -> usize {
        self.thread_count
    }

    pub fn handle_count(&self) -> usize {
        self.handle_count
    }

    pub fn memory_info(&self) -> &MemInfo {
        &self.mem_info
    }

    pub fn disk_info(&self) -> &DiskInfo {
        &self.disk_info
    }

    pub fn network_device_info(&self, if_name: &str) -> Option<&NetworkDevice> {
        self.net_devices.get(if_name)
    }

    pub fn gpu_info(&self) -> Option<&GPUInfo> {
        self.gpu_info.as_ref()
    }

    pub fn refresh_all(&mut self) {
        self.system.refresh_all();
        self.refresh_process_count();
        self.refresh_thread_count();
        self.refresh_handle_count();

        self.mem_info.refresh();
        self.disk_info.refresh();
        if let Some(gpu_info) = self.gpu_info.as_mut() {
            gpu_info.refresh();
        }
    }

    pub fn refresh_components_list(&mut self) {
        self.system.refresh_components_list();

        for (name, net_info) in self.system.networks() {
            if let Some(net_device_info) = self.net_info.as_mut() {
                if let Some(mut net_device) = net_device_info.load_device(name.as_str()) {
                    net_device.bytes_sent = net_info.transmitted();
                    net_device.bytes_received = net_info.received();
                    self.net_devices.insert(name.clone(), net_device);
                }
            }
        }
    }

    fn refresh_process_count(&mut self) {
        use gtk::glib::*;

        let is_flatpak = *IS_FLATPAK;
        if !is_flatpak {
            self.proc_count = self.system.processes().len();
            return;
        }

        let mut cmd = std::process::Command::new(FLATPAK_SPAWN_CMD);
        cmd.arg("--host")
            .arg("sh")
            .arg("-c")
            .arg("ls -d /proc/[1-9]* | wc -l");

        if let Ok(output) = cmd.output() {
            if output.stderr.len() > 0 {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to get process count, host command execution failed: {}",
                    std::str::from_utf8(output.stderr.as_slice()).unwrap_or("Unknown error")
                );
                return;
            }

            self.proc_count = std::str::from_utf8(output.stdout.as_slice()).map_or(
                self.system.processes().len(),
                |s| {
                    s.trim()
                        .parse()
                        .unwrap_or_else(|_| self.system.processes().len())
                },
            );
        } else {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to get process count, host command execution failed"
            );
        }
    }

    fn refresh_thread_count(&mut self) {
        use gtk::glib::*;

        let is_flatpak = *IS_FLATPAK;
        if !is_flatpak {
            self.thread_count = 0;
            for (pid, _) in self.system.processes() {
                if let Ok(entries) = std::fs::read_dir(format!("/proc/{}/task", pid)) {
                    self.thread_count += entries.count();
                }
            }

            return;
        }

        // https://askubuntu.com/questions/88972/how-to-get-from-terminal-total-number-of-threads-per-process-and-total-for-al
        let mut cmd = std::process::Command::new(FLATPAK_SPAWN_CMD);
        cmd.arg("--host")
            .arg("sh")
            .arg("-c")
            .arg("count() { printf %s\\\\n \"$#\" ; } ; count /proc/[0-9]*/task/[0-9]*");

        if let Ok(output) = cmd.output() {
            if output.stderr.len() > 0 {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to get thread count, host command execution failed: {}",
                    std::str::from_utf8(output.stderr.as_slice()).unwrap_or("Unknown error")
                );
                return;
            }

            self.thread_count = std::str::from_utf8(output.stdout.as_slice())
                .map_or(0, |s| s.trim().parse().unwrap_or_else(|_| 0));
        } else {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to get thread count, host command execution failed"
            );
        }
    }

    fn refresh_handle_count(&mut self) {
        use gtk::glib::*;

        let is_flatpak = *IS_FLATPAK;
        if !is_flatpak {
            if let Some(file_nr) = &mut self.file_nr {
                use std::io::*;

                let mut buf = String::new();
                if let Ok(_) = file_nr.read_to_string(&mut buf) {
                    let mut split = buf.split_whitespace();
                    if let Some(handle_count) = split.next() {
                        self.handle_count = if let Ok(handle_count) = handle_count.parse() {
                            handle_count
                        } else {
                            0
                        }
                    }

                    if let Err(_) = file_nr.seek(SeekFrom::Start(0)) {
                        g_warning!(
                            "MissionCenter::SysInfo",
                            "Failed to rewind 'file-nr' handle"
                        );
                    }
                }
            }
            return;
        }

        let mut cmd = std::process::Command::new(FLATPAK_SPAWN_CMD);
        cmd.arg("--host")
            .arg("sh")
            .arg("-c")
            .arg("cat /proc/sys/fs/file-nr");

        if let Ok(output) = cmd.output() {
            if output.stderr.len() > 0 {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to get thread count, host command execution failed: {}",
                    std::str::from_utf8(output.stderr.as_slice()).unwrap_or("Unknown error")
                );
                return;
            }

            self.handle_count = std::str::from_utf8(output.stdout.as_slice()).map_or(0, |s| {
                s.split_whitespace()
                    .next()
                    .map_or(0, |s| s.trim().parse().unwrap_or_else(|_| 0))
            });
        } else {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to get thread count, host command execution failed"
            );
        }
    }

    fn load_cpu_socket_count() -> Option<u8> {
        use std::{fs::*, io::*};

        let mut sockets = std::collections::HashSet::new();
        sockets.reserve(4);

        let mut buf = String::new();
        if let Ok(entries) = read_dir("/sys/devices/system/cpu/") {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            if let Ok(mut file) =
                                File::open(entry.path().join("topology/physical_package_id"))
                            {
                                buf.clear();
                                if let Ok(_) = file.read_to_string(&mut buf) {
                                    if let Ok(socket_id) = buf.trim().parse::<u8>() {
                                        sockets.insert(socket_id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if sockets.is_empty() {
            None
        } else {
            Some(sockets.len() as u8)
        }
    }

    unsafe fn load_is_vm() -> Option<bool> {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, gobject_ffi::*};

        let mut error: *mut GError = std::ptr::null_mut();
        let mut inner: *mut GVariant = std::ptr::null_mut();

        let systemd_proxy = g_dbus_proxy_new_for_bus_sync(
            G_BUS_TYPE_SYSTEM,
            G_DBUS_PROXY_FLAGS_NONE,
            std::ptr::null_mut(),
            b"org.freedesktop.systemd1\0".as_ptr() as _,
            b"/org/freedesktop/systemd1\0".as_ptr() as _,
            b"org.freedesktop.systemd1\0".as_ptr() as _,
            std::ptr::null_mut(),
            &mut error,
        );

        if systemd_proxy.is_null() {
            g_error_free(error);
            return None;
        }

        let variant = g_dbus_proxy_call_sync(
            systemd_proxy,
            b"org.freedesktop.DBus.Properties.Get\0".as_ptr() as _,
            g_variant_new(
                b"(ss)\0".as_ptr() as _,
                b"org.freedesktop.systemd1.Manager\0".as_ptr() as *const i8,
                b"Virtualization\0".as_ptr() as *const i8,
            ),
            G_DBUS_CALL_FLAGS_NONE,
            -1,
            std::ptr::null_mut(),
            &mut error,
        );
        if variant.is_null() {
            g_error_free(error);
            g_object_unref(systemd_proxy as _);
            return None;
        }

        g_variant_get(variant, b"(v)\0".as_ptr() as _, &mut inner);
        let virt = g_variant_get_string(inner, std::ptr::null_mut());
        let is_vm = g_utf8_strlen(virt, -1) > 0;

        g_variant_unref(variant);
        g_object_unref(systemd_proxy as _);

        Some(is_vm)
    }

    fn load_cpu_cache_info() -> std::collections::HashMap<u8, usize> {
        use std::{fs::*, io::*};

        let mut result = std::collections::HashMap::new();

        let mut buf = String::new();
        if let Ok(entries) = read_dir("/sys/devices/system/cpu/cpu0/cache/") {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let level = if let Ok(mut file) = File::open(entry.path().join("level")) {
                            buf.clear();
                            if let Ok(_) = file.read_to_string(&mut buf) {
                                if let Ok(level) = buf.trim().parse::<u8>() {
                                    Some(level)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        let size = if let Ok(mut file) = File::open(entry.path().join("size")) {
                            buf.clear();
                            if let Ok(_) = file.read_to_string(&mut buf) {
                                if let Ok(size) = buf.trim().trim_matches('K').parse::<usize>() {
                                    Some(size)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let (Some(level), Some(size)) = (level, size) {
                            if let Some(old_size) = result.get_mut(&level) {
                                *old_size += size;
                            } else {
                                result.insert(level, size);
                            }
                        }
                    }
                }
            }
        }

        result
    }

    fn cpuid_ex<const START: u8, const END: u8>(leaf: u32, result: &mut [u32]) -> Option<()> {
        use raw_cpuid::*;

        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        {
            return None;
        }

        let x = cpuid!(leaf);
        for (result_i, i) in (START..END).enumerate() {
            match i {
                0 => {
                    result[result_i] = x.eax;
                }
                1 => {
                    result[result_i] = x.ebx;
                }
                2 => {
                    result[result_i] = x.ecx;
                }
                3 => {
                    result[result_i] = x.edx;
                }
                _ => {
                    return None;
                }
            }
        }

        Some(())
    }
}
