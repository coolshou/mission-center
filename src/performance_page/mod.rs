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
mod network;
mod summary_graph;

type SummaryGraph = summary_graph::SummaryGraph;
type Cpu = cpu::PerformancePageCpu;
type Network = network::PerformancePageNetwork;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePage)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/performance_page/page.ui")]
    pub struct PerformancePage {
        #[template_child]
        cpu_usage_graph: TemplateChild<SummaryGraph>,
        #[template_child]
        pub sidebar: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub subpages: TemplateChild<gtk::Stack>,

        #[property(get, set)]
        refresh_interval: Cell<u32>,
    }

    impl Default for PerformancePage {
        fn default() -> Self {
            Self {
                cpu_usage_graph: Default::default(),
                sidebar: Default::default(),
                subpages: Default::default(),

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
            action.connect_activate(clone!(@weak this => move |_, _| {
                let row= this.imp()
                    .sidebar
                    .row_at_index(0)
                    .expect("Failed to select CPU row");
                this.imp().sidebar.select_row(Some(&row));
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
            use sysinfo::{CpuExt, NetworkExt, SystemExt};

            let sys_info = SYS_INFO
                .read()
                .expect("Failed to read system information: Unable to acquire lock");
            let cpu_info = sys_info.system().global_cpu_info();

            this.imp()
                .cpu_usage_graph
                .get()
                .graph_widget()
                .add_data_point(cpu_info.cpu_usage());
            this.imp().cpu_usage_graph.set_info1(format!(
                "{}% {:.2} Ghz",
                cpu_info.cpu_usage().round(),
                cpu_info.frequency() as f32 / 1024.
            ));

            for (name, data) in sys_info.system().networks() {
                let sent = data.transmitted();
                let received = data.received();
                dbg!(name, sent, received);

                break;
            }

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
            SummaryGraph::ensure_type();
            GraphWidget::ensure_type();
            Cpu::ensure_type();
            Network::ensure_type();

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

            let this = self.obj().upcast_ref::<super::PerformancePage>().clone();
            self.sidebar.connect_row_selected(move |_, selected_row| {
                use glib::translate::*;
                use std::ffi::CStr;

                if let Some(row) = selected_row {
                    let child = row.child().expect("Failed to get child of selected row");
                    let widget_name =
                        unsafe { gtk::ffi::gtk_widget_get_name(child.to_glib_none().0) };
                    if widget_name.is_null() {
                        return;
                    }
                    if let Ok(page_name) = unsafe { CStr::from_ptr(widget_name) }.to_str() {
                        this.imp().subpages.set_visible_child_name(page_name);
                    }
                }
            });

            Self::update_graph(self.obj().upcast_ref());
        }
    }

    impl BoxImpl for PerformancePage {}
}

glib::wrapper! {
    pub struct PerformancePage(ObjectSubclass<imp::PerformancePage>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}
