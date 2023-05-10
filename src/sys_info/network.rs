use gtk::glib;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NetDeviceType {
    Wired,
    Wireless,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDeviceDescriptor {
    pub r#type: NetDeviceType,
    pub if_name: String,
    pub adapter_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub hw_address: Option<[u8; 6]>,
    pub ip4_address: Option<u32>,
    pub ip6_address: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WirelessInfo {
    pub ssid: Option<String>,
    pub frequency: Option<u32>,
    pub bitrate: Option<u32>,
    pub signal_strength: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDevice {
    pub descriptor: NetworkDeviceDescriptor,
    pub address: Address,
    pub wireless_info: Option<WirelessInfo>,
}

pub struct NetInfo {
    udev: *mut libudev_sys::udev,
    nm_proxy: *mut gtk::gio::ffi::GDBusProxy,
}

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
        use gtk::glib::{ffi::*, translate::from_glib_full, Error};
        use libudev_sys::*;

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

        Ok(Self { udev, nm_proxy })
    }

    pub fn load_devices<
        'a,
        DeviceIf: Into<&'a str>,
        InterfaceNames: IntoIterator<Item = DeviceIf>,
    >(
        &self,
        devices: InterfaceNames,
    ) -> Vec<Option<NetworkDevice>> {
        use gtk::glib::gobject_ffi::*;

        let mut result = vec![];
        for device_if in devices {
            let if_name = device_if.into();

            if let Some(device_path) = unsafe { self.nm_obj_path_new(if_name) } {
                let device_proxy = unsafe {
                    Self::create_nm_dbus_proxy(
                        device_path.as_bytes_with_nul(),
                        b"org.freedesktop.NetworkManager.Device\0",
                    )
                };
                if device_proxy.is_null() {
                    result.push(None);
                    continue;
                }

                let r#type = Self::device_type(if_name);
                let adapter_name = unsafe { self.adapter_name(device_proxy) };
                let hw_address = Self::hw_address(device_proxy);
                let ip4_address = unsafe { Self::ip4_address(device_proxy) };
                let ip6_address = unsafe { Self::ip6_address(device_proxy) };

                unsafe { g_object_unref(device_proxy as _) };

                let descriptor = NetworkDeviceDescriptor {
                    r#type,
                    if_name: if_name.to_owned(),
                    adapter_name,
                };
                dbg!(descriptor);

                let address = Address {
                    hw_address,
                    ip4_address,
                    ip6_address,
                };
                dbg!(address);

                let wireless_info = if r#type == NetDeviceType::Wireless {
                    unsafe { Self::wireless_info(device_proxy) }
                } else {
                    None
                };
                dbg!(wireless_info);
            }
        }

        result
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
                    frequency,
                    bitrate,
                    signal_strength,
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
                let udev_device = udev_device_new_from_syspath(self.udev, udi.as_ptr() as _);
                if udev_device.is_null() {
                    let err = *errno_location();
                    let error_message = CStr::from_ptr(strerror(err))
                        .to_str()
                        .map_or("Unknown error", |s| s)
                        .to_owned();

                    g_error!(
                        "MissionCenter::NetInfo",
                        "Failed to create udev device from {:?}. {}",
                        udi,
                        error_message
                    );
                    return None;
                }

                let mut dev_name = Self::get_udev_property(
                    udev_device,
                    b"ID_MODEL_ENC\0".as_ptr() as _,
                    b"ID_MODEL_FROM_DATABASE\0".as_ptr() as _,
                );
                if dev_name.is_none() {
                    dev_name = Self::get_udev_property(
                        udev_device,
                        b"ID_MODEL_ENC\0".as_ptr() as _,
                        b"ID_PRODUCT_FROM_DATABASE\0".as_ptr() as _,
                    );
                }

                udev_device_unref(udev_device);

                dev_name
            } else {
                g_error!(
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
                g_error!(
                    "MissionCenter::NetInfo",
                    "Failed to create dbus proxy for interface '{:?}': {}",
                    CStr::from_ptr(interface.as_ptr() as _),
                    error.message()
                );
            } else {
                g_error!(
                    "MissionCenter::NetInfo",
                    "Failed to create dbus proxy for interface '{:?}': Unknown error",
                    CStr::from_ptr(interface.as_ptr() as _),
                );
            }
        }

        proxy
    }

    unsafe fn nm_obj_path_new(&self, device_if: &str) -> Option<std::ffi::CString> {
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
                let error: Error = unsafe { from_glib_full(error) };
                g_error!(
                    "MissionCenter::NetInfo",
                    "Failed to get device info: {}",
                    error.message()
                );

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
                g_error!(
                    "MissionCenter::NetInfo",
                    "Failed to get property {:?}: {}",
                    CStr::from_ptr(property.as_ptr() as _),
                    error.message()
                );
            } else {
                g_error!(
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

            g_error!(
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
        enc_prop: *const i8,
        db_prop: *const i8,
    ) -> Option<String> {
        use libudev_sys::*;
        use std::ffi::CStr;

        let mut enc_value: *const i8 = std::ptr::null_mut();
        let mut db_value: *const i8 = std::ptr::null_mut();
        let mut tmpdev: *mut udev_device = device;

        let mut count = 0;
        while (count < 3) && !tmpdev.is_null() && enc_value.is_null() {
            count += 1;

            if enc_value.is_null() {
                enc_value = udev_device_get_property_value(tmpdev, enc_prop);
            }

            if db_value.is_null() {
                db_value = udev_device_get_property_value(tmpdev, db_prop);
            }

            tmpdev = udev_device_get_parent(tmpdev);
        }

        if !db_value.is_null() {
            CStr::from_ptr(db_value)
                .to_str()
                .map_or(None, |s| Some(s.to_owned()))
        } else if !enc_value.is_null() {
            CStr::from_ptr(enc_value)
                .to_str()
                .map_or(None, |s| Some(s.to_owned()))
        } else {
            None
        }
    }
}
