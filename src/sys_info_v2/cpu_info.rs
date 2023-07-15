/* sys_info/cpu_info.rs
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

#[derive(Debug, Clone)]
pub struct StaticInfo {
    pub name: String,
    pub logical_cpu_count: u32,
    pub socket_count: Option<u8>,
    pub base_frequency_khz: Option<u64>,
    pub virtualization: Option<bool>,
    pub virtual_machine: Option<bool>,
    pub l1_cache: Option<usize>,
    pub l2_cache: Option<usize>,
    pub l3_cache: Option<usize>,
    pub l4_cache: Option<usize>,
}

impl StaticInfo {
    fn load(system: &mut sysinfo::System) -> StaticInfo {
        use sysinfo::*;

        system.refresh_cpu_specifics(CpuRefreshKind::new());

        let cache_info = Self::cache_info();

        StaticInfo {
            name: system.global_cpu_info().brand().to_owned(),
            logical_cpu_count: Self::logical_cpu_count(),
            socket_count: Self::socket_count(),
            base_frequency_khz: Self::base_frequency_khz(),
            virtualization: Self::virtualization(),
            virtual_machine: unsafe { Self::virtual_machine() },
            l1_cache: cache_info.get(&1).copied(),
            l2_cache: cache_info.get(&2).copied(),
            l3_cache: cache_info.get(&3).copied(),
            l4_cache: cache_info.get(&4).copied(),
        }
    }

    fn logical_cpu_count() -> u32 {
        num_cpus::get() as u32
    }

    fn socket_count() -> Option<u8> {
        use gtk::glib::*;
        use std::{fs::*, io::*};

        let mut sockets = std::collections::HashSet::new();
        sockets.reserve(4);

        let mut buf = String::new();

        let entries = match read_dir("/sys/devices/system/cpu/") {
            Ok(entries) => entries,
            Err(e) => {
                g_critical!(
                    "MissionCenter::CpuInfo",
                    "Could not read '/sys/devices/system/cpu': {}",
                    e
                );
                return None;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Could not read entry in '/sys/devices/system/cpu': {}",
                        e
                    );
                    continue;
                }
            };

            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Could not read file type for '/sys/devices/system/cpu/{}': {}",
                        entry.file_name().to_string_lossy(),
                        e
                    );
                    continue;
                }
            };

            if !file_type.is_dir() {
                continue;
            }

            let mut file = match File::open(entry.path().join("topology/physical_package_id")) {
                Ok(file) => file,
                Err(_) => {
                    continue;
                }
            };

            buf.clear();
            match file.read_to_string(&mut buf) {
                Ok(_) => {}
                Err(e) => {
                    g_critical!("MissionCenter::CpuInfo", "Could not read '/sys/devices/system/cpu/{}/topology/physical_package_id': {}", file_name, e);
                    continue;
                }
            };

            let socket_id = match buf.trim().parse::<u8>() {
                Ok(socket_id) => socket_id,
                Err(e) => {
                    g_critical!("MissionCenter::CpuInfo", "Could not read '/sys/devices/system/cpu/{}/topology/physical_package_id': {}", file_name, e);
                    continue;
                }
            };
            sockets.insert(socket_id);
        }

        if sockets.is_empty() {
            g_critical!("MissionCenter::CpuInfo", "Could not determine socket count");
            None
        } else {
            Some(sockets.len() as u8)
        }
    }

    fn base_frequency_khz() -> Option<u64> {
        use gtk::glib::*;

        match std::fs::read("/sys/devices/system/cpu/cpu0/cpufreq/base_frequency") {
            Ok(content) => {
                let content = match std::str::from_utf8(&content) {
                    Ok(content) => content,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::CpuInfo",
                            "Could not read base frequency: {}",
                            e
                        );
                        return None;
                    }
                };

                match content.trim().parse() {
                    Ok(freq) => Some(freq),
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::CpuInfo",
                            "Could not read base frequency: {}",
                            e
                        );
                        None
                    }
                }
            }
            Err(e) => {
                g_critical!(
                    "MissionCenter::CpuInfo",
                    "Could not read base frequency: {}; trying /proc/cpuinfo",
                    e
                );

                let cpuinfo = cmd!("cat /proc/cpuinfo").output();
                if cpuinfo.is_err() {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Could not read /proc/cpuinfo: {}",
                        cpuinfo.err().unwrap()
                    );
                    return None;
                }

                let cpuinfo = String::from_utf8(cpuinfo.unwrap().stdout);
                if cpuinfo.is_err() {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Could not read /proc/cpuinfo: {}",
                        cpuinfo.err().unwrap()
                    );
                    return None;
                }
                let cpuinfo = cpuinfo.unwrap();
                let index = cpuinfo.find("cpu MHz");
                if index.is_none() {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Could not find `cpu MHz` in /proc/cpuinfo",
                    );
                    return None;
                }
                let index = index.unwrap();

                let base_frequency = cpuinfo[index..]
                    .lines()
                    .next()
                    .map(|line| line.split(':').nth(1).unwrap_or("").trim())
                    .map(|mhz| mhz.parse::<f32>());
                if base_frequency.is_none() {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Failed to parse `cpu MHz` in /proc/cpuinfo",
                    );
                    return None;
                }
                let base_frequency = base_frequency.unwrap();
                if base_frequency.is_err() {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Failed to parse `cpu MHz` in /proc/cpuinfo: {}",
                        base_frequency.err().unwrap()
                    );
                    return None;
                }

                Some((base_frequency.unwrap() * 1000.).round() as u64)
            }
        }
    }

    fn virtualization() -> Option<bool> {
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

        let mut signature_reg = [0u32; 3];
        let res = cpuid_ex::<1, 4>(0, &mut signature_reg);
        if res.is_none() {
            return None;
        }

        let mut features = [0_u32];
        cpuid_ex::<2, 3>(1, &mut features);

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

    unsafe fn virtual_machine() -> Option<bool> {
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

    fn cache_info() -> std::collections::HashMap<u8, usize> {
        use gtk::glib::*;
        use std::{fs::*, io::*};

        let mut result = std::collections::HashMap::new();

        let mut buf = String::new();
        let entries = match read_dir("/sys/devices/system/cpu/cpu0/cache/") {
            Ok(entries) => entries,
            Err(e) => {
                g_critical!("MissionCenter::CpuInfo", "Could not read cache info: {}", e);
                return result;
            }
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(e) => {
                    g_critical!("MissionCenter::CpuInfo", "Could not read cache info: {}", e);
                    continue;
                }
            };

            if !file_type.is_dir() {
                continue;
            }

            let mut file = match File::open(entry.path().join("level")) {
                Ok(file) => file,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Could not read cache level: {}",
                        e
                    );
                    continue;
                }
            };

            buf.clear();
            match file.read_to_string(&mut buf) {
                Ok(_) => (),
                Err(e) => {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Could not read cache level: {}",
                        e
                    );
                    continue;
                }
            }
            let level = match buf.trim().parse::<u8>() {
                Ok(level) => level,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Could not read cache level: {}",
                        e
                    );
                    continue;
                }
            };

            let mut file = match File::open(entry.path().join("size")) {
                Ok(file) => file,
                Err(e) => {
                    g_critical!("MissionCenter::CpuInfo", "Could not read cache size: {}", e);
                    continue;
                }
            };

            buf.clear();
            match file.read_to_string(&mut buf) {
                Ok(_) => (),
                Err(e) => {
                    g_critical!("MissionCenter::CpuInfo", "Could not read cache size: {}", e);
                    continue;
                }
            }
            let size = match buf.trim().trim_matches('K').parse::<usize>() {
                Ok(level) => level,
                Err(e) => {
                    g_critical!("MissionCenter::CpuInfo", "Could not read cache size: {}", e);
                    continue;
                }
            };

            if let Some(old_size) = result.get_mut(&level) {
                *old_size += size;
            } else {
                result.insert(level, size);
            }
        }

        for (_, size) in result.iter_mut() {
            *size *= 1024;
        }

        result
    }
}

#[derive(Debug, Clone)]
pub struct DynamicInfo {
    pub utilization_percent: f32,
    pub utilization_percent_per_core: Vec<f32>,
    pub current_frequency_mhz: u64,
    pub process_count: u32,
    pub thread_count: u32,
    pub handle_count: u32,
    pub uptime_seconds: u64,
}

unsafe impl Send for DynamicInfo {}

impl DynamicInfo {
    pub fn load(system: &mut sysinfo::System) -> Self {
        use sysinfo::*;

        system.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage().with_frequency());

        let logical_core_count = num_cpus::get();
        let mut utilization_percent_per_core = vec![];
        utilization_percent_per_core.reserve(logical_core_count);
        for cpu in system.cpus() {
            utilization_percent_per_core.push(cpu.cpu_usage());
        }

        Self {
            utilization_percent: system.global_cpu_info().cpu_usage(),
            utilization_percent_per_core,
            current_frequency_mhz: system.global_cpu_info().frequency(),
            process_count: Self::process_count(),
            thread_count: Self::thread_count(),
            handle_count: Self::handle_count(),
            uptime_seconds: system.uptime(),
        }
    }

    fn process_count() -> u32 {
        use gtk::glib::*;

        let mut cmd = cmd!("ls -d /proc/[1-9]* | wc -l");

        if let Ok(output) = cmd.output() {
            std::str::from_utf8(output.stdout.as_slice()).map_or(0, |s| match s.trim().parse() {
                Ok(count) => count,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "Failed to get process count, host command output ({}) parsing failed: {}",
                        s,
                        e
                    );
                    0
                }
            })
        } else {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to get process count, host command execution failed"
            );

            return 0;
        }
    }

    fn thread_count() -> u32 {
        use gtk::glib::*;

        // https://askubuntu.com/questions/88972/how-to-get-from-terminal-total-number-of-threads-per-process-and-total-for-al
        let mut cmd = cmd!("count() { printf %s\\\\n \"$#\" ; } ; count /proc/[0-9]*/task/[0-9]*");

        if let Ok(output) = cmd.output() {
            if output.stderr.len() > 0 {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to get thread count, host command execution failed: {}",
                    String::from_utf8_lossy(output.stderr.as_slice())
                );
                return 0;
            }

            std::str::from_utf8(output.stdout.as_slice()).map_or(0, |s| match s.trim().parse() {
                Ok(count) => count,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "Failed to get thread count, host command output ({}) parsing: {}",
                        s,
                        e
                    );
                    0
                }
            })
        } else {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to get thread count, host command execution failed"
            );

            0
        }
    }

    fn handle_count() -> u32 {
        use gtk::glib::*;

        let mut cmd = cmd!("cat /proc/sys/fs/file-nr");

        if let Ok(output) = cmd.output() {
            if output.stderr.len() > 0 {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to get handle count, host command execution failed: {}",
                    String::from_utf8_lossy(output.stderr.as_slice())
                );
                return 0;
            }

            std::str::from_utf8(output.stdout.as_slice()).map_or(0, |s| {
                let s = match s.split_whitespace()
                    .next() {
                    Some(s) => s,
                    None => {
                        g_critical!(
                                "MissionCenter::SysInfo",
                                "Failed to get handle count, host command output ({}) empty or parsing failed",
                                s
                            );
                        return 0;
                    }
                };

                match s.trim().parse() {
                    Ok(count) => count,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Failed to get handle count, host command output ({}) parsing failed: {}",
                            s,
                            e
                        );
                        0
                    }
                }
            })
        } else {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to get handle count, host command execution failed"
            );
            0
        }
    }
}

#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub static_info: StaticInfo,
    pub dynamic_info: DynamicInfo,
}

impl CpuInfo {
    pub fn new(system: &mut sysinfo::System) -> Self {
        let static_info = StaticInfo::load(system);
        let dynamic_info = DynamicInfo::load(system);

        Self {
            static_info,
            dynamic_info,
        }
    }
}
