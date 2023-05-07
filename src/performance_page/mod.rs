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

use std::{cell::Cell, collections::HashMap};

use adw::subclass::prelude::*;
use gettextrs::gettext;
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
    use crate::to_human_readable;

    use super::*;

    enum Pages {
        Cpu((SummaryGraph, Cpu)),
        Network(HashMap<String, (SummaryGraph, Network)>),
    }

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePage)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/performance_page/page.ui")]
    pub struct PerformancePage {
        #[template_child]
        pub sidebar: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub page_stack: TemplateChild<gtk::Stack>,

        #[property(get, set)]
        refresh_interval: Cell<u32>,

        pages: Cell<Vec<Pages>>,
    }

    impl Default for PerformancePage {
        fn default() -> Self {
            Self {
                sidebar: Default::default(),
                page_stack: Default::default(),

                refresh_interval: Cell::new(1000),
                pages: Cell::new(Vec::new()),
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

        fn set_up_cpu_page(&self, pages: &mut Vec<Pages>) {
            const BASE_COLOR: [u8; 3] = [0x11, 0x7d, 0xbb];

            let summary = SummaryGraph::new();
            summary.set_widget_name("cpu");

            summary.set_heading(gettext("CPU"));
            summary.set_info1("0% 0.00 Ghz");

            summary.set_base_color(gtk::gdk::RGBA::new(
                BASE_COLOR[0] as f32 / 255.,
                BASE_COLOR[1] as f32 / 255.,
                BASE_COLOR[2] as f32 / 255.,
                1.,
            ));

            let page = Cpu::new();
            page.set_base_color(gtk::gdk::RGBA::new(
                BASE_COLOR[0] as f32 / 255.,
                BASE_COLOR[1] as f32 / 255.,
                BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            self.obj()
                .as_ref()
                .bind_property("refresh-interval", &page, "refresh-interval")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            self.sidebar.append(&summary);
            self.page_stack.add_named(&page, Some("cpu"));

            pages.push(Pages::Cpu((summary, page)));
        }

        fn set_up_network_pages(&self, pages: &mut Vec<Pages>) {
            use crate::SYS_INFO;
            use sysinfo::SystemExt;

            const BASE_COLOR: [u8; 3] = [0xe8, 0x89, 0xc5];

            let sys_info = SYS_INFO
                .read()
                .expect("Failed to read system information: Unable to acquire lock");

            let mut networks = HashMap::new();
            for (if_name, _net) in sys_info.system().networks() {
                if if_name == "lo" {
                    continue;
                }

                let conn_type = if if_name.starts_with("wl") {
                    gettext("Wi-Fi")
                } else {
                    gettext("Ethernet")
                };

                let summary = SummaryGraph::new();
                summary.set_widget_name(if_name);

                summary.set_heading(conn_type.clone());
                summary.set_info1(if_name.clone());

                {
                    let graph_widget = summary.graph_widget();

                    graph_widget.set_data_set_count(2);
                    graph_widget.set_auto_scale(true);
                    graph_widget.set_base_color(gtk::gdk::RGBA::new(
                        BASE_COLOR[0] as f32 / 255.,
                        BASE_COLOR[1] as f32 / 255.,
                        BASE_COLOR[2] as f32 / 255.,
                        1.,
                    ));
                }

                let page = Network::new(&if_name, &conn_type);
                page.set_base_color(gtk::gdk::RGBA::new(
                    BASE_COLOR[0] as f32 / 255.,
                    BASE_COLOR[1] as f32 / 255.,
                    BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));
                self.obj()
                    .as_ref()
                    .bind_property("refresh-interval", &page, "refresh-interval")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();

                self.sidebar.append(&summary);
                self.page_stack.add_named(&page, Some(if_name));

                networks.insert(if_name.clone(), (summary, page));
            }

            pages.push(Pages::Network(networks));
        }

        fn update_graphs(this: &super::PerformancePage) {
            use crate::SYS_INFO;
            use sysinfo::{CpuExt, NetworkExt, SystemExt};

            let sys_info = SYS_INFO
                .read()
                .expect("Failed to read system information: Unable to acquire lock");
            let cpu_info = sys_info.system().global_cpu_info();

            let pages = this.imp().pages.take();
            for page in &pages {
                match page {
                    Pages::Cpu((summary, _)) => {
                        summary
                            .graph_widget()
                            .add_data_point(0, cpu_info.cpu_usage());
                        summary.set_info1(format!(
                            "{}% {:.2} Ghz",
                            cpu_info.cpu_usage().round(),
                            cpu_info.frequency() as f32 / 1024.
                        ));
                    }
                    Pages::Network(pages) => {
                        for (name, data) in sys_info.system().networks() {
                            if let Some((summary, _)) = pages.get(name) {
                                let sent = data.transmitted() as f32;
                                let received = data.received() as f32;

                                let graph_widget = summary.graph_widget();
                                graph_widget.add_data_point(0, sent);
                                graph_widget.add_data_point(1, received);

                                let sent = to_human_readable(sent * 8., 1024.);
                                let received = to_human_readable(received * 8., 1024.);
                                summary.set_info2(gettext!(
                                    "{}: {} {}bps {}: {} {}bps",
                                    "S",
                                    sent.0.round(),
                                    sent.1,
                                    "R",
                                    received.0.round(),
                                    received.1
                                ));
                            }
                        }
                    }
                }
            }

            this.imp().pages.set(pages);

            let this = this.clone();
            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_graphs(&this);
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

            let this = self.obj().as_ref().clone();

            Self::configure_actions(&this);

            let mut pages = vec![];

            self.set_up_cpu_page(&mut pages);
            self.set_up_network_pages(&mut pages);

            let row = self
                .sidebar
                .row_at_index(0)
                .expect("Failed to select first row");
            self.sidebar.select_row(Some(&row));

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
                        this.imp().page_stack.set_visible_child_name(page_name);
                    }
                }
            });

            self.pages.set(pages);
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

            Self::update_graphs(self.obj().as_ref());
        }
    }

    impl BoxImpl for PerformancePage {}
}

glib::wrapper! {
    pub struct PerformancePage(ObjectSubclass<imp::PerformancePage>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}
