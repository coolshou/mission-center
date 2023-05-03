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

        pub system: std::cell::Cell<sysinfo::System>,
    }

    impl Default for PerformancePage {
        fn default() -> Self {
            use sysinfo::{System, SystemExt};

            Self {
                cpu_usage_graph: Default::default(),
                cpu_usage_label: Default::default(),
                system: std::cell::Cell::new(System::new()),
            }
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

    impl ObjectImpl for PerformancePage {}

    impl WidgetImpl for PerformancePage {
        fn realize(&self) {
            use sysinfo::{CpuExt, SystemExt};

            self.parent_realize();

            let system = unsafe { &mut *self.system.as_ptr() };
            system.refresh_cpu();

            // The windows should be destroyed after the main loop has exited so there should not be a
            // need to explicitly remove the timeout source.
            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePage>().clone();
            Some(glib::source::timeout_add_local(
                std::time::Duration::from_millis(500),
                move || {
                    let system = unsafe { &mut *this.imp().system.as_ptr() };
                    system.refresh_cpu();

                    let cpu_usage = system.global_cpu_info().cpu_usage();
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
