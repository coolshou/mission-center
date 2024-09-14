/* performance_page/widgets/graph_widget.rs
 *
 * Copyright 2024 Romeo Calota
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

use glib::{ParamSpec, Properties, Value};
use gtk::{
    gdk,
    gdk::prelude::*,
    glib::{
        self,
        subclass::{prelude::*, Signal},
    },
    graphene,
    gsk::{self, FillRule, PathBuilder, Stroke},
    prelude::*,
    subclass::prelude::*,
    Snapshot,
};

pub use imp::DataSetDescriptor;

use super::GRAPH_RADIUS;

mod imp {
    use super::*;

    #[derive(Clone)]
    pub struct DataSetDescriptor {
        pub dashed: bool,
        pub fill: bool,
        pub visible: bool,

        pub data_set: Vec<f32>,
    }

    #[derive(Properties)]
    #[properties(wrapper_type = super::GraphWidget)]
    pub struct GraphWidget {
        #[property(get, set = Self::set_data_points)]
        data_points: Cell<u32>,
        #[property(get, set = Self::set_data_sets)]
        data_set_count: Cell<u32>,
        #[property(get, set)]
        value_range_min: Cell<f32>,
        #[property(get, set)]
        value_range_max: Cell<f32>,
        #[property(get, set)]
        auto_scale: Cell<bool>,
        #[property(get, set)]
        auto_scale_pow2: Cell<bool>,
        #[property(get, set)]
        grid_visible: Cell<bool>,
        #[property(get, set)]
        scroll: Cell<bool>,
        #[property(get, set = Self::set_smooth_graphs)]
        smooth_graphs: Cell<bool>,
        #[property(get, set)]
        base_color: Cell<gdk::RGBA>,
        #[property(get, set = Self::set_horizontal_line_count)]
        horizontal_line_count: Cell<u32>,
        #[property(get, set = Self::set_vertical_line_count)]
        vertical_line_count: Cell<u32>,

        pub data_sets: Cell<Vec<DataSetDescriptor>>,

        scroll_offset: Cell<f32>,
        prev_size: Cell<(i32, i32)>,
    }

    impl Default for GraphWidget {
        fn default() -> Self {
            const DATA_SET_LEN_DEFAULT: usize = 60;

            let mut data_set = vec![0.0; DATA_SET_LEN_DEFAULT];
            data_set.reserve(1);

            Self {
                data_points: Cell::new(DATA_SET_LEN_DEFAULT as _),
                data_set_count: Cell::new(1),
                value_range_min: Cell::new(0.),
                value_range_max: Cell::new(100.),
                auto_scale: Cell::new(false),
                auto_scale_pow2: Cell::new(false),
                grid_visible: Cell::new(true),
                scroll: Cell::new(false),
                smooth_graphs: Cell::new(false),
                base_color: Cell::new(gdk::RGBA::new(0., 0., 0., 1.)),
                horizontal_line_count: Cell::new(9),
                vertical_line_count: Cell::new(6),

                data_sets: Cell::new(vec![DataSetDescriptor {
                    dashed: false,
                    fill: true,
                    visible: true,
                    data_set,
                }]),

                scroll_offset: Cell::new(0.),
                prev_size: Cell::new((0, 0)),
            }
        }
    }

    impl GraphWidget {
        fn set_data_points(&self, count: u32) {
            if self.data_points.take() != count {
                let mut data_points = self.data_sets.take();
                for values in data_points.iter_mut() {
                    if count == (values.data_set.len() as u32) {
                        continue;
                    }
                    // we need to truncate from the correct side
                    values.data_set.reverse();
                    values.data_set.resize(count as _, 0.);
                    values.data_set.reverse();
                }
                self.data_sets.set(data_points);
            }
            self.data_points.set(count);
        }

        fn set_data_sets(&self, count: u32) {
            let mut data_points = self.data_sets.take();
            data_points.resize(
                count as _,
                DataSetDescriptor {
                    dashed: false,
                    fill: true,
                    visible: true,
                    data_set: vec![0.; self.data_points.get() as _],
                },
            );
            self.data_sets.set(data_points);

            self.data_set_count.set(count);
        }

        fn set_horizontal_line_count(&self, count: u32) {
            if self.horizontal_line_count.get() != count {
                self.horizontal_line_count.set(count);
                self.obj().upcast_ref::<super::GraphWidget>().queue_draw();
            }
        }

        fn set_vertical_line_count(&self, count: u32) {
            if self.vertical_line_count.get() != count {
                self.vertical_line_count.set(count);
                self.obj().upcast_ref::<super::GraphWidget>().queue_draw();
            }
        }

        fn set_smooth_graphs(&self, smooth: bool) {
            if self.smooth_graphs.get() != smooth {
                self.smooth_graphs.set(smooth);
                self.obj().upcast_ref::<super::GraphWidget>().queue_draw();
            }
        }
    }

    impl GraphWidget {
        #[inline]
        fn draw_outline(&self, snapshot: &Snapshot, bounds: &gsk::RoundedRect, color: &gdk::RGBA) {
            let stroke_color = gdk::RGBA::new(color.red(), color.green(), color.blue(), 1.);
            snapshot.append_border(&bounds, &[1.; 4], &[stroke_color.clone(); 4]);
        }

        #[inline]
        fn draw_grid(
            &self,
            snapshot: &Snapshot,
            width: f32,
            height: f32,
            scale_factor: f64,
            data_point_count: usize,
            color: &gdk::RGBA,
        ) {
            let scale_factor = scale_factor as f32;
            let color = gdk::RGBA::new(color.red(), color.green(), color.blue(), 51. / 256.);

            let stroke = Stroke::new(1.);

            // Draw horizontal lines
            let horizontal_line_count = self.obj().horizontal_line_count() + 1;

            let col_width = width - scale_factor;
            let col_height = height / horizontal_line_count as f32;

            for i in 1..horizontal_line_count {
                let path_builder = PathBuilder::new();
                path_builder.move_to(scale_factor / 2., col_height * i as f32);
                path_builder.line_to(col_width, col_height * i as f32);
                snapshot.append_stroke(&path_builder.to_path(), &stroke, &color);
            }

            // Draw vertical lines
            let mut vertical_line_count = self.obj().vertical_line_count() + 1;

            let col_width = width / vertical_line_count as f32;
            let col_height = height - scale_factor;

            let x_offset = if self.obj().scroll() {
                vertical_line_count += 1;

                let mut x_offset = self.scroll_offset.get();
                x_offset += width / data_point_count as f32;
                x_offset %= col_width;
                self.scroll_offset.set(x_offset);

                x_offset
            } else {
                0.
            };

            for i in 1..vertical_line_count {
                let path_builder = PathBuilder::new();
                path_builder.move_to(col_width * i as f32 - x_offset, scale_factor / 2.);
                path_builder.line_to(col_width * i as f32 - x_offset, col_height);
                snapshot.append_stroke(&path_builder.to_path(), &stroke, &color);
            }
        }

        #[inline]
        fn plot_values(
            &self,
            snapshot: &Snapshot,
            width: f32,
            height: f32,
            scale_factor: f64,
            data_points: &DataSetDescriptor,
            color: &gdk::RGBA,
        ) {
            let scale_factor = scale_factor as f32;
            let stroke_color = gdk::RGBA::new(color.red(), color.green(), color.blue(), 1.);
            let fill_color = gdk::RGBA::new(color.red(), color.green(), color.blue(), 100. / 256.);

            let stroke = Stroke::new(1.);

            let offset = -1. * self.value_range_min.get();
            let val_max = self.value_range_max.get() - offset;
            let val_min = self.value_range_min.get() - offset;

            let spacing_x = width / (data_points.data_set.len() - 1) as f32;
            let points: Vec<(f32, f32)> = (0..)
                .zip(&data_points.data_set)
                .skip_while(|(_, y)| **y <= scale_factor)
                .map(|(x, y)| {
                    let x = x as f32 * spacing_x;
                    let y = height - ((y.clamp(val_min, val_max) / val_max) * (height));

                    (x, y)
                })
                .collect();

            if !points.is_empty() {
                let startindex;
                let (mut x, mut y);
                let pointlen = points.len();

                if pointlen < data_points.data_set.len() {
                    (x, y) = (
                        (data_points.data_set.len() - pointlen - 1) as f32 * spacing_x,
                        height,
                    );
                    startindex = 0;
                } else {
                    (x, y) = points[0];
                    startindex = 1;
                }
                let path_builder = PathBuilder::new();
                path_builder.move_to(x, y);

                let smooth = self.smooth_graphs.get();

                for i in startindex..pointlen {
                    (x, y) = points[i];
                    if smooth {
                        let (lastx, lasty);
                        if i > 0 {
                            (lastx, lasty) = points[i - 1];
                        } else {
                            (lastx, lasty) = (x - spacing_x, height);
                        }

                        path_builder.cubic_to(
                            lastx + spacing_x / 2f32,
                            lasty,
                            lastx + spacing_x / 2f32,
                            y,
                            x,
                            y,
                        );
                    } else {
                        path_builder.line_to(x, y);
                    }
                }

                // Make sure to close out the path
                path_builder.line_to(points[pointlen - 1].0, height);
                path_builder.line_to(points[0].0, height);
                path_builder.close();

                let path = path_builder.to_path();

                if data_points.fill {
                    snapshot.append_fill(&path, FillRule::Winding, &fill_color);
                }

                if data_points.dashed {
                    stroke.set_dash(&[5., 5.]);
                }

                snapshot.append_stroke(&path, &stroke, &stroke_color);
            }
        }

        fn render(&self, snapshot: &Snapshot, width: f32, height: f32, scale_factor: f64) {
            let data_sets = self.data_sets.take();
            let base_color = self.base_color.get();

            let radius = graphene::Size::new(GRAPH_RADIUS, GRAPH_RADIUS);
            let bounds = gsk::RoundedRect::new(
                graphene::Rect::new(0., 0., width, height),
                radius,
                radius,
                radius,
                radius,
            );

            gtk::prelude::SnapshotExt::push_rounded_clip(snapshot, &bounds);

            if self.obj().grid_visible() {
                self.draw_grid(
                    snapshot,
                    width,
                    height,
                    scale_factor,
                    self.obj().data_points() as _,
                    &base_color,
                );
            }

            for values in data_sets.iter() {
                if !values.visible {
                    continue;
                }

                self.plot_values(snapshot, width, height, scale_factor, &values, &base_color);
            }

            gtk::prelude::SnapshotExt::pop(snapshot);

            self.draw_outline(snapshot, &bounds, &base_color);

            self.data_sets.set(data_sets);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GraphWidget {
        const NAME: &'static str = "GraphWidget";
        type Type = super::GraphWidget;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for GraphWidget {
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn signals() -> &'static [Signal] {
            use std::sync::OnceLock;
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("resize").build()])
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for GraphWidget {
        fn realize(&self) {
            self.parent_realize();
        }

        fn snapshot(&self, snapshot: &Snapshot) {
            use glib::g_critical;

            let this = self.obj();

            let native = match this.native() {
                Some(native) => native,
                None => {
                    g_critical!("MissionCenter::GraphWidget", "Failed to get native");
                    return;
                }
            };

            let surface = match native.surface() {
                Some(surface) => surface,
                None => {
                    g_critical!("MissionCenter::GraphWidget", "Failed to get surface");
                    return;
                }
            };

            let (prev_width, prev_height) = self.prev_size.get();
            let (width, height) = (this.width(), this.height());

            if prev_width != width || prev_height != height {
                this.emit_by_name::<()>("resize", &[]);
                self.prev_size.set((width, height));
            }

            self.render(snapshot, width as f32, height as f32, surface.scale());
        }
    }
}

glib::wrapper! {
    pub struct GraphWidget(ObjectSubclass<imp::GraphWidget>)
        @extends gtk::Widget,
        @implements gtk::Buildable;
}

impl GraphWidget {
    pub fn new() -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(GraphWidget::static_type(), &mut [])
                .downcast()
                .unwrap()
        };
        this
    }

    pub fn set_dashed(&self, index: usize, dashed: bool) {
        let mut data = self.imp().data_sets.take();
        if index < data.len() {
            data[index].dashed = dashed;
        }
        self.imp().data_sets.set(data);
    }

    pub fn set_filled(&self, index: usize, filled: bool) {
        let mut data = self.imp().data_sets.take();
        if index < data.len() {
            data[index].fill = filled;
        }
        self.imp().data_sets.set(data);
    }

    pub fn set_data_visible(&self, index: usize, visible: bool) {
        let mut data = self.imp().data_sets.take();
        if index < data.len() {
            data[index].visible = visible;
        }
        self.imp().data_sets.set(data);
    }

    pub fn add_data_point(&self, index: usize, mut value: f32) {
        if value.is_infinite()
            || value.is_nan()
            || value.is_subnormal()
            || value < self.value_range_min()
        {
            value = self.value_range_min();
        }

        let mut data = self.imp().data_sets.take();
        if index < data.len() {
            data[index].data_set.push(value);
            data[index].data_set.remove(0);

            if self.auto_scale() {
                self.scale(&mut data, value);
            } else if value > self.value_range_max() {
                let data_set = &mut data[index].data_set;
                data_set.remove(data_set.len() - 1);
                data_set.push(self.value_range_max());
            }
        }
        self.imp().data_sets.set(data);

        if self.is_visible() {
            self.queue_draw();
        }
    }

    pub fn set_data(&self, index: usize, mut values: Vec<f32>) {
        use std::cmp::Ordering;

        let imp = self.imp();

        let mut data = imp.data_sets.take();
        if index < data.len() {
            values.truncate(data[index].data_set.len());
            data[index].data_set = values;

            if self.auto_scale() {
                let max = *data[index]
                    .data_set
                    .iter()
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                    .unwrap_or(&0.0);
                self.scale(&mut data, max);
            }
        }

        imp.data_sets.set(data);
    }

    pub fn data(&self, index: usize) -> Option<Vec<f32>> {
        let imp = self.imp();

        let data = imp.data_sets.take();
        let result = if index < data.len() {
            Some(data[index].data_set.clone())
        } else {
            None
        };
        imp.data_sets.set(data);

        result
    }

    fn scale(&self, data: &mut Vec<DataSetDescriptor>, value: f32) {
        fn round_up_to_next_power_of_two(num: u64) -> u64 {
            if num == 0 {
                return 0;
            }

            let mut n = num - 1;
            n |= n >> 1;
            n |= n >> 2;
            n |= n >> 4;
            n |= n >> 8;
            n |= n >> 16;

            n + 1
        }

        let mut max_y = value.max(self.value_range_max());

        let mut value_max = value;
        for data_set in data.iter() {
            for value in data_set.data_set.iter() {
                if value_max < *value {
                    value_max = *value;
                }
            }
        }

        let value_max_abs = value_max.abs();
        let mut max_y_abs = max_y.abs();

        while value_max_abs < max_y_abs {
            max_y_abs /= 2.;
        }
        if value_max_abs > max_y_abs {
            max_y_abs *= 2.;
        }

        if self.auto_scale_pow2() {
            max_y_abs = max_y_abs.round();
            max_y_abs = round_up_to_next_power_of_two(max_y_abs as u64) as f32;
        }

        max_y = if max_y < 0. {
            max_y_abs * -1.
        } else {
            max_y_abs
        };

        self.set_value_range_max(max_y);
    }
}
