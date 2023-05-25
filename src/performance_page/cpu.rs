/* performance_page/cpu.rs
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

use super::widgets::GraphWidget;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageCpu)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/cpu.ui")]
    pub struct PerformancePageCpu {
        #[template_child]
        pub utilization_label_all: TemplateChild<gtk::Label>,
        #[template_child]
        pub utilization_label_overall: TemplateChild<gtk::Label>,
        #[template_child]
        pub cpu_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graphs: TemplateChild<gtk::Grid>,
        #[template_child]
        pub overall_graph_labels: TemplateChild<gtk::Box>,
        #[template_child]
        pub utilization: TemplateChild<gtk::Label>,
        #[template_child]
        pub speed: TemplateChild<gtk::Label>,
        #[template_child]
        pub processes: TemplateChild<gtk::Label>,
        #[template_child]
        pub threads: TemplateChild<gtk::Label>,
        #[template_child]
        pub handles: TemplateChild<gtk::Label>,
        #[template_child]
        pub uptime: TemplateChild<gtk::Label>,
        #[template_child]
        pub base_speed: TemplateChild<gtk::Label>,
        #[template_child]
        pub sockets: TemplateChild<gtk::Label>,
        #[template_child]
        pub virt_proc: TemplateChild<gtk::Label>,
        #[template_child]
        pub virtualization: TemplateChild<gtk::Label>,
        #[template_child]
        pub virt_machine: TemplateChild<gtk::Label>,
        #[template_child]
        pub l1_cache: TemplateChild<gtk::Label>,
        #[template_child]
        pub l2_cache: TemplateChild<gtk::Label>,
        #[template_child]
        pub l3_cache: TemplateChild<gtk::Label>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get, set)]
        refresh_interval: Cell<u32>,
        #[property(get, set = Self::set_base_color)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,

        pub graph_widgets: Cell<Vec<GraphWidget>>,
    }

    impl Default for PerformancePageCpu {
        fn default() -> Self {
            Self {
                utilization_label_all: Default::default(),
                utilization_label_overall: Default::default(),
                cpu_name: Default::default(),
                usage_graphs: Default::default(),
                overall_graph_labels: Default::default(),
                utilization: Default::default(),
                speed: Default::default(),
                processes: Default::default(),
                threads: Default::default(),
                handles: Default::default(),
                uptime: Default::default(),
                base_speed: Default::default(),
                sockets: Default::default(),
                virt_proc: Default::default(),
                virtualization: Default::default(),
                virt_machine: Default::default(),
                l1_cache: Default::default(),
                l2_cache: Default::default(),
                l3_cache: Default::default(),
                context_menu: Default::default(),

                refresh_interval: Cell::new(1000),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),

                graph_widgets: Cell::new(Vec::new()),
            }
        }
    }

    impl PerformancePageCpu {
        fn set_base_color(&self, base_color: gtk::gdk::RGBA) {
            let graph_widgets = self.graph_widgets.take();
            for graph_widget in &graph_widgets {
                graph_widget.set_base_color(base_color.clone());
            }
            self.graph_widgets.set(graph_widgets);

            self.base_color.set(base_color);
        }
    }

    impl PerformancePageCpu {
        fn configure_actions(this: &super::PerformancePageCpu) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            // TODO: Read these from GSettings
            let overall_action =
                gio::SimpleAction::new_stateful("overall", None, glib::Variant::from(false));
            let all_processors_action =
                gio::SimpleAction::new_stateful("all-processors", None, glib::Variant::from(true));

            let apa = all_processors_action.clone();
            overall_action.connect_activate(clone!(@weak this => move |action, _| {
                let graph_widgets = this.imp().graph_widgets.take();

                graph_widgets[0].set_visible(true);
                this.imp().overall_graph_labels.set_visible(true);
                this.imp().utilization_label_overall.set_visible(true);
                this.imp().utilization_label_all.set_visible(false);

                for graph_widget in graph_widgets.iter().skip(1) {
                    graph_widget.set_visible(false);
                }

                action.set_state(glib::Variant::from(true));
                apa.set_state(glib::Variant::from(false));

                this.imp().graph_widgets.set(graph_widgets);
            }));
            actions.add_action(&overall_action);

            let ova = overall_action.clone();
            all_processors_action.connect_activate(clone!(@weak this => move |action, _| {
                let graph_widgets = this.imp().graph_widgets.take();

                graph_widgets[0].set_visible(false);
                this.imp().overall_graph_labels.set_visible(false);
                this.imp().utilization_label_overall.set_visible(false);
                this.imp().utilization_label_all.set_visible(true);

                for graph_widget in graph_widgets.iter().skip(1) {
                    graph_widget.set_visible(true);
                }

                action.set_state(glib::Variant::from(true));
                ova.set_state(glib::Variant::from(false));

                this.imp().graph_widgets.set(graph_widgets);
            }));
            actions.add_action(&all_processors_action);

            let action =
                gio::SimpleAction::new_stateful("kernel-times", None, glib::Variant::from(false));
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
            }));
            actions.add_action(&action);
        }

        fn configure_context_menu(this: &super::PerformancePageCpu) {
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

        fn populate_usage_graphs(&self) {
            use crate::SYS_INFO;
            use sysinfo::SystemExt;

            let base_color = self.obj().base_color();

            let cpu_count = SYS_INFO
                .read()
                .expect("Failed to read CPU count: Unable to acquire lock")
                .system()
                .cpus()
                .len();

            let (_, col_count) = Self::compute_row_column_count(cpu_count);

            // Add one for overall CPU utilization
            let mut graph_widgets = vec![];

            graph_widgets.push(GraphWidget::new());
            graph_widgets[0].set_base_color(&base_color);
            self.usage_graphs.attach(&graph_widgets[0], 0, 0, 1, 1);
            graph_widgets[0].set_data_points(60);
            graph_widgets[0].set_scroll(true);
            graph_widgets[0].set_visible(false);

            self.overall_graph_labels.set_visible(false);
            self.utilization_label_overall.set_visible(false);

            for i in 0..cpu_count {
                let row_idx = i / col_count;
                let col_idx = i % col_count;

                let graph_widget_index = graph_widgets.len();

                graph_widgets.push(GraphWidget::new());
                graph_widgets[graph_widget_index].set_data_points(60);
                graph_widgets[graph_widget_index].set_base_color(&base_color);
                self.usage_graphs.attach(
                    &graph_widgets[graph_widget_index],
                    col_idx as i32,
                    row_idx as i32,
                    1,
                    1,
                );
            }

            self.graph_widgets.set(graph_widgets);
        }

        fn update_view(this: &super::PerformancePageCpu) {
            use crate::SYS_INFO;
            use sysinfo::{CpuExt, SystemExt};

            let sys_info = SYS_INFO
                .read()
                .expect("Failed to read CPU information: Unable to acquire lock");

            let mut graph_widgets = this.imp().graph_widgets.take();
            // Update global CPU graph
            graph_widgets[0].add_data_point(0, sys_info.system().global_cpu_info().cpu_usage());

            // Update per-core graphs
            for (i, cpu) in sys_info.system().cpus().iter().enumerate() {
                let graph_widget = &mut graph_widgets[i + 1];
                graph_widget.add_data_point(0, cpu.cpu_usage());
            }

            this.imp().graph_widgets.set(graph_widgets);

            // Update footer labels
            {
                let global_cpu_info = sys_info.system().global_cpu_info();
                this.imp()
                    .utilization
                    .set_text(&format!("{}%", global_cpu_info.cpu_usage().round()));
                this.imp().speed.set_text(&format!(
                    "{:.2} GHz",
                    global_cpu_info.frequency() as f32 / 1024.
                ));
                this.imp()
                    .processes
                    .set_text(&format!("{}", sys_info.process_count()));
                this.imp()
                    .threads
                    .set_text(&format!("{}", sys_info.thread_count()));
                this.imp()
                    .handles
                    .set_text(&format!("{}", sys_info.handle_count()));

                let uptime = sys_info.system().uptime();
                let days = uptime / 86400;
                let hours = (uptime % 86400) / 3600;
                let minutes = (uptime % 3600) / 60;
                let seconds = uptime % 60;
                this.imp().uptime.set_text(&format!(
                    "{:02}:{:02}:{:02}:{:02}",
                    days, hours, minutes, seconds
                ));
            }

            let this = this.clone();
            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_view(&this);
                },
            ));
        }

        fn update_static_information(&self) {
            use sysinfo::{CpuExt, SystemExt};

            let sys_info = crate::SYS_INFO
                .read()
                .expect("Failed to read CPU information: Unable to acquire lock");

            self.cpu_name
                .set_text(sys_info.system().global_cpu_info().brand());

            if let Some(base_frequency) = sys_info.cpu_base_frequency() {
                self.base_speed.set_text(&format!(
                    "{:.2} GHz",
                    base_frequency as f32 / (1000. * 1000.)
                ));
            } else {
                self.base_speed.set_text(&gettextrs::gettext("Unknown"));
            }

            self.virt_proc
                .set_text(&format!("{}", sys_info.system().cpus().len()));

            if let Some(virtualization) = sys_info.virtualization() {
                if virtualization {
                    self.virtualization
                        .set_text(&gettextrs::gettext("Supported"));
                } else {
                    self.virtualization
                        .set_text(&gettextrs::gettext("Unsupported"));
                }
            } else {
                self.virtualization.set_text(&gettextrs::gettext("Unknown"));
            }

            if let Some(is_vm) = sys_info.is_vm() {
                if is_vm {
                    self.virt_machine.set_text(&gettextrs::gettext("Yes"));
                } else {
                    self.virt_machine.set_text(&gettextrs::gettext("No"));
                }
            } else {
                self.virt_machine.set_text(&gettextrs::gettext("Unknown"));
            }

            if let Some(socket_count) = sys_info.cpu_socket_count() {
                self.sockets.set_text(&format!("{}", socket_count));
            } else {
                self.sockets.set_text(&gettextrs::gettext("Unknown"));
            }

            let l1_cache_size = if let Some(size) = sys_info.cpu_cache_size(1) {
                if size > 1024 {
                    format!("{:.1} MiB", size as f32 / 1024.)
                } else {
                    format!("{} KiB", size)
                }
            } else {
                format!("N/A")
            };
            self.l1_cache.set_text(&l1_cache_size);

            let l2_cache_size = if let Some(size) = sys_info.cpu_cache_size(2) {
                if size > 1024 {
                    format!("{:.1} MiB", size as f32 / 1024.)
                } else {
                    format!("{} KiB", size)
                }
            } else {
                format!("N/A")
            };
            self.l2_cache.set_text(&l2_cache_size);

            let l3_cache_size = if let Some(size) = sys_info.cpu_cache_size(3) {
                if size > 1024 {
                    format!("{:.1} MiB", size as f32 / 1024.)
                } else {
                    format!("{} KiB", size)
                }
            } else {
                format!("N/A")
            };
            self.l3_cache.set_text(&l3_cache_size);
        }

        fn compute_row_column_count(item_count: usize) -> (usize, usize) {
            let item_count = item_count as isize;
            let mut factors = Vec::new();
            factors.reserve(item_count as usize);

            for i in 2..=(item_count as f64).sqrt().floor() as isize {
                if item_count % i == 0 {
                    factors.push((i, item_count / i));
                }
            }
            let mut valid_factors = vec![];
            for (i, j) in factors {
                if (i - j).abs() <= 2 {
                    valid_factors.push((i, j));
                }
            }

            let result = if let Some((i, j)) = valid_factors.into_iter().max_by_key(|&(i, j)| i * j)
            {
                (i, j)
            } else {
                let i = item_count.min(((item_count as f64).sqrt() + 1.).floor() as isize);
                let j = ((item_count as f64) / i as f64).ceil() as isize;
                (i, j)
            };

            (result.0 as usize, result.1 as usize)
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePageCpu {
        const NAME: &'static str = "PerformancePageCpu";
        type Type = super::PerformancePageCpu;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePageCpu {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageCpu>().clone();

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);
            self.populate_usage_graphs();
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

    impl WidgetImpl for PerformancePageCpu {
        fn realize(&self) {
            self.parent_realize();

            Self::update_view(self.obj().upcast_ref());
        }

        fn snapshot(&self, snapshot: &Snapshot) {
            self.parent_snapshot(snapshot);

            let graph_widgets = self.graph_widgets.take();

            let graph_width = self.obj().allocated_width() as u32;
            graph_widgets[0].set_vertical_line_count(graph_width / 100);

            self.graph_widgets.set(graph_widgets);
        }
    }

    impl BoxImpl for PerformancePageCpu {}
}

glib::wrapper! {
    pub struct PerformancePageCpu(ObjectSubclass<imp::PerformancePageCpu>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePageCpu {
    pub fn new() -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(Self::static_type(), &mut [])
                .downcast()
                .unwrap()
        };
        this
    }

    pub fn set_initial_values(&self, values: Vec<f32>) {
        let imp = self.imp();

        let graph_widgets = imp.graph_widgets.take();
        for i in 0..(graph_widgets.len() - 1) {
            graph_widgets[i].set_data(0, values.clone());
        }
        graph_widgets[graph_widgets.len() - 1].set_data(0, values);

        imp.graph_widgets.set(graph_widgets);
    }
}
