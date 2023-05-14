use gtk::glib;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum NetDeviceType {
    Wired = 0,
    Wireless = 1,
    Other = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDeviceDescriptor {
    pub r#type: NetDeviceType,
    pub if_name: String,
    pub adapter_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkAddress {
    pub hw_address: Option<[u8; 6]>,
    pub ip4_address: Option<u32>,
    pub ip6_address: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WirelessInfo {
    pub ssid: Option<String>,
    pub frequency_mhz: Option<u32>,
    pub bitrate_kbps: Option<u32>,
    pub signal_strength_percent: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDevice {
    pub descriptor: NetworkDeviceDescriptor,
    pub address: NetworkAddress,
    pub wireless_info: Option<WirelessInfo>,

    pub bytes_sent: u64,
    pub bytes_received: u64,
}

pub struct NetInfo {
    udev: *mut libudev_sys::udev,
    nm_proxy: *mut gtk::gio::ffi::GDBusProxy,

    hwdb_conn: Option<rusqlite::Connection>,
    device_name_cache: std::cell::Cell<std::collections::HashMap<String, String>>,
}

unsafe impl Send for NetInfo {}

unsafe impl Sync for NetInfo {}

#[derive(Debug, Copy, Clone, glib::ErrorDomain)]
#[error_domain(name = "Udev")]
enum NetInfoError {
    NetworkManagerInitializationError,
    UdevInitializationError,
}

impl Drop for NetInfo {
    fn drop(&mut self) {
        use gtk::glib::gobject_ffi::*;
        use libudev_sys::*;

        unsafe {
            if self.nm_proxy != std::ptr::null_mut() {
                g_object_unref(self.nm_proxy as _);
            }

            if self.udev != std::ptr::null_mut() {
                udev_unref(self.udev);
            }
        }
    }
}

impl NetInfo {
    pub fn new() -> Result<Self, glib::Error> {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, translate::from_glib_full, Error, *};
        use libudev_sys::*;
        use std::{cell::*, collections::*, path::*};

        let mut error: *mut GError = std::ptr::null_mut();

        let nm_proxy = unsafe {
            g_dbus_proxy_new_for_bus_sync(
                G_BUS_TYPE_SYSTEM,
                G_DBUS_PROXY_FLAGS_NONE,
                std::ptr::null_mut(),
                b"org.freedesktop.NetworkManager\0".as_ptr() as _,
                b"/org/freedesktop/NetworkManager\0".as_ptr() as _,
                b"org.freedesktop.NetworkManager\0".as_ptr() as _,
                std::ptr::null_mut(),
                &mut error,
            )
        };
        if nm_proxy.is_null() {
            if !error.is_null() {
                let error = unsafe { from_glib_full(error) };
                return Err(error);
            }
            return Err(Error::new::<NetInfoError>(
                NetInfoError::NetworkManagerInitializationError,
                "Failed to create NetworkManager proxy",
            ));
        }

        let udev = unsafe { udev_new() };
        if nm_proxy == std::ptr::null_mut() {
            let error = Error::new::<NetInfoError>(
                NetInfoError::UdevInitializationError,
                "Failed to create udev context",
            );
            return Err(error);
        }

        let conn = if let Ok(conn) =
            rusqlite::Connection::open(Path::new(crate::HW_DB_DIR.as_str()).join("hw.db"))
        {
            Some(conn)
        } else {
            g_warning!(
                "MissionCenter::NetInfo",
                "Failed to load hadrware database, network devices will (probably) have missing names",
            );

            None
        };

        Ok(Self {
            udev,
            nm_proxy,
            hwdb_conn: conn,
            device_name_cache: Cell::new(HashMap::new()),
        })
    }

    pub fn load_device<'a, DeviceIfName: Into<&'a str>>(
        &self,
        device: DeviceIfName,
    ) -> Option<NetworkDevice> {
        use gtk::glib::gobject_ffi::*;

        let if_name = device.into();

        if let Some(device_path) = unsafe { self.nm_device_obj_path_new(if_name) } {
            let device_proxy = unsafe {
                Self::create_nm_dbus_proxy(
                    device_path.as_bytes_with_nul(),
                    b"org.freedesktop.NetworkManager.Device\0",
                )
            };
            if device_proxy.is_null() {
                return None;
            }

            let r#type = Self::device_type(if_name);
            let adapter_name = unsafe { self.adapter_name(device_proxy) };
            let hw_address = Self::hw_address(device_proxy);
            let ip4_address = unsafe { Self::ip4_address(device_proxy) };
            let ip6_address = unsafe { Self::ip6_address(device_proxy) };

            let descriptor = NetworkDeviceDescriptor {
                r#type,
                if_name: if_name.to_owned(),
                adapter_name,
            };

            let address = NetworkAddress {
                hw_address,
                ip4_address,
                ip6_address,
            };

            let wireless_info = if r#type == NetDeviceType::Wireless {
                unsafe { Self::wireless_info(device_proxy) }
            } else {
                None
            };

            unsafe { g_object_unref(device_proxy as _) };

            Some(NetworkDevice {
                descriptor,
                address,
                wireless_info,

                bytes_sent: 0,
                bytes_received: 0,
            })
        } else {
            None
        }
    }

    fn device_type(device_if: &str) -> NetDeviceType {
        if device_if.starts_with("en") {
            NetDeviceType::Wired
        } else if device_if.starts_with("wl") {
            NetDeviceType::Wireless
        } else {
            NetDeviceType::Other
        }
    }

    fn hw_address(dbus_proxy: *mut gtk::gio::ffi::GDBusProxy) -> Option<[u8; 6]> {
        if let Some(hw_address_variant) =
            unsafe { Self::nm_device_property(dbus_proxy, b"HwAddress\0") }
        {
            if let Some(hw_address_str) = hw_address_variant.str() {
                let mut hw_address = [0; 6];

                hw_address_str
                    .split(':')
                    .take(6)
                    .enumerate()
                    .map(|(i, s)| (i, u8::from_str_radix(s, 16).map_or(0, |v| v)))
                    .for_each(|(i, v)| hw_address[i] = v);

                Some(hw_address)
            } else {
                None
            }
        } else {
            None
        }
    }

    unsafe fn ip4_address(dbus_proxy: *mut gtk::gio::ffi::GDBusProxy) -> Option<u32> {
        use gtk::glib::gobject_ffi::*;

        if let Some(ip4_address_obj_path) = Self::nm_device_property(dbus_proxy, b"Ip4Config\0") {
            if let Some(ip4_address_obj_path_str) = ip4_address_obj_path.str() {
                let ip4_config_proxy = Self::create_nm_dbus_proxy(
                    ip4_address_obj_path_str.as_bytes(),
                    b"org.freedesktop.NetworkManager.IP4Config\0",
                );
                if ip4_config_proxy.is_null() {
                    return None;
                }

                let result = if let Some(ip4_address_variant) =
                    Self::nm_device_property(ip4_config_proxy, b"Addresses\0")
                {
                    // Just take the first entry in the list of lists
                    if let Some(ip4_address_info) = ip4_address_variant.iter().next() {
                        // The first entry in the inner list is the IP address
                        ip4_address_info
                            .iter()
                            .next()
                            .map_or(None, |v| v.get::<u32>())
                    } else {
                        None
                    }
                } else {
                    None
                };

                g_object_unref(ip4_config_proxy as _);

                result
            } else {
                None
            }
        } else {
            None
        }
    }

    unsafe fn ip6_address(dbus_proxy: *mut gtk::gio::ffi::GDBusProxy) -> Option<u128> {
        use gtk::glib::gobject_ffi::*;

        if let Some(ip6_address_obj_path) = Self::nm_device_property(dbus_proxy, b"Ip6Config\0") {
            if let Some(ip6_address_obj_path_str) = ip6_address_obj_path.str() {
                let ip6_config_proxy = Self::create_nm_dbus_proxy(
                    ip6_address_obj_path_str.as_bytes(),
                    b"org.freedesktop.NetworkManager.IP6Config\0",
                );
                if ip6_config_proxy.is_null() {
                    return None;
                }

                let result = if let Some(ip6_address_variant) =
                    Self::nm_device_property(ip6_config_proxy, b"Addresses\0")
                {
                    // Just take the first entry in the list of lists
                    if let Some(ip6_address_info) = ip6_address_variant.iter().next() {
                        ip6_address_info.iter().next().map_or(None, |v| {
                            let mut ip6_address = [0; 16];
                            v.iter().enumerate().for_each(|(i, v)| {
                                ip6_address[i] = v.get::<u8>().unwrap_or(0);
                            });

                            Some(u128::from_be_bytes(ip6_address))
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };

                g_object_unref(ip6_config_proxy as _);

                result
            } else {
                None
            }
        } else {
            None
        }
    }

    unsafe fn wireless_info(dbus_proxy: *mut gtk::gio::ffi::GDBusProxy) -> Option<WirelessInfo> {
        use gtk::{gio::ffi::*, glib::gobject_ffi::*};

        use std::ffi::CStr;

        let wireless_obj_path = CStr::from_ptr(g_dbus_proxy_get_object_path(dbus_proxy));

        let wireless_info_proxy = Self::create_nm_dbus_proxy(
            wireless_obj_path.to_bytes_with_nul(),
            b"org.freedesktop.NetworkManager.Device.Wireless\0",
        );
        if wireless_info_proxy.is_null() {
            return None;
        }

        let result = if let Some(wireless_info_variant) =
            Self::nm_device_property(wireless_info_proxy, b"ActiveAccessPoint\0")
        {
            if let Some(wireless_info_obj_path) = wireless_info_variant.str() {
                let wireless_info_proxy = Self::create_nm_dbus_proxy(
                    wireless_info_obj_path.as_bytes(),
                    b"org.freedesktop.NetworkManager.AccessPoint\0",
                );
                if wireless_info_proxy.is_null() {
                    return None;
                }

                let ssid = if let Some(ssid_variant) =
                    Self::nm_device_property(wireless_info_proxy, b"Ssid\0")
                {
                    let ssid = ssid_variant
                        .iter()
                        .filter_map(|v| v.get::<u8>())
                        .collect::<Vec<_>>();

                    String::from_utf8(ssid).ok()
                } else {
                    None
                };

                let frequency = if let Some(frequency) =
                    Self::nm_device_property(wireless_info_proxy, b"Frequency\0")
                {
                    frequency.get::<u32>()
                } else {
                    None
                };

                let bitrate = if let Some(bitrate) =
                    Self::nm_device_property(wireless_info_proxy, b"MaxBitrate\0")
                {
                    bitrate.get::<u32>()
                } else {
                    None
                };

                let signal_strength = if let Some(signal_strength) =
                    Self::nm_device_property(wireless_info_proxy, b"Strength\0")
                {
                    signal_strength.get::<u8>()
                } else {
                    None
                };

                g_object_unref(wireless_info_proxy as _);
                Some(WirelessInfo {
                    ssid,
                    frequency_mhz: frequency,
                    bitrate_kbps: bitrate,
                    signal_strength_percent: signal_strength,
                })
            } else {
                None
            }
        } else {
            None
        };

        g_object_unref(wireless_info_proxy as _);

        result
    }

    fn device_name_from_hw_db(&self, udi: &str) -> Option<String> {
        use gtk::glib::*;
        use std::{fs::*, io::*, path::*};

        if self.hwdb_conn.is_none() {
            return None;
        }

        let device_name_cache = self.device_name_cache.take();
        if let Some(device_name) = device_name_cache.get(udi) {
            let device_name = device_name.clone();
            self.device_name_cache.set(device_name_cache);

            return Some(device_name.clone());
        }
        self.device_name_cache.set(device_name_cache);

        let conn = self.hwdb_conn.as_ref().unwrap();

        let stmt = conn.prepare("SELECT value FROM key_len WHERE key = 'min'");
        if stmt.is_err() {
            g_critical!(
                "MissionCenter::NetInfo",
                "Failed to extract min key length from {}/hw.db: Prepare query failed",
                crate::HW_DB_DIR.as_str()
            );
            return None;
        }
        let mut stmt = stmt.unwrap();
        let query_result = stmt.query_map([], |row| row.get::<usize, i32>(0));
        if query_result.is_err() {
            g_critical!(
                "MissionCenter::NetInfo",
                "Failed to extract min key length from {}/hw.db: Query map failed",
                crate::HW_DB_DIR.as_str()
            );
            return None;
        }
        let min_key_len = if let Some(min_len) = query_result.unwrap().next() {
            min_len.unwrap_or(0)
        } else {
            0
        };

        let stmt = conn.prepare("SELECT value FROM key_len WHERE key = 'max'");
        if stmt.is_err() {
            g_critical!(
                "MissionCenter::NetInfo",
                "Failed to extract max key length from {}/hw.db: Prepare query failed",
                crate::HW_DB_DIR.as_str()
            );
            return None;
        }
        let mut stmt = stmt.unwrap();
        let query_result = stmt.query_map([], |row| row.get::<usize, i32>(0));
        if query_result.is_err() {
            g_critical!(
                "MissionCenter::NetInfo",
                "Failed to extract max key length from {}/hw.db: Query map failed",
                crate::HW_DB_DIR.as_str()
            );
            return None;
        }
        let mut max_key_len = if let Some(max_len) = query_result.unwrap().next() {
            max_len.unwrap_or(i32::MAX)
        } else {
            i32::MAX
        };

        let device_id = format!("{}/device", udi);
        let mut sys_device_path = Path::new(&device_id);
        let mut modalias = String::new();
        for _ in 0..4 {
            if let Some(p) = sys_device_path.parent() {
                sys_device_path = p;
            } else {
                break;
            }

            let modalias_path = sys_device_path.join("modalias");
            if modalias_path.exists() {
                if let Ok(mut modalias_file) = File::options()
                    .create(false)
                    .read(true)
                    .write(false)
                    .open(modalias_path)
                {
                    modalias.clear();

                    if let Ok(_) = modalias_file.read_to_string(&mut modalias) {
                        modalias = modalias.trim().to_owned();
                        if max_key_len == i32::MAX {
                            max_key_len = modalias.len() as i32;
                        }

                        for i in (min_key_len..max_key_len).rev() {
                            modalias.truncate(i as usize);
                            let stmt = conn.prepare(
                                "SELECT value FROM models WHERE key LIKE ?1 || '%' LIMIT 1",
                            );
                            if stmt.is_err() {
                                g_warning!(
                                    "MissionCenter::NetInfo",
                                    "Failed to find model in {}/hw.db: Prepare query failed",
                                    crate::HW_DB_DIR.as_str()
                                );
                                continue;
                            }
                            let mut stmt = stmt.unwrap();
                            let query_result = stmt
                                .query_map([modalias.trim()], |row| row.get::<usize, String>(0));
                            if query_result.is_err() {
                                g_warning!(
                                    "MissionCenter::NetInfo",
                                    "Failed to find model in {}/hw.db: Query map failed",
                                    crate::HW_DB_DIR.as_str()
                                );
                                continue;
                            }

                            let model_name = if let Some(model) = query_result.unwrap().next() {
                                model.ok()
                            } else {
                                None
                            };

                            if let Some(model_name) = model_name {
                                let mut device_name_cache = self.device_name_cache.take();
                                device_name_cache.insert(udi.to_owned(), model_name.clone());
                                self.device_name_cache.set(device_name_cache);

                                return Some(model_name);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    unsafe fn adapter_name(&self, dbus_proxy: *mut gtk::gio::ffi::GDBusProxy) -> Option<String> {
        extern "C" {
            fn strerror(error: i32) -> *const i8;
        }

        use errno_sys::errno_location;
        use gtk::glib::*;
        use libudev_sys::*;

        use std::ffi::CStr;

        if let Some(udi_variant) = Self::nm_device_property(dbus_proxy, b"Udi\0") {
            if let Some(udi) = udi_variant.str() {
                if let Some(device_name) = self.device_name_from_hw_db(udi) {
                    return Some(device_name);
                }

                let udev_device = udev_device_new_from_syspath(self.udev, udi.as_ptr() as _);
                if udev_device.is_null() {
                    let err = *errno_location();
                    let error_message = CStr::from_ptr(strerror(err))
                        .to_str()
                        .map_or("Unknown error", |s| s)
                        .to_owned();

                    g_critical!(
                        "MissionCenter::NetInfo",
                        "Failed to create udev device from {:?}. {}",
                        udi,
                        error_message
                    );
                    return None;
                }

                let dev_name =
                    Self::get_udev_property(udev_device, b"ID_MODEL_ENC\0".as_ptr() as _);

                udev_device_unref(udev_device);

                dev_name
            } else {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to get udev device path, cannot extract device sys path from variant: Unknown error"
                );
                None
            }
        } else {
            None
        }
    }

    unsafe fn create_nm_dbus_proxy(
        path: &[u8],
        interface: &[u8],
    ) -> *mut gtk::gio::ffi::GDBusProxy {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, translate::from_glib_full, Error, *};
        use std::ffi::CStr;

        let mut error: *mut GError = std::ptr::null_mut();

        let proxy = g_dbus_proxy_new_for_bus_sync(
            G_BUS_TYPE_SYSTEM,
            G_DBUS_PROXY_FLAGS_NONE,
            std::ptr::null_mut(),
            b"org.freedesktop.NetworkManager\0".as_ptr() as _,
            path.as_ptr() as _,
            interface.as_ptr() as _,
            std::ptr::null_mut(),
            &mut error,
        );
        if proxy.is_null() {
            if !error.is_null() {
                let error: Error = from_glib_full(error);
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to create dbus proxy for interface '{:?}': {}",
                    CStr::from_ptr(interface.as_ptr() as _),
                    error.message()
                );
            } else {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to create dbus proxy for interface '{:?}': Unknown error",
                    CStr::from_ptr(interface.as_ptr() as _),
                );
            }
        }

        proxy
    }

    unsafe fn nm_device_obj_path_new(&self, device_if: &str) -> Option<std::ffi::CString> {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, translate::from_glib_full, Error, *};
        use std::ffi::{CStr, CString};

        if let Ok(device_name) = CString::new(device_if) {
            let mut error: *mut GError = std::ptr::null_mut();

            let device_path_variant = unsafe {
                g_dbus_proxy_call_sync(
                    self.nm_proxy,
                    b"GetDeviceByIpIface\0".as_ptr() as _,
                    g_variant_new(b"(s)\0".as_ptr() as _, device_name.as_c_str().as_ptr()),
                    G_DBUS_CALL_FLAGS_NONE,
                    -1,
                    std::ptr::null_mut(),
                    &mut error,
                )
            };
            if device_path_variant.is_null() {
                if !error.is_null() {
                    let error: Error = unsafe { from_glib_full(error) };
                    g_critical!(
                        "MissionCenter::NetInfo",
                        "Failed to get device info for {:?}: {}",
                        device_if,
                        error.message()
                    );
                } else {
                    g_critical!(
                        "MissionCenter::NetInfo",
                        "Failed to get device info for {:?}: Unknown error",
                        device_if,
                    );
                }

                return None;
            }

            let mut device_path: *mut i8 = std::ptr::null_mut();
            unsafe {
                g_variant_get(
                    device_path_variant,
                    b"(&o)\0".as_ptr() as _,
                    &mut device_path,
                )
            };
            if device_path.is_null() {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to get device info for {:?}: Variant error",
                    device_if,
                );
                return None;
            }

            let device_path = CStr::from_ptr(device_path).to_owned();
            let _: Variant = from_glib_full(device_path_variant);

            Some(device_path)
        } else {
            None
        }
    }

    unsafe fn nm_device_property(
        dbus_proxy: *mut gtk::gio::ffi::GDBusProxy,
        property: &[u8],
    ) -> Option<gtk::glib::Variant> {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, translate::from_glib_full, Error, *};
        use std::ffi::CStr;

        let mut error: *mut GError = std::ptr::null_mut();

        let variant = g_dbus_proxy_call_sync(
            dbus_proxy,
            b"org.freedesktop.DBus.Properties.Get\0".as_ptr() as _,
            g_variant_new(
                b"(ss)\0".as_ptr() as _,
                g_dbus_proxy_get_interface_name(dbus_proxy),
                property.as_ptr() as *const i8,
            ),
            G_DBUS_CALL_FLAGS_NONE,
            -1,
            std::ptr::null_mut(),
            &mut error,
        );
        if variant.is_null() {
            if !error.is_null() {
                let error: Error = from_glib_full(error);
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to get property {:?}: {}",
                    CStr::from_ptr(property.as_ptr() as _),
                    error.message()
                );
            } else {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to get property {:?}: Unknown error",
                    CStr::from_ptr(property.as_ptr() as _),
                );
            }

            return None;
        }

        let mut inner: *mut GVariant = std::ptr::null_mut();
        g_variant_get(variant, b"(v)\0".as_ptr() as _, &mut inner);
        if inner.is_null() {
            g_variant_unref(variant);

            g_critical!(
                "MissionCenter::NetInfo",
                "Failed to get property {:?}, cannot extract inner variant: Unknown error",
                CStr::from_ptr(property.as_ptr() as _),
            );

            return None;
        }

        g_variant_ref_sink(inner);
        g_variant_unref(variant);

        from_glib_full(inner)
    }

    // Yanked from NetworkManager: src/libnm-client-impl/nm-device.c: _get_udev_property()
    unsafe fn get_udev_property(
        device: *mut libudev_sys::udev_device,
        property: *const i8,
    ) -> Option<String> {
        use libudev_sys::*;
        use std::ffi::CStr;

        let mut value: *const i8 = std::ptr::null_mut();
        let mut tmpdev: *mut udev_device = device;

        let mut count = 0;
        while (count < 3) && !tmpdev.is_null() && value.is_null() {
            count += 1;

            if value.is_null() {
                value = udev_device_get_property_value(tmpdev, property);
            }

            tmpdev = udev_device_get_parent(tmpdev);
        }

        if !value.is_null() {
            CStr::from_ptr(value)
                .to_str()
                .map_or(None, |s| Some(s.to_owned()))
        } else {
            None
        }
    }
}
