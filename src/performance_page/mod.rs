/* performance_page/mod.rs
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

use crate::graph_widget::GraphWidget;

mod cpu;

pub type Cpu = cpu::PerformancePageCpu;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePage)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/performance_page/page.ui")]
    pub struct PerformancePage {
        #[template_child]
        pub cpu_usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub cpu_usage_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub sidebar: TemplateChild<gtk::ListBox>,

        #[property(get, set)]
        refresh_interval: Cell<u32>,
    }

    impl Default for PerformancePage {
        fn default() -> Self {
            Self {
                cpu_usage_graph: Default::default(),
                cpu_usage_label: Default::default(),
                sidebar: Default::default(),

                refresh_interval: Cell::new(1000),
            }
        }
    }

    impl PerformancePage {
        fn configure_actions(this: &super::PerformancePage) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let action = gio::SimpleAction::new("summary", None);
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("cpu", None);
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("memory", None);
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("disk", None);
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("network", None);
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("gpu", None);
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("copy", None);
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
            }));
            actions.add_action(&action);
        }

        fn update_graph(this: &super::PerformancePage) {
            use crate::SYS_INFO;
            use sysinfo::{CpuExt, SystemExt};

            let cpu_usage = SYS_INFO
                .read()
                .expect("Failed to read CPU information: Unable to acquire lock")
                .global_cpu_info()
                .cpu_usage();

            this.imp().cpu_usage_graph.get().add_data_point(cpu_usage);
            this.imp()
                .cpu_usage_label
                .set_label(&format!("{}% 3.50Ghz", cpu_usage.round()));

            let this = this.clone();
            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_graph(&this);
                },
            ));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePage {
        const NAME: &'static str = "PerformancePage";
        type Type = super::PerformancePage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePage {
        fn constructed(&self) {
            self.parent_constructed();
            Self::configure_actions(self.obj().upcast_ref());
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

    impl WidgetImpl for PerformancePage {
        fn realize(&self) {
            self.parent_realize();

            let row = self
                .sidebar
                .row_at_index(0)
                .expect("Failed to select first row");
            self.sidebar.select_row(Some(&row));

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePage>().clone();
            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_graph(&this);
                },
            ));
        }
    }

    impl BoxImpl for PerformancePage {}
}

glib::wrapper! {
    pub struct PerformancePage(ObjectSubclass<imp::PerformancePage>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}
