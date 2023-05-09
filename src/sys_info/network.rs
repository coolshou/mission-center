use gtk::glib;

pub enum NetDeviceType {
    Wired,
    Wireless,
}

pub struct WirelessInfo {
    pub ssid: String,
    pub connection_type: String,
    pub signal_strength: u8,
}

pub struct NetDevice {
    pub r#type: NetDeviceType,
    pub if_name: String,
    pub device_name: String,
    pub ip4_address: u32,
    pub ip6_address: u64,
    pub wireless_info: Option<WirelessInfo>,
}

pub struct NetInfo {
    udev: *mut libudev_sys::udev,
    nm_proxy: *mut gtk::gio::ffi::GDBusProxy,
}

#[derive(Debug, Copy, Clone, glib::ErrorDomain)]
#[error_domain(name = "Udev")]
enum UdevError {
    InitializationFailed,
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
    pub fn new() -> Result<Self, gtk::glib::Error> {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, gobject_ffi::*, translate::from_glib_full, Error};
        use libudev_sys::*;

        use std::ffi::CStr;

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
        if nm_proxy == std::ptr::null_mut() {
            let error = unsafe { from_glib_full(error) };
            return Err(error);
        }

        let udev = unsafe { udev_new() };
        if nm_proxy == std::ptr::null_mut() {
            let error = Error::new::<UdevError>(
                UdevError::InitializationFailed,
                "Failed to create udev context",
            );
            return Err(error);
        }

        Ok(Self { udev, nm_proxy })
    }

    pub fn load_net_info<
        'a,
        DeviceIf: Into<&'a str>,
        InterfaceNames: IntoIterator<Item = DeviceIf>,
    >(
        &self,
        devices: InterfaceNames,
    ) -> Vec<Option<NetDevice>> {
        extern "C" {
            fn strerror(error: i32) -> *const i8;
        }

        use errno_sys::errno_location;
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, gobject_ffi::*, translate::from_glib_full, Error, *};
        use libudev_sys::*;

        use std::ffi::{CStr, CString};

        let mut error: *mut GError = std::ptr::null_mut();

        let mut result = vec![];
        for device_if in devices {
            if let Ok(device_name) = CString::new(device_if.into()) {
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
                if device_path_variant == std::ptr::null_mut() {
                    let error: Error = unsafe { from_glib_full(error) };
                    g_error!(
                        "MissionCenter::NetInfo",
                        "Failed to get device info: {}",
                        error.message()
                    );

                    result.push(None);
                    continue;
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
                    result.push(None);
                    continue;
                }

                let device_proxy = unsafe {
                    g_dbus_proxy_new_for_bus_sync(
                        G_BUS_TYPE_SYSTEM,
                        G_DBUS_PROXY_FLAGS_NONE,
                        std::ptr::null_mut(),
                        b"org.freedesktop.NetworkManager\0".as_ptr() as _,
                        device_path,
                        b"org.freedesktop.NetworkManager.Device\0".as_ptr() as _,
                        std::ptr::null_mut(),
                        &mut error,
                    )
                };
                if device_proxy == std::ptr::null_mut() {
                    let error: Error = unsafe { from_glib_full(error) };
                    g_error!(
                        "MissionCenter::NetInfo",
                        "Failed to get udev device path: {}",
                        error.message()
                    );

                    unsafe { g_variant_unref(device_path_variant) };

                    result.push(None);
                    continue;
                }

                let udi_variant = unsafe {
                    g_dbus_proxy_call_sync(
                        device_proxy,
                        b"org.freedesktop.DBus.Properties.Get\0".as_ptr() as _,
                        g_variant_new(
                            b"(ss)\0".as_ptr() as _,
                            b"org.freedesktop.NetworkManager.Device\0".as_ptr() as *const i8,
                            b"Udi\0".as_ptr() as *const i8,
                        ),
                        G_DBUS_CALL_FLAGS_NONE,
                        -1,
                        std::ptr::null_mut(),
                        &mut error,
                    )
                };
                if udi_variant == std::ptr::null_mut() {
                    let error: Error = unsafe { from_glib_full(error) };
                    g_error!(
                        "MissionCenter::NetInfo",
                        "Failed to get udev device path: {}",
                        error.message()
                    );

                    unsafe { g_variant_unref(device_path_variant) };

                    result.push(None);
                    continue;
                }

                let mut udi_inner: *mut GVariant = std::ptr::null_mut();
                unsafe { g_variant_get(udi_variant, b"(v)\0".as_ptr() as _, &mut udi_inner) };
                if udi_inner.is_null() {
                    unsafe {
                        g_variant_unref(udi_variant);
                        g_variant_unref(device_path_variant);
                    };

                    result.push(None);
                    continue;
                }

                let mut udi: *mut i8 = std::ptr::null_mut();
                unsafe { g_variant_get(udi_inner, b"&s\0".as_ptr() as _, &mut udi) };
                if udi.is_null() {
                    unsafe {
                        g_variant_unref(udi_variant);
                        g_variant_unref(device_path_variant);
                    };

                    result.push(None);
                    continue;
                }

                let udev_device = unsafe { udev_device_new_from_syspath(self.udev, udi) };
                if udev_device == std::ptr::null_mut() {
                    let err = unsafe { *errno_location() };
                    let error_message = unsafe { CStr::from_ptr(strerror(err)) }
                        .to_str()
                        .map_or("Unknown error", |s| s)
                        .to_owned();

                    unsafe {
                        g_variant_unref(udi_variant);
                        g_variant_unref(device_path_variant);
                    };

                    g_error!(
                        "MissionCenter::NetInfo",
                        "Failed to create udev device from {:?}. {}",
                        unsafe { CStr::from_ptr(udi) },
                        error_message
                    );

                    result.push(None);
                    continue;
                }

                let mut dev_name = unsafe {
                    Self::get_udev_property(
                        udev_device,
                        b"ID_MODEL_ENC\0".as_ptr() as _,
                        b"ID_MODEL_FROM_DATABASE\0".as_ptr() as _,
                    )
                };
                dbg!(dev_name);
                dev_name = unsafe {
                    Self::get_udev_property(
                        udev_device,
                        b"ID_MODEL_ENC\0".as_ptr() as _,
                        b"ID_PRODUCT_FROM_DATABASE\0".as_ptr() as _,
                    )
                };
                dbg!(dev_name);

                unsafe {
                    udev_device_unref(udev_device);

                    g_variant_unref(udi_variant);
                    g_variant_unref(device_path_variant)
                };
            }
        }

        result
    }

    unsafe fn get_udev_property(
        device: *mut libudev_sys::udev_device,
        enc_prop: *const i8, /* ID_XXX_ENC */
        db_prop: *const i8,  /* ID_XXX_FROM_DATABASE */
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

        /* Prefer the hwdata database value over what comes directly
         * from the device. */
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
