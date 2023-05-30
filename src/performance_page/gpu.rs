/* performance_page/gpu.rs
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

use adw;
use adw::subclass::prelude::*;
use glib::{clone, ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*, Snapshot};

use super::widgets::GraphWidget;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageGpu)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/gpu.ui")]
    pub struct PerformancePageGpu {
        #[template_child]
        pub admin_banner: TemplateChild<adw::Banner>,
        #[template_child]
        pub device_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph_overall: TemplateChild<GraphWidget>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get = Self::name, set = Self::set_name, type = String)]
        name: Cell<String>,
        #[property(get, set)]
        refresh_interval: Cell<u32>,
        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,
    }

    impl Default for PerformancePageGpu {
        fn default() -> Self {
            Self {
                admin_banner: Default::default(),
                device_name: Default::default(),
                usage_graph_overall: Default::default(),
                toast_overlay: Default::default(),
                context_menu: Default::default(),

                name: Cell::new(String::new()),
                refresh_interval: Cell::new(1000),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),
            }
        }
    }

    impl PerformancePageGpu {
        fn name(&self) -> String {
            unsafe { &*self.name.as_ptr() }.clone()
        }

        fn set_name(&self, name: String) {
            {
                let if_name = unsafe { &*self.name.as_ptr() };
                if if_name == &name {
                    return;
                }
            }

            self.name.replace(name);
            self.update_static_information();
        }
    }

    impl PerformancePageGpu {
        fn configure_actions(this: &super::PerformancePageGpu) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));
        }

        fn configure_context_menu(this: &super::PerformancePageGpu) {
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

        fn update_view(&self, this: &super::PerformancePageGpu) {
            use crate::SYS_INFO;

            let this = this.clone();
            let _sys_info = SYS_INFO.read().expect("Failed to acquire read lock");

            self.update_graphs_grid_layout();

            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_view(this.imp(), &this);
                },
            ));
        }

        fn update_static_information(&self) {
            self.device_name.set_text(&self.name());
        }

        fn update_graphs_grid_layout(&self) {
            let width = self.usage_graph_overall.allocated_width() as f32;
            let height = self.usage_graph_overall.allocated_height() as f32;

            let mut a = width;
            let mut b = height;
            if width > height {
                a = height;
                b = width;
            }

            self.usage_graph_overall
                .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePageGpu {
        const NAME: &'static str = "PerformancePageGpu";
        type Type = super::PerformancePageGpu;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePageGpu {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageGpu>().clone();

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);
            self.update_static_information();

            self.admin_banner
                .connect_button_clicked(clone!(@weak this => move |_| {
                    let ptr = this.as_ptr() as usize;
                    let _ = std::thread::spawn(move || {
                        glib::idle_add_once(move || {
                            use glib::translate::from_glib_none;

                            let this: gtk::Widget = unsafe { from_glib_none(ptr as *mut gtk::ffi::GtkWidget) };
                            let this = this.downcast_ref::<super::PerformancePageGpu>().unwrap();
                            this.imp().admin_banner.set_revealed(false);
                            // _ => this.imp().toast_overlay.add_toast(adw::Toast::new("Authentication failed"))
                        });
                    });
                }));
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

    impl WidgetImpl for PerformancePageGpu {
        fn realize(&self) {
            self.parent_realize();

            let this = self.obj().upcast_ref::<super::PerformancePageGpu>().clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
                this.imp().admin_banner.set_revealed(true);
            });

            self.update_view(self.obj().upcast_ref());
        }

        fn snapshot(&self, snapshot: &Snapshot) {
            self.parent_snapshot(snapshot);
            self.update_graphs_grid_layout();
        }
    }

    impl BoxImpl for PerformancePageGpu {}
}

glib::wrapper! {
    pub struct PerformancePageGpu(ObjectSubclass<imp::PerformancePageGpu>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePageGpu {
    pub fn new(name: &str) -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(Self::static_type(), &mut [("name", name.into())])
                .downcast()
                .unwrap()
        };

        this
    }

    pub fn set_initial_values(&self, values: Vec<f32>) {
        self.imp().usage_graph_overall.set_data(0, values);
    }
}
