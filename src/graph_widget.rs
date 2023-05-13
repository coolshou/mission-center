/* graph_widget.rs
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

use glib::{ParamSpec, Properties, Value};
use gtk::{gdk, gdk::prelude::*, glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Clone)]
    pub struct DataSetDescriptor {
        pub dashed: bool,
        pub fill: bool,

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
        #[property(get, set)]
        base_color: Cell<gdk::RGBA>,
        #[property(get, set)]
        horizontal_line_count: Cell<u32>,
        #[property(get, set)]
        vertical_line_count: Cell<u32>,

        skia_context: Cell<Option<skia::gpu::DirectContext>>,
        pub data_sets: Cell<Vec<DataSetDescriptor>>,

        scroll_offset: Cell<f32>,
    }

    impl Default for GraphWidget {
        fn default() -> Self {
            const DATA_SET_LEN_DEFAULT: usize = 60;

            Self {
                data_points: Cell::new(DATA_SET_LEN_DEFAULT as _),
                data_set_count: Cell::new(1),
                value_range_min: Cell::new(0.),
                value_range_max: Cell::new(100.),
                auto_scale: Cell::new(false),
                auto_scale_pow2: Cell::new(false),
                grid_visible: Cell::new(true),
                scroll: Cell::new(false),
                base_color: Cell::new(gdk::RGBA::new(0., 0., 0., 1.)),
                horizontal_line_count: Cell::new(9),
                vertical_line_count: Cell::new(6),

                skia_context: Cell::new(None),
                data_sets: Cell::new(vec![
                    DataSetDescriptor {
                        dashed: false,
                        fill: true,
                        data_set: vec![0.0; DATA_SET_LEN_DEFAULT],
                    };
                    1
                ]),

                scroll_offset: Cell::new(0.),
            }
        }
    }

    impl GraphWidget {
        fn set_data_points(&self, count: u32) {
            let mut data_points = self.data_sets.take();
            for values in data_points.iter_mut() {
                values.data_set.resize(count as _, 0.);
            }
            self.data_sets.set(data_points);

            self.data_points.set(count);
        }

        fn set_data_sets(&self, count: u32) {
            let mut data_points = self.data_sets.take();
            data_points.resize(
                count as _,
                DataSetDescriptor {
                    dashed: false,
                    fill: true,
                    data_set: vec![0.; self.data_points.get() as _],
                },
            );
            self.data_sets.set(data_points);

            self.data_set_count.set(count);
        }
    }

    impl GraphWidget {
        fn realize(&self) -> Result<(), Box<dyn std::error::Error>> {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            this.make_current();

            let interface = skia::gpu::gl::Interface::new_native();
            self.skia_context.set(Some(
                skia::gpu::DirectContext::new_gl(interface, None)
                    .ok_or("Failed to create Skia DirectContext with OpenGL interface")?,
            ));

            this.set_auto_render(false);

            Ok(())
        }

        pub fn render_graph(
            &self,
            canvas: &mut skia::Canvas,
            width: i32,
            height: i32,
            scale_factor: f32,
        ) -> Result<(), Box<dyn std::error::Error>> {
            use skia::Paint;

            let data_sets = self.data_sets.take();
            let base_color = self.base_color.get();

            let mut outer_paint = Paint::new(
                skia::Color4f::new(base_color.red(), base_color.green(), base_color.blue(), 1.0),
                None,
            );
            outer_paint.set_stroke_width(scale_factor);
            outer_paint.set_style(skia::paint::Style::Stroke);

            let mut inner_paint = Paint::new(
                skia::Color4f::new(base_color.red(), base_color.green(), base_color.blue(), 0.2),
                None,
            );
            inner_paint.set_stroke_width(scale_factor);
            inner_paint.set_style(skia::paint::Style::Stroke);

            self.draw_outline(canvas, width, height, scale_factor, &outer_paint);

            if self.obj().grid_visible() {
                self.draw_grid(
                    canvas,
                    width,
                    height,
                    scale_factor,
                    self.obj().data_points() as _,
                    &inner_paint,
                );
            }

            outer_paint.set_anti_alias(true);
            inner_paint.set_style(skia::paint::Style::Fill);
            for values in data_sets.iter() {
                if values.dashed {
                    outer_paint.set_path_effect(skia::PathEffect::dash(
                        &[scale_factor * 5., scale_factor * 2.],
                        0.,
                    ));
                } else {
                    outer_paint.set_path_effect(None);
                }

                let inner_paint = if values.fill {
                    Some(&inner_paint)
                } else {
                    None
                };

                self.plot_values(
                    canvas,
                    width,
                    height,
                    scale_factor,
                    &values.data_set,
                    &outer_paint,
                    inner_paint,
                );
            }

            self.data_sets.set(data_sets);

            Ok(())
        }

        fn draw_outline(
            &self,
            canvas: &mut skia::canvas::Canvas,
            width: i32,
            height: i32,
            scale_factor: f32,
            paint: &skia::Paint,
        ) {
            let boundary = skia::Rect::new(
                scale_factor,
                scale_factor,
                width as f32 - scale_factor,
                height as f32 - scale_factor,
            );
            canvas.draw_rect(&boundary, &paint);
        }

        fn draw_grid(
            &self,
            canvas: &mut skia::canvas::Canvas,
            width: i32,
            height: i32,
            scale_factor: f32,
            data_point_count: usize,
            paint: &skia::Paint,
        ) {
            // Draw horizontal lines
            let horizontal_line_count = self.obj().horizontal_line_count() + 1;

            let col_width = width as f32 - scale_factor;
            let col_height = height as f32 / horizontal_line_count as f32;

            for i in 1..horizontal_line_count {
                canvas.draw_line(
                    (scale_factor, col_height * i as f32),
                    (col_width, col_height * i as f32),
                    &paint,
                );
            }

            // Draw vertical lines
            let mut vertical_line_count = self.obj().vertical_line_count() + 1;

            let col_width = width as f32 / vertical_line_count as f32;
            let col_height = height as f32 - scale_factor;

            let x_offset = if self.obj().scroll() {
                vertical_line_count += 1;

                let mut x_offset = self.scroll_offset.get();
                x_offset += (width as f32 / scale_factor) / data_point_count as f32;
                x_offset %= col_width;
                self.scroll_offset.set(x_offset);

                x_offset
            } else {
                0.
            };

            for i in 1..vertical_line_count {
                canvas.draw_line(
                    (col_width * i as f32 - x_offset, scale_factor),
                    (col_width * i as f32 - x_offset, col_height),
                    &paint,
                );
            }
        }

        fn plot_values(
            &self,
            canvas: &mut skia::Canvas,
            width: i32,
            height: i32,
            scale_factor: f32,
            data_points: &Vec<f32>,
            outer_paint: &skia::Paint,
            inner_paint: Option<&skia::Paint>,
        ) {
            let width = width as f32;
            let height = height as f32;

            let offset = -1. * self.value_range_min.get();
            let val_max = self.value_range_max.get() - offset;
            let val_min = self.value_range_min.get() - offset;

            let spacing_x = width / (data_points.len() - 1) as f32;
            let mut points = (0..).zip(data_points).map(|(x, y)| {
                (
                    x as f32 * spacing_x - 2. * scale_factor,
                    height - ((y.clamp(val_min, val_max) / val_max) * (height - 2. * scale_factor)),
                )
            });

            let mut path = skia::Path::new();
            if let Some((x, y)) = points.next() {
                let mut final_x = x;
                let mut prev = (x, y);

                path.move_to(skia::Point::new(x, y));
                for (x, y) in points {
                    path.line_to(skia::Point::new(x, y));
                    final_x = x;

                    canvas.draw_line(
                        skia::Point::new(prev.0, prev.1),
                        skia::Point::new(x, y),
                        outer_paint,
                    );
                    prev = (x, y);
                }

                canvas.draw_line(
                    skia::Point::new(prev.0, prev.1),
                    skia::Point::new(final_x, height),
                    outer_paint,
                );

                // Make sure to close out the path
                path.line_to(skia::Point::new(final_x, height));
                path.line_to(skia::Point::new(x, height));

                path.close();
            }

            if let Some(inner_paint) = inner_paint {
                canvas.draw_path(&path, &inner_paint);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GraphWidget {
        const NAME: &'static str = "GraphWidget";
        type Type = super::GraphWidget;
        type ParentType = gtk::GLArea;
    }

    impl ObjectImpl for GraphWidget {
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

    impl WidgetImpl for GraphWidget {
        fn realize(&self) {
            self.parent_realize();
            self.realize().expect("Failed to realize widget");
        }
    }

    impl GLAreaImpl for GraphWidget {
        fn render(&self, _: &gdk::GLContext) -> bool {
            use skia::*;

            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            let native = this.native().expect("Failed to get scale factor");

            let mut viewport_info: [gl_rs::types::GLint; 4] = [0; 4];
            unsafe {
                gl_rs::GetIntegerv(gl_rs::VIEWPORT, &mut viewport_info[0]);
            }
            let width = viewport_info[2];
            let height = viewport_info[3];

            unsafe {
                gl_rs::ClearColor(0.0, 0.0, 0.0, 0.0);
                gl_rs::Clear(gl_rs::COLOR_BUFFER_BIT);
            }

            let framebuffer_info = {
                use gl_rs::{types::*, *};

                let mut fboid: GLint = 0;
                unsafe { GetIntegerv(FRAMEBUFFER_BINDING, &mut fboid) };

                gpu::gl::FramebufferInfo {
                    fboid: fboid.try_into().unwrap(),
                    format: gpu::gl::Format::RGBA8.into(),
                }
            };

            let skia_render_target =
                gpu::BackendRenderTarget::new_gl((width, height), 0, 8, framebuffer_info);

            let skia_context = unsafe {
                (&mut *self.skia_context.as_ptr())
                    .as_mut()
                    .unwrap_unchecked()
            };
            let mut surface = Surface::from_backend_render_target(
                skia_context,
                &skia_render_target,
                gpu::SurfaceOrigin::BottomLeft,
                ColorType::RGBA8888,
                None,
                None,
            )
            .expect("Failed to create Skia surface");

            self.render_graph(surface.canvas(), width, height, native.scale_factor() as _)
                .expect("Failed to render");

            skia_context.flush_and_submit();

            return true;
        }
    }
}

glib::wrapper! {
    pub struct GraphWidget(ObjectSubclass<imp::GraphWidget>)
        @extends gtk::GLArea, gtk::Widget,
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

    pub fn add_data_point(&self, index: usize, value: f32) {
        fn round_up_to_next_power_of_two(num: u32) -> u32 {
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

        let mut data = self.imp().data_sets.take();
        if index < data.len() {
            data[index].data_set.push(value);
            data[index].data_set.remove(0);

            if self.auto_scale() {
                let mut max_y = value.max(self.value_range_max());

                let mut value_max = value;
                for data_set in data.iter() {
                    for value in data_set.data_set.iter() {
                        if value_max < *value {
                            value_max = *value;
                        }
                    }
                }

                while value_max < max_y {
                    max_y /= 2.;
                }
                if value_max > max_y {
                    max_y *= 2.;
                }

                if self.auto_scale_pow2() {
                    max_y = max_y.round();
                    if max_y < 0. {
                        max_y = -max_y;
                        max_y = round_up_to_next_power_of_two(max_y as u32) as f32 * -1.;
                    } else {
                        max_y = round_up_to_next_power_of_two(max_y as u32) as f32;
                    }
                }

                self.set_value_range_max(max_y);
            }
        }
        self.imp().data_sets.set(data);

        self.queue_render();
    }
}
