/* performance_cpu.rs
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
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/performance_cpu.ui")]
    pub struct PerformanceCpu {
        #[template_child]
        pub usage_graphs: TemplateChild<gtk::Grid>,

        pub graph_widgets: std::cell::Cell<Vec<crate::graph_widget::GraphWidget>>,
    }

    impl Default for PerformanceCpu {
        fn default() -> Self {
            Self {
                usage_graphs: Default::default(),
                graph_widgets: std::cell::Cell::new(Vec::new()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformanceCpu {
        const NAME: &'static str = "PerformanceCpu";
        type Type = super::PerformanceCpu;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformanceCpu {}

    impl WidgetImpl for PerformanceCpu {
        fn realize(&self) {
            use crate::SYS_INFO;
            use sysinfo::{CpuExt, SystemExt};

            self.parent_realize();

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

                let result =
                    if let Some((i, j)) = valid_factors.into_iter().max_by_key(|&(i, j)| i * j) {
                        (i, j)
                    } else {
                        let i = item_count.min(((item_count as f64).sqrt() + 1.).floor() as isize);
                        let j = ((item_count as f64) / i as f64).ceil() as isize;
                        (i, j)
                    };

                (result.0 as usize, result.1 as usize)
            }

            let cpu_count = SYS_INFO
                .read()
                .expect("Failed to read CPU count: Unable to acquire lock")
                .cpus()
                .len();

            let (_, col_count) = compute_row_column_count(cpu_count);

            for i in 0..cpu_count {
                let row_idx = i / col_count;
                let col_idx = i % col_count;

                let graph_widgets = unsafe { &mut *self.graph_widgets.as_ptr() };
                let graph_widget_index = graph_widgets.len();

                graph_widgets.push(crate::graph_widget::GraphWidget::new());
                self.usage_graphs.attach(
                    &graph_widgets[graph_widget_index],
                    col_idx as i32,
                    row_idx as i32,
                    1,
                    1,
                );
            }

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformanceCpu>().clone();
            Some(glib::source::timeout_add_local(
                std::time::Duration::from_millis(1000),
                move || {
                    let sys_info = SYS_INFO
                        .read()
                        .expect("Failed to read CPU information: Unable to acquire lock");

                    for (i, cpu) in sys_info.cpus().iter().enumerate() {
                        let graph_widgets = unsafe { &mut *this.imp().graph_widgets.as_ptr() };
                        let graph_widget = &mut graph_widgets[i];
                        graph_widget.add_data_point(cpu.cpu_usage());
                    }

                    Continue(true)
                },
            ));
        }
    }

    impl BoxImpl for PerformanceCpu {}
}

glib::wrapper! {
    pub struct PerformanceCpu(ObjectSubclass<imp::PerformanceCpu>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}
