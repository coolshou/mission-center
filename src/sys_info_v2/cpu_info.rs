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
            l1_cache: cache_info[1],
            l2_cache: cache_info[2],
            l3_cache: cache_info[3],
            l4_cache: cache_info[4],
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
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        fn cpuid_ex<const START: u8, const END: u8>(leaf: u32, result: &mut [u32]) -> Option<()> {
            use raw_cpuid::*;

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

        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        fn cpuid_ex<const _START: u8, const _END: u8>(_: u32, _: &mut [u32]) -> Option<()> {
            None
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

    fn cache_info() -> [Option<usize>; 5] {
        use gtk::glib::*;
        use std::{collections::HashSet, fs::*, os::unix::prelude::*, str::FromStr};

        fn read_index_entry_content(
            file_name: &str,
            index_path: &std::path::Path,
        ) -> Option<String> {
            let path = index_path.join(file_name);
            let content = read_to_string(path);
            if content.is_err() {
                g_critical!(
                    "MissionCenter::CpuInfo",
                    "Could not read '{}/{}': {}",
                    index_path.display(),
                    file_name,
                    content.err().unwrap()
                );
                return None;
            }

            Some(content.unwrap())
        }

        fn read_index_entry_number<R: FromStr<Err = core::num::ParseIntError>>(
            file_name: &str,
            index_path: &std::path::Path,
            suffix: Option<&str>,
        ) -> Option<R> {
            let content = read_index_entry_content(file_name, index_path);
            if content.is_none() {
                return None;
            }
            let content = content.unwrap();
            let content = content.trim();
            let value = if suffix.is_none() {
                content.parse::<R>()
            } else {
                content.trim_end_matches(suffix.unwrap()).parse::<R>()
            };
            if value.is_err() {
                g_critical!(
                    "MissionCenter::CpuInfo",
                    "Failed to parse '{}/{}': {}",
                    index_path.display(),
                    file_name,
                    value.err().unwrap()
                );
                return None;
            }

            Some(value.unwrap())
        }

        let mut result = [None; 5];

        let numa_node_entries = match read_dir("/sys/devices/system/node/") {
            Ok(entries) => entries,
            Err(e) => {
                g_critical!(
                    "MissionCenter::CpuInfo",
                    "Could not read '/sys/devices/system/node': {}",
                    e
                );
                return result;
            }
        };

        for nn_entry in numa_node_entries {
            let nn_entry = match nn_entry {
                Ok(entry) => entry,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Could not read entry in '/sys/devices/system/node': {}",
                        e
                    );
                    continue;
                }
            };
            let path = nn_entry.path();
            if !path.is_dir() {
                continue;
            }

            let is_node = path
                .file_name()
                .map(|file| &file.as_bytes()[0..4] == b"node")
                .unwrap_or(false);
            if !is_node {
                continue;
            }

            let mut l1_visited_data = HashSet::new();
            let mut l1_visited_instr = HashSet::new();
            let mut l2_visited = HashSet::new();
            let mut l3_visited = HashSet::new();
            let mut l4_visited = HashSet::new();

            let cpu_entries = match path.read_dir() {
                Ok(entries) => entries,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Could not read '{}': {}",
                        path.display(),
                        e
                    );
                    return result;
                }
            };
            for cpu_entry in cpu_entries {
                let cpu_entry = match cpu_entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::CpuInfo",
                            "Could not read cpu entry in '{}': {}",
                            path.display(),
                            e
                        );
                        continue;
                    }
                };
                let mut path = cpu_entry.path();
                if !path.is_symlink() {
                    continue;
                }

                let cpu_name = path.file_name();
                if cpu_name.is_none() {
                    continue;
                }
                let cpu_name = cpu_name.unwrap();

                let is_cpu = &cpu_name.as_bytes()[0..3] == b"cpu";
                if is_cpu {
                    let cpu_number =
                        unsafe { std::str::from_utf8_unchecked(&cpu_name.as_bytes()[3..]) }
                            .parse::<u16>();
                    if cpu_number.is_err() {
                        continue;
                    }
                    let cpu_number = cpu_number.unwrap();

                    path.push("cache");
                    let cache_entries = match path.read_dir() {
                        Ok(entries) => entries,
                        Err(e) => {
                            g_critical!(
                                "MissionCenter::CpuInfo",
                                "Could not read '{}': {}",
                                path.display(),
                                e
                            );
                            return result;
                        }
                    };
                    for cache_entry in cache_entries {
                        let cache_entry = match cache_entry {
                            Ok(entry) => entry,
                            Err(e) => {
                                g_critical!(
                                    "MissionCenter::CpuInfo",
                                    "Could not read cpu entry in '{}': {}",
                                    path.display(),
                                    e
                                );
                                continue;
                            }
                        };
                        let path = cache_entry.path();
                        let is_cache_entry = path
                            .file_name()
                            .map(|file| &file.as_bytes()[0..5] == b"index")
                            .unwrap_or(false);
                        if is_cache_entry {
                            let level = read_index_entry_number::<u8>("level", &path, None);
                            if level.is_none() {
                                continue;
                            }
                            let level = level.unwrap();

                            let cache_type = read_index_entry_content("type", &path);
                            if cache_type.is_none() {
                                continue;
                            }
                            let cache_type = cache_type.unwrap();

                            let visited_cpus = match cache_type.trim() {
                                "Data" => &mut l1_visited_data,
                                "Instruction" => &mut l1_visited_instr,
                                "Unified" => match level {
                                    2 => &mut l2_visited,
                                    3 => &mut l3_visited,
                                    4 => &mut l4_visited,
                                    _ => continue,
                                },
                                _ => continue,
                            };

                            if visited_cpus.contains(&cpu_number) {
                                continue;
                            }

                            let size = read_index_entry_number::<usize>("size", &path, Some("K"));
                            if size.is_none() {
                                continue;
                            }
                            let size = size.unwrap();

                            let result_index = level as usize;
                            if result[result_index].is_none() {
                                result[result_index] = Some(size);
                            } else {
                                result[result_index] = Some(result[result_index].unwrap() + size);
                            }

                            let shared_cpu_list =
                                read_index_entry_content("shared_cpu_list", &path);
                            if shared_cpu_list.is_some() {
                                let shared_cpu_list = shared_cpu_list.unwrap();
                                let shared_cpu_list = shared_cpu_list.trim();

                                let shared_cpu_list = shared_cpu_list.split(',');
                                for cpu in shared_cpu_list {
                                    let mut shared_cpu_sequence = cpu.split('-');
                                    let start = shared_cpu_sequence.next();
                                    if start.is_some() {
                                        let start = start.unwrap().parse::<u16>().unwrap();

                                        let end = shared_cpu_sequence.next();
                                        if end.is_some() {
                                            let end = end.unwrap().parse::<u16>().unwrap();
                                            for i in start..=end {
                                                visited_cpus.insert(i);
                                            }
                                        } else {
                                            visited_cpus.insert(start);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for i in 1..result.len() {
            result[i] = result[i].map(|size| size * 1024);
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
