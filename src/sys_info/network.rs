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
    pub ip6_address: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WirelessInfo {
    pub ssid: String,
    pub connection_type: String,
    pub signal_strength: u8,
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

            if let Some(device_path) = unsafe { self.nm_device_obj_path(if_name) } {
                let device_proxy = unsafe {
                    self.create_nm_dbus_proxy(
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

                unsafe { g_object_unref(device_proxy as _) };

                let descriptor = NetworkDeviceDescriptor {
                    r#type,
                    if_name: if_name.to_owned(),
                    adapter_name,
                };
                dbg!(descriptor);

                let address = Address {
                    hw_address,
                    ip4_address: None,
                    ip6_address: None,
                };
                dbg!(address);
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
            unsafe { Self::nm_device_get_property(dbus_proxy, b"HwAddress\0") }
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

    unsafe fn adapter_name(&self, dbus_proxy: *mut gtk::gio::ffi::GDBusProxy) -> Option<String> {
        extern "C" {
            fn strerror(error: i32) -> *const i8;
        }

        use errno_sys::errno_location;
        use gtk::glib::*;
        use libudev_sys::*;

        use std::ffi::CStr;

        if let Some(udi_variant) = Self::nm_device_get_property(dbus_proxy, b"Udi\0") {
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
        &self,
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

    unsafe fn nm_device_obj_path(&self, device_if: &str) -> Option<std::ffi::CString> {
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

    unsafe fn nm_device_get_property(
        dbus_proxy: *mut gtk::gio::ffi::GDBusProxy,
        property: &[u8],
    ) -> Option<gtk::glib::Variant> {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, translate::from_glib_full, Error, *};

        let mut error: *mut GError = std::ptr::null_mut();

        let udi_variant = g_dbus_proxy_call_sync(
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
        if udi_variant.is_null() {
            if !error.is_null() {
                let error: Error = from_glib_full(error);
                g_error!(
                    "MissionCenter::NetInfo",
                    "Failed to get udev device path: {}",
                    error.message()
                );
            } else {
                g_error!(
                    "MissionCenter::NetInfo",
                    "Failed to get udev device path: Unknown error"
                );
            }

            return None;
        }

        let mut udi_inner: *mut GVariant = std::ptr::null_mut();
        g_variant_get(udi_variant, b"(v)\0".as_ptr() as _, &mut udi_inner);
        if udi_inner.is_null() {
            g_variant_unref(udi_variant);

            g_error!(
                "MissionCenter::NetInfo",
                "Failed to get udev device path, cannot extract inner variant: Unknown error"
            );

            return None;
        }

        g_variant_ref_sink(udi_inner);
        g_variant_unref(udi_variant);

        from_glib_full(udi_inner)
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
