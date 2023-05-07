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
use gtk::{gio, glib, prelude::*, Snapshot};

use crate::graph_widget::GraphWidget;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageNetwork)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/performance_page/network.ui")]
    pub struct PerformancePageNetwork {
        #[template_child]
        pub usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub interface_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub connection_type_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub ipv4_address: TemplateChild<gtk::Label>,
        #[template_child]
        pub ipv6_address: TemplateChild<gtk::Label>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get, set)]
        refresh_interval: Cell<u32>,
        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get = Self::interface_name, set = Self::set_interface_name, type = String)]
        pub interface_name: Cell<String>,
        #[property(get = Self::connection_type, set = Self::set_connection_type, type = String)]
        pub connection_type: Cell<String>,
    }

    impl Default for PerformancePageNetwork {
        fn default() -> Self {
            Self {
                usage_graph: Default::default(),
                interface_name_label: Default::default(),
                connection_type_label: Default::default(),
                ipv4_address: Default::default(),
                ipv6_address: Default::default(),
                context_menu: Default::default(),

                refresh_interval: Cell::new(1000),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),

                interface_name: Cell::new(String::new()),
                connection_type: Cell::new(String::new()),
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
            self.update_static_information();
        }

        fn connection_type(&self) -> String {
            unsafe { &*self.connection_type.as_ptr() }.clone()
        }

        fn set_connection_type(&self, connection_type: String) {
            {
                let conn_type = unsafe { &*self.interface_name.as_ptr() };
                if conn_type == &connection_type {
                    return;
                }
            }

            self.connection_type.replace(connection_type);
            self.update_static_information();
        }
    }

    impl PerformancePageNetwork {
        fn configure_actions(this: &super::PerformancePageNetwork) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let action = gio::SimpleAction::new("network-details", None);
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
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
            this.imp()
                .usage_graph
                .add_controller(right_click_controller);
        }

        fn update_view(this: &super::PerformancePageNetwork) {
            use crate::SYS_INFO;
            use sysinfo::SystemExt;

            let this = this.clone();

            let sys_info = SYS_INFO.read().expect("Failed to acquire read lock");
            let interface_name = this.imp().interface_name.take();
            for (name, _data) in sys_info.system().networks() {
                if name == &interface_name {
                    break;
                }
            }
            this.imp().interface_name.set(interface_name);

            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_view(&this);
                },
            ));
        }

        fn update_static_information(&self) {
            let interface_name = self.interface_name.take();
            let connection_type = self.connection_type.take();

            self.interface_name_label.set_text(&interface_name);
            self.connection_type_label.set_text(&connection_type);

            self.interface_name.set(interface_name);
            self.connection_type.set(connection_type);
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
            self.update_static_information();
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

    impl WidgetImpl for PerformancePageNetwork {
        fn realize(&self) {
            self.parent_realize();

            Self::update_view(self.obj().upcast_ref());
        }

        fn snapshot(&self, snapshot: &Snapshot) {
            self.parent_snapshot(snapshot);

            let graph_width = self.obj().allocated_width() as u32;
            self.usage_graph.set_vertical_line_count(graph_width / 100);
        }
    }

    impl BoxImpl for PerformancePageNetwork {}
}

glib::wrapper! {
    pub struct PerformancePageNetwork(ObjectSubclass<imp::PerformancePageNetwork>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePageNetwork {
    pub fn new(interface_name: &str, connection_type: &str) -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(
                Self::static_type(),
                &mut [
                    ("interface-name", interface_name.to_owned().into()),
                    ("connection-type", connection_type.to_owned().into()),
                ],
            )
            .downcast()
            .unwrap()
        };

        this
    }
}
