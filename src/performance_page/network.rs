/* performance_page/network.rs
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

use std::cell::Cell;

use adw::subclass::prelude::*;
use glib::{clone, ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*};

use crate::i18n::*;

use super::widgets::GraphWidget;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageNetwork)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/network.ui")]
    pub struct PerformancePageNetwork {
        #[template_child]
        pub title_connection_type: TemplateChild<gtk::Label>,
        #[template_child]
        pub device_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub max_y: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub legend_send: TemplateChild<gtk::Picture>,
        #[template_child]
        pub speed_send: TemplateChild<gtk::Label>,
        #[template_child]
        pub legend_recv: TemplateChild<gtk::Picture>,
        #[template_child]
        pub speed_recv: TemplateChild<gtk::Label>,
        #[template_child]
        pub interface_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub connection_type_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub ssid: TemplateChild<gtk::Label>,
        #[template_child]
        pub signal_strength: TemplateChild<gtk::Image>,
        #[template_child]
        pub max_bitrate: TemplateChild<gtk::Label>,
        #[template_child]
        pub frequency: TemplateChild<gtk::Label>,
        #[template_child]
        pub hw_address: TemplateChild<gtk::Label>,
        #[template_child]
        pub ipv4_address: TemplateChild<gtk::Label>,
        #[template_child]
        pub ipv6_address: TemplateChild<gtk::Label>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,

        #[property(get = Self::interface_name, set = Self::set_interface_name, type = String)]
        pub interface_name: Cell<String>,
        #[property(get = Self::connection_type, set = Self::set_connection_type, type = u8)]
        pub connection_type: Cell<crate::sys_info_v2::NetDeviceType>,

        signal_strength_percent: Cell<Option<u8>>,
    }

    impl Default for PerformancePageNetwork {
        fn default() -> Self {
            Self {
                title_connection_type: Default::default(),
                device_name: Default::default(),
                max_y: Default::default(),
                usage_graph: Default::default(),
                legend_send: Default::default(),
                speed_send: Default::default(),
                legend_recv: Default::default(),
                speed_recv: Default::default(),
                interface_name_label: Default::default(),
                connection_type_label: Default::default(),
                ssid: Default::default(),
                signal_strength: Default::default(),
                max_bitrate: Default::default(),
                frequency: Default::default(),
                hw_address: Default::default(),
                ipv4_address: Default::default(),
                ipv6_address: Default::default(),
                context_menu: Default::default(),

                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),

                interface_name: Cell::new(String::new()),
                connection_type: Cell::new(crate::sys_info_v2::NetDeviceType::Other),

                signal_strength_percent: Cell::new(None),
            }
        }
    }

    impl PerformancePageNetwork {
        fn interface_name(&self) -> String {
            unsafe { &*self.interface_name.as_ptr() }.clone()
        }

        fn set_interface_name(&self, interface_name: String) {
            {
                let if_name = unsafe { &*self.interface_name.as_ptr() };
                if if_name == &interface_name {
                    return;
                }
            }

            self.interface_name.replace(interface_name);
        }

        fn connection_type(&self) -> u8 {
            self.connection_type.get() as u8
        }

        fn set_connection_type(&self, connection_type: u8) {
            {
                let if_type = self.connection_type.get();
                if if_type as u8 == connection_type {
                    return;
                }
            }

            match connection_type {
                0_u8 => self
                    .connection_type
                    .replace(crate::sys_info_v2::NetDeviceType::Wired),
                1_u8 => self
                    .connection_type
                    .replace(crate::sys_info_v2::NetDeviceType::Wireless),
                _ => self
                    .connection_type
                    .replace(crate::sys_info_v2::NetDeviceType::Other),
            };
        }
    }

    impl PerformancePageNetwork {
        fn configure_actions(this: &super::PerformancePageNetwork) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let action = gio::SimpleAction::new("network-settings", None);
            action.connect_activate(clone!(@weak this => move |_, _| {
                use crate::sys_info_v2::NetDeviceType;
                unsafe {
                    PerformancePageNetwork::gnome_settings_activate_action(
                        if this.connection_type() == NetDeviceType::Wireless as u8 {
                            "('launch-panel', [<('wifi', [<''>])>], {})"
                        } else {
                            "('launch-panel', [<('network', [<''>])>], {})"
                        }
                    )
                }
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("copy", None);
            action.connect_activate(clone!(@weak this => move |_, _| {
                let clipboard = this.clipboard();
                clipboard.set_text(this.imp().data_summary().as_str());
            }));
            actions.add_action(&action);
        }

        fn configure_context_menu(this: &super::PerformancePageNetwork) {
            let right_click_controller = gtk::GestureClick::new();
            right_click_controller.set_button(3); // Secondary click (AKA right click)
            right_click_controller.connect_released(
                clone!(@weak this => move |_click, _n_press, x, y| {
                    this
                        .imp()
                        .context_menu
                        .set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                            x.round() as i32,
                            y.round() as i32,
                            1,
                            1,
                        )));
                    this.imp().context_menu.popup();
                }),
            );
            this.add_controller(right_click_controller);
        }

        unsafe fn gnome_settings_activate_action(variant_str: &str) {
            use gtk::gio::ffi::*;
            use gtk::glib::{ffi::*, gobject_ffi::*, translate::from_glib_full, *};

            let mut error: *mut GError = std::ptr::null_mut();

            let gnome_settings_proxy = g_dbus_proxy_new_for_bus_sync(
                G_BUS_TYPE_SESSION,
                G_DBUS_PROXY_FLAGS_NONE,
                std::ptr::null_mut(),
                b"org.gnome.Settings\0".as_ptr() as _,
                b"/org/gnome/Settings\0".as_ptr() as _,
                b"org.freedesktop.Application\0".as_ptr() as _,
                std::ptr::null_mut(),
                &mut error,
            );

            if gnome_settings_proxy.is_null() {
                if !error.is_null() {
                    let error: Error = from_glib_full(error);
                    g_critical!(
                        "MissionCenter",
                        "Failed to open settings panel, failed connect to 'org.gnome.Settings': {}",
                        error.message()
                    );
                } else {
                    g_critical!(
                        "MissionCenter",
                        "Failed to open settings panel, failed connect to 'org.gnome.Settings': Unknown error",
                    );
                }
                return;
            }

            let method_params =
                Variant::parse(Some(VariantTy::new("(sava{sv})").unwrap()), variant_str);
            if method_params.is_err() {
                g_object_unref(gnome_settings_proxy as _);

                g_critical!(
                    "MissionCenter",
                    "Failed to open settings panel, failed set-up D-Bus call parameters: {}",
                    method_params.err().unwrap().message()
                );

                return;
            }
            let method_params = method_params.unwrap();

            let variant = g_dbus_proxy_call_sync(
                gnome_settings_proxy,
                b"org.freedesktop.Application.ActivateAction\0".as_ptr() as _,
                method_params.as_ptr(),
                G_DBUS_CALL_FLAGS_NONE,
                -1,
                std::ptr::null_mut(),
                &mut error,
            );
            if variant.is_null() {
                g_object_unref(gnome_settings_proxy as _);

                if !error.is_null() {
                    let error: Error = from_glib_full(error);
                    g_critical!(
                        "MissionCenter",
                        "Failed to open settings panel, failed to call 'org.freedesktop.Application.ActivateAction': {}",
                        error.message()
                    );
                } else {
                    g_critical!(
                        "MissionCenter",
                        "Failed to open settings panel, failed to call 'org.freedesktop.Application.ActivateAction': Unknown error",
                    );
                }

                return;
            }

            g_variant_unref(variant);
            g_object_unref(gnome_settings_proxy as _);
        }
    }

    impl PerformancePageNetwork {
        pub fn set_static_information(
            this: &super::PerformancePageNetwork,
            network_device: &crate::sys_info_v2::NetworkDevice,
        ) -> bool {
            use crate::sys_info_v2::NetDeviceType;

            let this = this.imp();

            let interface_name = this.interface_name.take();
            let connection_type = this.connection_type.get();

            if let Some(adapter_name) = network_device.descriptor.adapter_name.as_ref() {
                this.device_name.set_text(adapter_name.as_str());
            }

            this.usage_graph.connect_resize(|graph, width, height| {
                let width = width as f32;
                let height = height as f32;

                let mut a = width;
                let mut b = height;
                if width > height {
                    a = height;
                    b = width;
                }

                graph.set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);
            });

            this.interface_name_label.set_text(&interface_name);

            let conn_type = match connection_type {
                NetDeviceType::Wired => i18n("Ethernet"),
                NetDeviceType::Wireless => {
                    this.ssid.set_visible(true);
                    this.signal_strength.set_visible(true);
                    this.max_bitrate.set_visible(true);
                    this.frequency.set_visible(true);

                    i18n("Wi-Fi")
                }
                NetDeviceType::Other => i18n("Other"),
            };
            this.connection_type_label.set_text(&conn_type);
            this.title_connection_type.set_text(&conn_type);

            this.legend_send
                .set_resource(Some("/io/missioncenter/MissionCenter/line-dashed-net.svg"));
            this.legend_recv
                .set_resource(Some("/io/missioncenter/MissionCenter/line-solid-net.svg"));

            this.interface_name.set(interface_name);

            this.usage_graph.set_filled(0, false);
            this.usage_graph.set_dashed(0, true);

            true
        }

        pub fn update_readings(
            this: &super::PerformancePageNetwork,
            network_device: &crate::sys_info_v2::NetworkDevice,
        ) -> bool {
            let this = this.imp();

            let sent = network_device.send_bps as f32 * 8.;
            let received = network_device.recv_bps as f32 * 8.;

            this.usage_graph.add_data_point(0, sent);
            this.usage_graph.add_data_point(1, received);

            if let Some(wireless_info) = &network_device.wireless_info {
                this.ssid.set_text(
                    &wireless_info
                        .ssid
                        .as_ref()
                        .map_or(i18n("Unknown"), |ssid| ssid.clone()),
                );
                this.signal_strength_percent
                    .set(wireless_info.signal_strength_percent.clone());
                this.signal_strength.set_icon_name(Some(
                    if let Some(percentage) = wireless_info.signal_strength_percent.as_ref() {
                        if *percentage <= 20_u8 {
                            "network-wireless-signal-none-symbolic"
                        } else if *percentage <= 40_u8 {
                            "network-wireless-signal-weak-symbolic"
                        } else if *percentage <= 60_u8 {
                            "network-wireless-signal-ok-symbolic"
                        } else if *percentage <= 80_u8 {
                            "network-wireless-signal-good-symbolic"
                        } else {
                            "network-wireless-signal-excellent-symbolic"
                        }
                    } else {
                        "network-wireless-no-route-symbolic"
                    },
                ));
                this.max_bitrate
                    .set_text(&wireless_info.bitrate_kbps.as_ref().map_or(
                        i18n("Unknown"),
                        |kbps| {
                            let (val, unit) = crate::to_human_readable(*kbps as f32 * 1000., 1024.);
                            format!("{} {}bps", val.round(), unit)
                        },
                    ));
                this.frequency
                    .set_text(&wireless_info.frequency_mhz.as_ref().map_or(
                        i18n("Unknown"),
                        |freq| {
                            let (freq, unit) =
                                crate::to_human_readable(*freq as f32 * 1000. * 1000., 1000.);
                            format!("{:.2} {}Hz", freq, unit)
                        },
                    ));
            }

            let max_y = crate::to_human_readable(this.usage_graph.value_range_max(), 1024.);
            this.max_y.set_text(&i18n_f(
                "{} {}bps",
                &[&format!("{}", max_y.0), &format!("{}", max_y.1)],
            ));

            let speed_send_info = crate::to_human_readable(sent, 1024.);
            this.speed_send.set_text(&i18n_f(
                "{} {}bps",
                &[
                    &format!("{}", speed_send_info.0.round()),
                    &format!("{}", speed_send_info.1),
                ],
            ));
            let speed_recv_info = crate::to_human_readable(received, 1024.);
            this.speed_recv.set_text(&i18n_f(
                "{} {}bps",
                &[
                    &format!("{}", speed_recv_info.0.round()),
                    &format!("{}", speed_recv_info.1),
                ],
            ));

            this.hw_address
                .set_text(
                    &network_device
                        .address
                        .hw_address
                        .map_or(i18n("Unknown"), |hw| {
                            format!(
                                "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                                hw[0], hw[1], hw[2], hw[3], hw[4], hw[5]
                            )
                        }),
                );

            this.ipv4_address
                .set_text(
                    &network_device
                        .address
                        .ip4_address
                        .map_or(i18n("N/A"), |ip| {
                            let ip_array = unsafe {
                                std::slice::from_raw_parts(&ip as *const u32 as *const u8, 4)
                            };
                            format!(
                                "{}.{}.{}.{}",
                                ip_array[0], ip_array[1], ip_array[2], ip_array[3]
                            )
                        }),
                );

            this.ipv6_address
                .set_text(
                    &network_device
                        .address
                        .ip6_address
                        .map_or(i18n("N/A"), |ip| {
                            let ip_array = unsafe {
                                std::slice::from_raw_parts(&ip as *const u128 as *const u16, 16)
                            };
                            let mut ip_address = format!("{:x}:", u16::from_le(ip_array[7]));
                            ip_address.reserve(8 * 4);

                            for i in (0..7).rev() {
                                if ip_array[i] != 0 {
                                    ip_address.push(':');
                                    ip_address
                                        .push_str(&format!("{:x}", u16::from_le(ip_array[i])));
                                }
                            }
                            ip_address
                        }),
                );

            true
        }

        fn data_summary(&self) -> String {
            format!(
                r#"{}

    {}

    Interface name:   {}
    Connection type:  {}{}
    Hardware address: {}
    IPv4 address:     {}
    IPv6 address:     {}

    Send:            {}
    Receive:         {}"#,
                self.title_connection_type.label(),
                self.device_name.label(),
                self.interface_name_label.label(),
                self.connection_type_label.label(),
                if self.connection_type.get() == crate::sys_info_v2::NetDeviceType::Wireless {
                    format!(
                        r#"
    SSID:             {}
    Signal strength:  {}
    Max bitrate:      {}
    Frequency:        {}"#,
                        self.ssid.label(),
                        self.signal_strength_percent
                            .get()
                            .map_or(i18n("Unknown"), |percent| format!("{}%", percent)),
                        self.max_bitrate.label(),
                        self.frequency.label(),
                    )
                } else {
                    "".to_owned()
                },
                self.hw_address.label(),
                self.ipv4_address.label(),
                self.ipv6_address.label(),
                self.speed_send.label(),
                self.speed_recv.label(),
            )
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePageNetwork {
        const NAME: &'static str = "PerformancePageNetwork";
        type Type = super::PerformancePageNetwork;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePageNetwork {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageNetwork>().clone();

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);
        }

        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for PerformancePageNetwork {}

    impl BoxImpl for PerformancePageNetwork {}
}

glib::wrapper! {
    pub struct PerformancePageNetwork(ObjectSubclass<imp::PerformancePageNetwork>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePageNetwork {
    pub fn new(interface_name: &str, connection_type: crate::sys_info_v2::NetDeviceType) -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(
                Self::static_type(),
                &mut [
                    ("interface-name", interface_name.into()),
                    ("connection-type", (connection_type as u8).into()),
                ],
            )
            .downcast()
            .unwrap()
        };

        this
    }

    pub fn set_static_information(
        &self,
        network_device: &crate::sys_info_v2::NetworkDevice,
    ) -> bool {
        imp::PerformancePageNetwork::set_static_information(self, network_device)
    }

    pub fn update_readings(&self, network_device: &crate::sys_info_v2::NetworkDevice) -> bool {
        imp::PerformancePageNetwork::update_readings(self, network_device)
    }
}
