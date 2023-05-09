use sysinfo::{System, SystemExt};

use crate::sys_info::network::NetInfo;

mod network;

pub struct SysInfo {
    system: System,

    cpu_base_frequency: Option<usize>,
    cpu_socket_count: Option<u8>,
    cpu_caches: std::collections::HashMap<u8, usize>,
    is_vm: Option<bool>,

    file_nr: Option<std::fs::File>,
    thread_count: usize,
    handle_count: usize,
}

impl SysInfo {
    pub fn new() -> Self {
        use network::*;

        let file_nr = std::fs::OpenOptions::new()
            .read(true)
            .open("/proc/sys/fs/file-nr")
            .ok();

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

        let net = NetInfo::new().unwrap();
        let _bla = net.load_net_info(["enp6s18"]);

        Self {
            system: System::new_all(),

            cpu_base_frequency,
            cpu_socket_count: Self::load_cpu_socket_count(),
            is_vm: unsafe { Self::load_is_vm() },
            cpu_caches: Self::load_cpu_cache_info(),

            file_nr,
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

    pub fn is_vm(&self) -> Option<bool> {
        self.is_vm
    }

    pub fn cpu_cache_size(&self, level: u8) -> Option<usize> {
        self.cpu_caches.get(&level).copied()
    }

    pub fn thread_count(&self) -> usize {
        self.thread_count
    }

    pub fn handle_count(&self) -> usize {
        self.handle_count
    }

    pub fn refresh_all(&mut self) {
        self.system.refresh_all();
        self.refresh_thread_count();
        self.refresh_handle_count();
    }

    pub fn refresh_components_list(&mut self) {
        self.system.refresh_components_list();
    }

    pub fn refresh_thread_count(&mut self) {
        self.thread_count = 0;
        for (pid, _) in self.system.processes() {
            if let Ok(entries) = std::fs::read_dir(format!("/proc/{}/task", pid)) {
                self.thread_count += entries.count();
            }
        }
    }

    pub fn refresh_handle_count(&mut self) {
        if let Some(file_nr) = &mut self.file_nr {
            use std::io::*;

            let mut buf = String::new();
            file_nr.read_to_string(&mut buf).unwrap();

            let mut split = buf.split_whitespace();
            if let Some(handle_count) = split.next() {
                self.handle_count = if let Ok(handle_count) = handle_count.parse() {
                    handle_count
                } else {
                    0
                }
            }

            file_nr.seek(SeekFrom::Start(0)).unwrap();
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
                                file.read_to_string(&mut buf).unwrap();
                                if let Ok(socket_id) = buf.trim().parse::<u8>() {
                                    sockets.insert(socket_id);
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

        if systemd_proxy == std::ptr::null_mut() {
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
        if variant == std::ptr::null_mut() {
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
                            file.read_to_string(&mut buf).unwrap();
                            if let Ok(level) = buf.trim().parse::<u8>() {
                                Some(level)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        let size = if let Ok(mut file) = File::open(entry.path().join("size")) {
                            buf.clear();
                            file.read_to_string(&mut buf).unwrap();
                            if let Ok(size) = buf.trim().trim_matches('K').parse::<usize>() {
                                Some(size)
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
}
