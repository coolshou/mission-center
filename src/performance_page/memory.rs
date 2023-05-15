/* performance_page/memory.rs
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

use crate::{graph_widget::GraphWidget, mem_composition_widget::MemoryCompositionWidget};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageMemory)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/memory.ui")]
    pub struct PerformancePageMemory {
        #[template_child]
        pub total_ram: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub mem_consumption: TemplateChild<MemoryCompositionWidget>,
        #[template_child]
        pub in_use: TemplateChild<gtk::Label>,
        #[template_child]
        pub available: TemplateChild<gtk::Label>,
        #[template_child]
        pub committed: TemplateChild<gtk::Label>,
        #[template_child]
        pub cached: TemplateChild<gtk::Label>,
        #[template_child]
        pub swap_available: TemplateChild<gtk::Label>,
        #[template_child]
        pub swap_used: TemplateChild<gtk::Label>,
        #[template_child]
        pub speed: TemplateChild<gtk::Label>,
        #[template_child]
        pub slots_used: TemplateChild<gtk::Label>,
        #[template_child]
        pub form_factor: TemplateChild<gtk::Label>,
        #[template_child]
        pub hw_reserved: TemplateChild<gtk::Label>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get, set)]
        refresh_interval: Cell<u32>,
        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
    }

    impl Default for PerformancePageMemory {
        fn default() -> Self {
            Self {
                total_ram: Default::default(),
                usage_graph: Default::default(),
                mem_consumption: Default::default(),
                in_use: Default::default(),
                available: Default::default(),
                committed: Default::default(),
                cached: Default::default(),
                swap_available: Default::default(),
                swap_used: Default::default(),
                speed: Default::default(),
                slots_used: Default::default(),
                form_factor: Default::default(),
                hw_reserved: Default::default(),
                context_menu: Default::default(),

                refresh_interval: Cell::new(1000),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
            }
        }
    }

    impl PerformancePageMemory {}

    impl PerformancePageMemory {
        fn configure_actions(this: &super::PerformancePageMemory) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));
        }

        fn configure_context_menu(this: &super::PerformancePageMemory) {
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

        fn update_view(this: &super::PerformancePageMemory) {
            use crate::SYS_INFO;

            let this = this.clone();

            let _sys_info = SYS_INFO.read().expect("Failed to acquire read lock");

            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_view(&this);
                },
            ));
        }

        fn update_static_information(&self) {}
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePageMemory {
        const NAME: &'static str = "PerformancePageMemory";
        type Type = super::PerformancePageMemory;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            MemoryCompositionWidget::ensure_type();
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePageMemory {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageMemory>().clone();

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

    impl WidgetImpl for PerformancePageMemory {
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

    impl BoxImpl for PerformancePageMemory {}
}

glib::wrapper! {
    pub struct PerformancePageMemory(ObjectSubclass<imp::PerformancePageMemory>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePageMemory {
    pub fn new() -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(Self::static_type(), &mut [])
                .downcast()
                .unwrap()
        };

        this
    }
}
