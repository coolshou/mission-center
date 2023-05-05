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
use gtk::{gio, glib, prelude::*};

use crate::graph_widget::GraphWidget;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageCpu)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/performance_page/cpu.ui")]
    pub struct PerformancePageCpu {
        #[template_child]
        pub usage_graphs: TemplateChild<gtk::Grid>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get, set)]
        refresh_interval: Cell<u32>,

        pub graph_widgets: Cell<Vec<GraphWidget>>,
    }

    impl Default for PerformancePageCpu {
        fn default() -> Self {
            Self {
                usage_graphs: Default::default(),
                context_menu: Default::default(),
                refresh_interval: Cell::new(1000),
                graph_widgets: std::cell::Cell::new(Vec::new()),
            }
        }
    }

    impl PerformancePageCpu {
        fn configure_actions(this: &super::PerformancePageCpu) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let action = gio::SimpleAction::new("overall", None);
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("all-processors", None);
            action.connect_activate(clone!(@weak this => move |action, parameter| {
                dbg!(action, parameter);
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new("kernel-times", None);
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
            this.imp()
                .usage_graphs
                .add_controller(right_click_controller);
        }

        fn populate_usage_graphs(&self) {
            use crate::SYS_INFO;
            use sysinfo::SystemExt;

            let cpu_count = SYS_INFO
                .read()
                .expect("Failed to read CPU count: Unable to acquire lock")
                .cpus()
                .len();

            let (_, col_count) = Self::compute_row_column_count(cpu_count);

            for i in 0..cpu_count {
                let row_idx = i / col_count;
                let col_idx = i % col_count;

                let graph_widgets = unsafe { &mut *self.graph_widgets.as_ptr() };
                let graph_widget_index = graph_widgets.len();

                graph_widgets.push(crate::graph_widget::GraphWidget::new());
                graph_widgets[graph_widget_index].set_base_color(gtk::gdk::RGBA::new(
                    crate::CPU_USAGE_GRAPH_BASE_COLOR[0],
                    crate::CPU_USAGE_GRAPH_BASE_COLOR[1],
                    crate::CPU_USAGE_GRAPH_BASE_COLOR[2],
                    1.,
                ));
                self.usage_graphs.attach(
                    &graph_widgets[graph_widget_index],
                    col_idx as i32,
                    row_idx as i32,
                    1,
                    1,
                );
            }
        }

        fn update_graph(this: &super::PerformancePageCpu) {
            use crate::SYS_INFO;
            use sysinfo::{CpuExt, SystemExt};

            let sys_info = SYS_INFO
                .read()
                .expect("Failed to read CPU information: Unable to acquire lock");

            for (i, cpu) in sys_info.cpus().iter().enumerate() {
                let graph_widgets = unsafe { &mut *this.imp().graph_widgets.as_ptr() };
                let graph_widget = &mut graph_widgets[i];
                graph_widget.add_data_point(cpu.cpu_usage());
            }

            let this = this.clone();
            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_graph(&this);
                },
            ));
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
                if (i - j).abs() <= 1 {
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

    impl WidgetImpl for PerformancePageCpu {
        fn realize(&self) {
            self.parent_realize();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageCpu>().clone();

            Self::configure_context_menu(&this);
            self.populate_usage_graphs();

            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_graph(&this);
                },
            ));
        }
    }

    impl BoxImpl for PerformancePageCpu {}
}

glib::wrapper! {
    pub struct PerformancePageCpu(ObjectSubclass<imp::PerformancePageCpu>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}
