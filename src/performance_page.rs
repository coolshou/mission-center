/* performance_page.rs
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

use adw::subclass::prelude::*;
use glib::clone;
use gtk::{gio, glib, prelude::*};

mod imp {
    use crate::graph_widget::GraphWidget;

    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/performance_page.ui")]
    pub struct PerformancePage {
        #[template_child]
        pub cpu_usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub cpu_usage_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub sidebar: TemplateChild<gtk::ListBox>,
    }

    impl Default for PerformancePage {
        fn default() -> Self {
            Self {
                cpu_usage_graph: Default::default(),
                cpu_usage_label: Default::default(),
                sidebar: Default::default(),
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
    }

    impl WidgetImpl for PerformancePage {
        fn realize(&self) {
            use crate::SYS_INFO;
            use sysinfo::{CpuExt, SystemExt};

            self.parent_realize();

            let row = self
                .sidebar
                .row_at_index(0)
                .expect("Failed to select first row");
            self.sidebar.select_row(Some(&row));

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePage>().clone();
            Some(glib::source::timeout_add_local(
                std::time::Duration::from_millis(500),
                move || {
                    let cpu_usage = SYS_INFO
                        .read()
                        .expect("Failed to read CPU information: Unable to acquire lock")
                        .global_cpu_info()
                        .cpu_usage();

                    this.imp().cpu_usage_graph.get().add_data_point(cpu_usage);
                    this.imp()
                        .cpu_usage_label
                        .set_label(&format!("{}% 3.50Ghz", cpu_usage.round()));

                    Continue(true)
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
