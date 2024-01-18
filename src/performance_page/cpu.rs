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

use std::cell::{Cell, OnceCell};

use adw::subclass::prelude::*;
use glib::{clone, ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*};

use crate::i18n::*;

use super::widgets::GraphWidget;

mod imp {
    use super::*;

    const GRAPH_SELECTION_OVERALL: i32 = 1;
    const GRAPH_SELECTION_ALL: i32 = 2;
    const GRAPH_SELECTION_STACKED: i32 = 3;

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
        pub graph_max_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,
        #[property(get, set = Self::set_base_color)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,

        pub graph_widgets: Cell<Vec<GraphWidget>>,

        pub settings: Cell<Option<gio::Settings>>,

        #[property(get = Self::infobar_content, type = Option < gtk::Widget >)]
        pub infobar_content: OnceCell<gtk::Box>,

        pub utilization: OnceCell<gtk::Label>,
        pub speed: OnceCell<gtk::Label>,
        pub processes: OnceCell<gtk::Label>,
        pub threads: OnceCell<gtk::Label>,
        pub handles: OnceCell<gtk::Label>,
        pub uptime: OnceCell<gtk::Label>,
        pub base_speed: OnceCell<gtk::Label>,
        pub sockets: OnceCell<gtk::Label>,
        pub virt_proc: OnceCell<gtk::Label>,
        pub virtualization: OnceCell<gtk::Label>,
        pub virt_machine: OnceCell<gtk::Label>,
        pub l1_cache: OnceCell<gtk::Label>,
        pub l2_cache: OnceCell<gtk::Label>,
        pub l3_cache: OnceCell<gtk::Label>,
    }

    impl Default for PerformancePageCpu {
        fn default() -> Self {
            Self {
                utilization_label_all: Default::default(),
                utilization_label_overall: Default::default(),
                cpu_name: Default::default(),
                usage_graphs: Default::default(),
                overall_graph_labels: Default::default(),
                graph_max_duration: Default::default(),
                context_menu: Default::default(),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),

                graph_widgets: Cell::new(Vec::new()),

                settings: Cell::new(None),

                infobar_content: Default::default(),
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

        fn infobar_content(&self) -> Option<gtk::Widget> {
            self.infobar_content.get().map(|ic| ic.clone().into())
        }
    }

    impl PerformancePageCpu {
        fn configure_actions(this: &super::PerformancePageCpu) {
            use gtk::glib::*;

            let settings = this.imp().settings.take();
            let mut graph_selection = GRAPH_SELECTION_OVERALL;
            let mut show_kernel_times = false;
            match settings {
                Some(settings) => {
                    graph_selection = settings.int("performance-page-cpu-graph");
                    show_kernel_times = settings.boolean("performance-page-kernel-times");

                    this.imp().settings.set(Some(settings));
                }
                None => {}
            }

            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let overall_action = gio::SimpleAction::new_stateful(
                "overall",
                None,
                &glib::Variant::from(graph_selection == GRAPH_SELECTION_OVERALL),
            );
            let all_processors_action = gio::SimpleAction::new_stateful(
                "all-processors",
                None,
                &glib::Variant::from(graph_selection == GRAPH_SELECTION_ALL),
            );
            let stacked_all_processors_action = gio::SimpleAction::new_stateful(
                "stacked-all-processors",
                None,
                &glib::Variant::from(graph_selection == GRAPH_SELECTION_STACKED),
            );

            let apa1 = all_processors_action.clone();
            let sapa1 = stacked_all_processors_action.clone();
            let ova1 = overall_action.clone();
            overall_action.connect_activate(clone!(@weak this => move |action, _| {
                use gtk::glib::*;

                let graph_widgets = this.imp().graph_widgets.take();

                graph_widgets[0].set_visible(true);
                graph_widgets[1].set_visible(false);
                this.imp().overall_graph_labels.set_visible(true);
                this.imp().utilization_label_overall.set_visible(true);
                this.imp().utilization_label_all.set_visible(false);

                for graph_widget in graph_widgets.iter().skip(2) {
                    graph_widget.set_visible(false);
                }

                action.set_state(&glib::Variant::from(true));
                apa1.set_state(&glib::Variant::from(false));
                sapa1.set_state(&glib::Variant::from(false));

                let settings = this.imp().settings.take();
                if settings.is_some() {
                    let settings = settings.unwrap();
                    settings.set_int("performance-page-cpu-graph", GRAPH_SELECTION_OVERALL).unwrap_or_else(|_| {
                        g_critical!("MissionCenter::PerformancePage", "Failed to save selected CPU graph");
                    });
                    this.imp().settings.set(Some(settings));
                }

                this.imp().graph_widgets.set(graph_widgets);
            }));
            actions.add_action(&overall_action);

            let apa2 = all_processors_action.clone();
            let sapa2 = stacked_all_processors_action.clone();
            let ova2 = overall_action.clone();

            all_processors_action.connect_activate(clone!(@weak this => move |action, _| {
                let graph_widgets = this.imp().graph_widgets.take();

                graph_widgets[0].set_visible(false);
                graph_widgets[1].set_visible(false);
                this.imp().overall_graph_labels.set_visible(false);
                this.imp().utilization_label_overall.set_visible(false);
                this.imp().utilization_label_all.set_visible(true);

                for graph_widget in graph_widgets.iter().skip(2) {
                    graph_widget.set_visible(true);
                }

                action.set_state(&glib::Variant::from(true));
                ova2.set_state(&glib::Variant::from(false));
                sapa2.set_state(&glib::Variant::from(false));

                let settings = this.imp().settings.take();
                if settings.is_some() {
                    let settings = settings.unwrap();
                    settings.set_int("performance-page-cpu-graph", GRAPH_SELECTION_ALL).unwrap_or_else(|_| {
                        g_critical!("MissionCenter::PerformancePage", "Failed to save selected CPU graph");
                    });
                    this.imp().settings.set(Some(settings));
                }

                this.imp().graph_widgets.set(graph_widgets);
            }));
            actions.add_action(&all_processors_action);

            let apa3 = all_processors_action.clone();
            let sapa3 = stacked_all_processors_action.clone();
            let ova3 = overall_action.clone();

            stacked_all_processors_action.connect_activate(clone!(@weak this => move |action, _| {
                let graph_widgets = this.imp().graph_widgets.take();

                graph_widgets[0].set_visible(false);
                graph_widgets[1].set_visible(true);
                this.imp().overall_graph_labels.set_visible(true);
                this.imp().utilization_label_overall.set_visible(true);
                this.imp().utilization_label_all.set_visible(false);

                for graph_widget in graph_widgets.iter().skip(2) {
                    graph_widget.set_visible(false);
                }

                action.set_state(&glib::Variant::from(true));
                ova3.set_state(&glib::Variant::from(false));
                apa3.set_state(&glib::Variant::from(false));

                let settings = this.imp().settings.take();
                if settings.is_some() {
                    let settings = settings.unwrap();
                    settings.set_int("performance-page-cpu-graph", GRAPH_SELECTION_STACKED).unwrap_or_else(|_| {
                        g_critical!("MissionCenter::PerformancePage", "Failed to save selected CPU graph");
                    });
                    this.imp().settings.set(Some(settings));
                }

                this.imp().graph_widgets.set(graph_widgets);
            }));
            actions.add_action(&stacked_all_processors_action);

            let action = gio::SimpleAction::new_stateful(
                "kernel_times",
                None,
                &glib::Variant::from(show_kernel_times),
            );
            action.connect_activate(clone!(@weak this => move |action, _| {
                let graph_widgets = this.imp().graph_widgets.take();

                let visible = !action.state().and_then(|v|v.get::<bool>()).unwrap_or(false);

                graph_widgets[0].set_data_visible(1, visible);

                for graph_widget in graph_widgets.iter().skip(2) {
                    graph_widget.set_data_visible(1, visible);
                }

                action.set_state(&glib::Variant::from(visible));

                let settings = this.imp().settings.take();
                if settings.is_some() {
                    let settings = settings.unwrap();
                    settings.set_boolean("performance-page-kernel-times", visible).unwrap_or_else(|_| {
                        g_critical!("MissionCenter::PerformancePage", "Failed to save kernel times setting");
                    });
                    this.imp().settings.set(Some(settings));
                }

                this.imp().graph_widgets.set(graph_widgets);
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("copy", None);
            action.connect_activate(clone!(@weak this => move |_, _| {
                let clipboard = this.clipboard();
                clipboard.set_text(this.imp().data_summary().as_str());
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
    }

    impl PerformancePageCpu {
        pub fn set_static_information(
            this: &super::PerformancePageCpu,
            readings: &crate::sys_info_v2::Readings,
        ) -> bool {
            let this = this.imp();

            let static_cpu_info = &readings.cpu_static_info;

            this.cpu_name.set_text(&static_cpu_info.name);

            this.populate_usage_graphs(static_cpu_info.logical_cpu_count as usize);

            if let Some(base_speed) = this.base_speed.get() {
                if let Some(base_frequency) = static_cpu_info.base_frequency_khz {
                    base_speed.set_text(&format!(
                        "{:.2} GHz",
                        base_frequency as f32 / (1000. * 1000.)
                    ));
                } else {
                    base_speed.set_text(&i18n("Unknown"));
                }
            }

            if let Some(virt_proc) = this.virt_proc.get() {
                virt_proc.set_text(&format!("{}", static_cpu_info.logical_cpu_count));
            }

            if let Some(virtualization) = this.virtualization.get() {
                if let Some(vt) = static_cpu_info.virtualization_technology.as_ref() {
                    virtualization.set_text(vt.as_ref());
                } else {
                    virtualization.set_text(&i18n("Unsupported"));
                }
            }

            if let Some(virt_machine) = this.virt_machine.get() {
                if let Some(is_vm) = static_cpu_info.is_virtual_machine {
                    if is_vm {
                        virt_machine.set_text(&i18n("Yes"));
                    } else {
                        virt_machine.set_text(&i18n("No"));
                    }
                } else {
                    virt_machine.set_text(&i18n("Unknown"));
                }
            }

            if let Some(sockets) = this.sockets.get() {
                if let Some(socket_count) = static_cpu_info.socket_count {
                    sockets.set_text(&format!("{}", socket_count));
                } else {
                    sockets.set_text(&i18n("Unknown"));
                }
            }

            let l1_cache_size = if let Some(size) = static_cpu_info.l1_combined_cache {
                let size = crate::to_human_readable(size as f32, 1024.);
                format!(
                    "{} {}{}B",
                    size.0,
                    size.1,
                    if size.1.is_empty() { "" } else { "i" }
                )
            } else {
                format!("N/A")
            };
            if let Some(l1_cache) = this.l1_cache.get() {
                l1_cache.set_text(&l1_cache_size);
            }

            let l2_cache_size = if let Some(size) = static_cpu_info.l2_cache {
                let size = crate::to_human_readable(size as f32, 1024.);
                format!(
                    "{} {}{}B",
                    size.0,
                    size.1,
                    if size.1.is_empty() { "" } else { "i" }
                )
            } else {
                format!("N/A")
            };
            if let Some(l2_cache) = this.l2_cache.get() {
                l2_cache.set_text(&l2_cache_size);
            }

            let l3_cache_size = if let Some(size) = static_cpu_info.l3_cache {
                let size = crate::to_human_readable(size as f32, 1024.);
                format!(
                    "{} {}{}B",
                    size.0,
                    size.1,
                    if size.1.is_empty() { "" } else { "i" }
                )
            } else {
                format!("N/A")
            };
            if let Some(l3_cache) = this.l3_cache.get() {
                l3_cache.set_text(&l3_cache_size);
            }

            let _ = if let Some(size) = static_cpu_info.l4_cache {
                let size = crate::to_human_readable(size as f32, 1024.);
                format!(
                    "{} {}{}B",
                    size.0,
                    size.1,
                    if size.1.is_empty() { "" } else { "i" }
                )
            } else {
                format!("N/A")
            };

            true
        }

        pub fn update_readings(
            this: &super::PerformancePageCpu,
            readings: &crate::sys_info_v2::Readings,
        ) -> bool {
            let mut graph_widgets = this.imp().graph_widgets.take();

            let dynamic_cpu_info = &readings.cpu_dynamic_info;

            // Update global CPU graph
            graph_widgets[0].add_data_point(0, dynamic_cpu_info.overall_utilization_percent);
            graph_widgets[0].add_data_point(1, dynamic_cpu_info.overall_kernel_utilization_percent);

            // Update per-core graphs
            for i in 0..dynamic_cpu_info.per_logical_cpu_utilization_percent.len() {
                graph_widgets[1].add_data_point(i, dynamic_cpu_info.per_logical_cpu_utilization_percent[i]);

                let graph_widget = &mut graph_widgets[i + 2];
                graph_widget
                    .add_data_point(0, dynamic_cpu_info.per_logical_cpu_utilization_percent[i]);
                graph_widget.add_data_point(
                    1,
                    dynamic_cpu_info.per_logical_cpu_kernel_utilization_percent[i],
                );
            }

            this.imp().graph_widgets.set(graph_widgets);

            if let Some(utilization) = this.imp().utilization.get() {
                utilization.set_text(&format!(
                    "{}%",
                    dynamic_cpu_info.overall_utilization_percent.round()
                ));
            }

            if let Some(speed) = this.imp().speed.get() {
                speed.set_text(&format!(
                    "{:.2} GHz",
                    readings.cpu_dynamic_info.current_frequency_mhz as f32 / 1024.
                ));
            }

            if let Some(processes) = this.imp().processes.get() {
                processes.set_text(&format!("{}", dynamic_cpu_info.process_count));
            }

            if let Some(threads) = this.imp().threads.get() {
                threads.set_text(&format!("{}", dynamic_cpu_info.thread_count));
            }

            if let Some(handles) = this.imp().handles.get() {
                handles.set_text(&format!("{}", dynamic_cpu_info.handle_count));
            }

            let uptime = dynamic_cpu_info.uptime_seconds;
            let days = uptime / 86400;
            let hours = (uptime % 86400) / 3600;
            let minutes = (uptime % 3600) / 60;
            let seconds = uptime % 60;

            if let Some(uptime) = this.imp().uptime.get() {
                uptime.set_text(&format!(
                    "{:02}:{:02}:{:02}:{:02}",
                    days, hours, minutes, seconds
                ));
            }

            true
        }

        fn data_summary(&self) -> String {
            let base_speed = self
                .base_speed
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let sockets = self
                .sockets
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let virt_proc = self
                .virt_proc
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let virtualization = self
                .virtualization
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let virt_machine = self
                .virt_machine
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let l1_cache = self
                .l1_cache
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let l2_cache = self
                .l2_cache
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let l3_cache = self
                .l3_cache
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let utilization = self
                .utilization
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let speed = self
                .speed
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let processes = self
                .processes
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let threads = self
                .threads
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let handles = self
                .handles
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());
            let uptime = self
                .uptime
                .get()
                .and_then(|v| Some(v.label()))
                .unwrap_or("".into());

            format!(
                r#"CPU
            
            {}
            
            Base speed:         {}
            Sockets:            {}
            Virtual processors: {}
            Virtualization:     {}
            Virtual machine:    {}
            L1 cache:           {}
            L2 cache:           {}
            L3 cache:           {}
            
            Utilization: {}
            Speed:       {}
            Processes:   {}
            Threads:     {}
            Handles:     {}
            Up time:     {}"#,
                self.cpu_name.label(),
                base_speed,
                sockets,
                virt_proc,
                virtualization,
                virt_machine,
                l1_cache,
                l2_cache,
                l3_cache,
                utilization,
                speed,
                processes,
                threads,
                handles,
                uptime
            )
        }

        fn populate_usage_graphs(&self, cpu_count: usize) {
            let base_color = self.obj().base_color();

            let col_count = Self::compute_column_count(cpu_count);

            let settings = self.settings.take();
            let mut graph_selection = GRAPH_SELECTION_OVERALL;
            let mut show_kernel_times = false;
            match settings {
                Some(settings) => {
                    graph_selection = settings.int("performance-page-cpu-graph");
                    show_kernel_times = settings.boolean("performance-page-kernel-times");

                    self.settings.set(Some(settings));
                }
                None => {}
            }

            // Add one for overall CPU utilization
            let mut graph_widgets = vec![];

            graph_widgets.push(GraphWidget::new());
            self.usage_graphs.attach(&graph_widgets[0], 0, 0, 1, 1);
            graph_widgets[0].set_data_points(60);
            graph_widgets[0].set_scroll(true);
            graph_widgets[0].set_data_set_count(2);
            graph_widgets[0].set_filled(1, false);
            graph_widgets[0].set_dashed(1, true);
            graph_widgets[0].set_data_visible(1, show_kernel_times);
            graph_widgets[0].set_base_color(&base_color);
            graph_widgets[0].set_visible(graph_selection == GRAPH_SELECTION_OVERALL);

            let this = self.obj().upcast_ref::<super::PerformancePageCpu>().clone();
            graph_widgets[0].connect_resize(move |_, _, _| {
                let graph_widgets = this.imp().graph_widgets.take();

                let width = graph_widgets[0].width() as f32;
                let height = graph_widgets[0].height() as f32;

                let mut a = width;
                let mut b = height;
                if width > height {
                    a = height;
                    b = width;
                }

                graph_widgets[0]
                    .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                this.imp().graph_widgets.set(graph_widgets);
            });

            self.overall_graph_labels
                .set_visible(graph_selection == GRAPH_SELECTION_OVERALL);
            self.utilization_label_overall
                .set_visible(graph_selection == GRAPH_SELECTION_OVERALL);
            self.utilization_label_all
                .set_visible(graph_selection == GRAPH_SELECTION_ALL);

            graph_widgets.push(GraphWidget::new());
            self.usage_graphs.attach(&graph_widgets[1], 0, 0, 1, 1);
            graph_widgets[1].set_data_points(60);
            graph_widgets[1].set_scroll(true);
            graph_widgets[1].set_data_set_count(cpu_count as u32);
            graph_widgets[1].set_base_color(&base_color);
            graph_widgets[1].set_visible(graph_selection == GRAPH_SELECTION_STACKED);

            for i in 0..cpu_count {
                let row_idx = i / col_count;
                let col_idx = i % col_count;

                let graph_widget_index = graph_widgets.len();

                graph_widgets.push(GraphWidget::new());
                if i == 0 {
                    let this = self.obj().upcast_ref::<super::PerformancePageCpu>().clone();
                    graph_widgets[graph_widget_index].connect_resize(move |_, _, _| {
                        let graph_widgets = this.imp().graph_widgets.take();

                        for graph_widget in graph_widgets.iter().skip(1) {
                            let width = graph_widget.width() as f32;
                            let height = graph_widget.height() as f32;

                            let mut a = width;
                            let mut b = height;
                            if width > height {
                                a = height;
                                b = width;
                            }

                            graph_widget.set_vertical_line_count(
                                (width * (a / b) / 30.).round().max(5.) as u32,
                            );
                        }

                        this.imp().graph_widgets.set(graph_widgets);
                    });
                }
                graph_widgets[graph_widget_index].set_data_points(60);
                graph_widgets[graph_widget_index].set_data_set_count(2);
                graph_widgets[graph_widget_index].set_filled(1, false);
                graph_widgets[graph_widget_index].set_dashed(1, true);
                graph_widgets[graph_widget_index].set_data_visible(1, show_kernel_times);
                graph_widgets[graph_widget_index].set_base_color(&base_color);
                graph_widgets[graph_widget_index]
                    .set_visible(graph_selection == GRAPH_SELECTION_ALL);
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

        fn compute_column_count(item_count: usize) -> usize {
            if item_count <= 3 {
                return item_count;
            }

            let sqrt_item_count = (item_count as f64).sqrt().round() as usize;
            for i in sqrt_item_count..item_count.min(sqrt_item_count * 2) {
                if item_count % i == 0 {
                    return i;
                }
            }

            sqrt_item_count
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
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageCpu>().clone();

            if let Some(app) = crate::MissionCenterApplication::default_instance() {
                self.settings.set(app.settings());
            }

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);

            let sidebar_content_builder = gtk::Builder::from_resource(
                "/io/missioncenter/MissionCenter/ui/performance_page/cpu_details.ui",
            );

            let _ = self.infobar_content.set(
                sidebar_content_builder
                    .object::<gtk::Box>("root")
                    .expect("Could not find `root` object in details pane"),
            );
            let _ = self.utilization.set(
                sidebar_content_builder
                    .object::<gtk::Label>("utilization")
                    .expect("Could not find `utilization` object in details pane"),
            );
            let _ = self.speed.set(
                sidebar_content_builder
                    .object::<gtk::Label>("speed")
                    .expect("Could not find `speed` object in details pane"),
            );
            let _ = self.processes.set(
                sidebar_content_builder
                    .object::<gtk::Label>("processes")
                    .expect("Could not find `processes` object in details pane"),
            );
            let _ = self.threads.set(
                sidebar_content_builder
                    .object::<gtk::Label>("threads")
                    .expect("Could not find `threads` object in details pane"),
            );
            let _ = self.handles.set(
                sidebar_content_builder
                    .object::<gtk::Label>("handles")
                    .expect("Could not find `handles` object in details pane"),
            );
            let _ = self.uptime.set(
                sidebar_content_builder
                    .object::<gtk::Label>("uptime")
                    .expect("Could not find `uptime` object in details pane"),
            );
            let _ = self.base_speed.set(
                sidebar_content_builder
                    .object::<gtk::Label>("base_speed")
                    .expect("Could not find `base_speed` object in details pane"),
            );
            let _ = self.sockets.set(
                sidebar_content_builder
                    .object::<gtk::Label>("sockets")
                    .expect("Could not find `sockets` object in details pane"),
            );
            let _ = self.virt_proc.set(
                sidebar_content_builder
                    .object::<gtk::Label>("virt_proc")
                    .expect("Could not find `virt_proc` object in details pane"),
            );
            let _ = self.virtualization.set(
                sidebar_content_builder
                    .object::<gtk::Label>("virtualization")
                    .expect("Could not find `virtualization` object in details pane"),
            );
            let _ = self.virt_machine.set(
                sidebar_content_builder
                    .object::<gtk::Label>("virt_machine")
                    .expect("Could not find `virt_machine` object in details pane"),
            );
            let _ = self.l1_cache.set(
                sidebar_content_builder
                    .object::<gtk::Label>("l1_cache")
                    .expect("Could not find `l1_cache` object in details pane"),
            );
            let _ = self.l2_cache.set(
                sidebar_content_builder
                    .object::<gtk::Label>("l2_cache")
                    .expect("Could not find `l2_cache` object in details pane"),
            );
            let _ = self.l3_cache.set(
                sidebar_content_builder
                    .object::<gtk::Label>("l3_cache")
                    .expect("Could not find `l3_cache` object in details pane"),
            );
        }
    }

    impl WidgetImpl for PerformancePageCpu {}

    impl BoxImpl for PerformancePageCpu {}
}

glib::wrapper! {
    pub struct PerformancePageCpu(ObjectSubclass<imp::PerformancePageCpu>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePageCpu {
    pub fn new(settings: &gio::Settings) -> Self {
        let this: Self = glib::Object::builder().build();

        fn update_refresh_rate_sensitive_labels(
            this: &PerformancePageCpu,
            settings: &gio::Settings,
        ) {
            let update_speed_ms = settings.int("update-speed") * 500;
            let graph_max_duration = (update_speed_ms * 60) / 1000;

            let this = this.imp();
            this.utilization_label_all.set_text(&i18n_f(
                "Utilization over {} seconds",
                &[&format!("{}", graph_max_duration)],
            ));
            this.graph_max_duration
                .set_text(&i18n_f("{} seconds", &[&format!("{}", graph_max_duration)]))
        }
        update_refresh_rate_sensitive_labels(&this, settings);

        settings.connect_changed(
            Some("update-speed"),
            clone!(@weak this => move |settings, _| {
                update_refresh_rate_sensitive_labels(&this, settings);
            }),
        );

        this
    }

    pub fn set_static_information(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        imp::PerformancePageCpu::set_static_information(self, readings)
    }

    pub fn update_readings(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        imp::PerformancePageCpu::update_readings(self, readings)
    }

    pub fn infobar_collapsed(&self) {
        self.imp()
            .infobar_content
            .get()
            .and_then(|ic| Some(ic.set_margin_top(10)));
    }

    pub fn infobar_uncollapsed(&self) {
        self.imp()
            .infobar_content
            .get()
            .and_then(|ic| Some(ic.set_margin_top(65)));
    }
}
