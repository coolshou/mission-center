/* performance_page/widgets/graph_widget.rs
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

use crate::performance_page::widgets::graph_widget::imp::DataSetDescriptor;

mod imp {
    use pathfinder_gl::GLDevice;
    use pathfinder_renderer::gpu::renderer::Renderer;

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
        #[property(get, set)]
        base_color: Cell<gdk::RGBA>,
        #[property(get, set = Self::set_horizontal_line_count)]
        horizontal_line_count: Cell<u32>,
        #[property(get, set = Self::set_vertical_line_count)]
        vertical_line_count: Cell<u32>,

        renderer: Cell<Option<Renderer<GLDevice>>>,
        render_function: Cell<fn(&Self, width: i32, height: i32, scale_factor: f32)>,

        pub data_sets: Cell<Vec<DataSetDescriptor>>,

        scroll_offset: Cell<f32>,
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
                base_color: Cell::new(gdk::RGBA::new(0., 0., 0., 1.)),
                horizontal_line_count: Cell::new(9),
                vertical_line_count: Cell::new(6),

                renderer: Cell::new(None),
                render_function: Cell::new(Self::render_init_pathfinder),

                data_sets: Cell::new(vec![DataSetDescriptor {
                    dashed: false,
                    fill: true,
                    visible: true,
                    data_set,
                }]),

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
    }

    impl GraphWidget {
        #[inline]
        fn draw_outline(
            &self,
            canvas: &mut pathfinder_canvas::CanvasRenderingContext2D,
            width: i32,
            height: i32,
            scale_factor: f32,
            color: &gdk::RGBA,
        ) {
            use pathfinder_canvas::*;

            canvas.set_stroke_style(FillStyle::Color(ColorU::new(
                (color.red() * 255.) as u8,
                (color.green() * 255.) as u8,
                (color.blue() * 255.) as u8,
                255,
            )));
            canvas.set_line_dash(vec![]);
            canvas.stroke_rect(RectF::new(
                vec2f(scale_factor / 2., scale_factor / 2.),
                vec2f(width as f32 - scale_factor, height as f32 - scale_factor),
            ));
        }

        #[inline]
        fn draw_grid(
            &self,
            canvas: &mut pathfinder_canvas::CanvasRenderingContext2D,
            width: i32,
            height: i32,
            scale_factor: f32,
            data_point_count: usize,
            color: &gdk::RGBA,
        ) {
            use pathfinder_canvas::*;

            canvas.set_stroke_style(FillStyle::Color(ColorU::new(
                (color.red() * 255.) as u8,
                (color.green() * 255.) as u8,
                (color.blue() * 255.) as u8,
                51,
            )));

            // Draw horizontal lines
            let horizontal_line_count = self.obj().horizontal_line_count() + 1;

            let col_width = width as f32 - scale_factor;
            let col_height = height as f32 / horizontal_line_count as f32;

            for i in 1..horizontal_line_count {
                let mut path = Path2D::new();
                path.move_to(vec2f(scale_factor / 2., col_height * i as f32));
                path.line_to(vec2f(col_width, col_height * i as f32));
                canvas.stroke_path(path);
            }

            // Draw vertical lines
            let mut vertical_line_count = self.obj().vertical_line_count() + 1;

            let col_width = width as f32 / vertical_line_count as f32;
            let col_height = height as f32 - scale_factor;

            let x_offset = if self.obj().scroll() {
                vertical_line_count += 1;

                let mut x_offset = self.scroll_offset.get();
                x_offset += width as f32 / data_point_count as f32;
                x_offset %= col_width;
                self.scroll_offset.set(x_offset);

                x_offset
            } else {
                0.
            };

            for i in 1..vertical_line_count {
                let mut path = Path2D::new();
                path.move_to(vec2f(col_width * i as f32 - x_offset, scale_factor / 2.));
                path.line_to(vec2f(col_width * i as f32 - x_offset, col_height));
                canvas.stroke_path(path);
            }
        }

        #[inline]
        fn plot_values(
            &self,
            canvas: &mut pathfinder_canvas::CanvasRenderingContext2D,
            width: i32,
            height: i32,
            scale_factor: f32,
            data_points: &DataSetDescriptor,
            color: &gdk::RGBA,
        ) {
            use pathfinder_canvas::*;

            let width = width as f32;
            let height = height as f32;

            let offset = -1. * self.value_range_min.get();
            let val_max = self.value_range_max.get() - offset;
            let val_min = self.value_range_min.get() - offset;

            let spacing_x = width / (data_points.data_set.len() - 1) as f32;
            let mut points = (0..)
                .zip(&data_points.data_set)
                .skip_while(|(_, y)| **y <= scale_factor)
                .map(|(x, y)| {
                    (
                        x as f32 * spacing_x - scale_factor / 2.,
                        height
                            - ((y.clamp(val_min, val_max) / val_max)
                                * (height - scale_factor / 2.)),
                    )
                });

            canvas.set_stroke_style(FillStyle::Color(ColorU::new(
                (color.red() * 255.) as u8,
                (color.green() * 255.) as u8,
                (color.blue() * 255.) as u8,
                255,
            )));

            if let Some((x, y)) = points.next() {
                let mut final_x = x;

                let mut path = Path2D::new();
                path.move_to(vec2f(x, y));

                for (x, y) in points {
                    path.line_to(vec2f(x, y));
                    final_x = x;
                }

                // Make sure to close out the path
                path.line_to(vec2f(final_x, height));
                path.line_to(vec2f(x, height));
                path.close_path();

                if data_points.fill {
                    canvas.set_fill_style(FillStyle::Color(ColorU::new(
                        (color.red() * 255.) as u8,
                        (color.green() * 255.) as u8,
                        (color.blue() * 255.) as u8,
                        100,
                    )));
                    canvas.fill_path(path.clone(), FillRule::Winding);
                }

                if data_points.dashed {
                    canvas.set_line_dash(vec![scale_factor * 5., scale_factor * 2.]);
                } else {
                    canvas.set_line_dash(vec![]);
                }

                canvas.stroke_path(path);
            }
        }

        fn render_init_pathfinder(&self, width: i32, height: i32, scale_factor: f32) {
            use pathfinder_canvas::*;
            use pathfinder_gl::*;
            use pathfinder_renderer::gpu::{options::*, renderer::*};
            use pathfinder_resources::embedded::*;

            let mut fboid: gl::types::GLint = 0;
            unsafe {
                gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid);
            }

            let device = GLDevice::new(GLVersion::GL3, fboid as _);
            let mode = RendererMode::default_for_device(&device);

            let framebuffer_size = Vector2I::new(width, height);
            let options = RendererOptions {
                dest: DestFramebuffer::full_window(framebuffer_size.clone()),
                background_color: None,
                ..RendererOptions::default()
            };

            self.renderer.set(Some(Renderer::new(
                device,
                &EmbeddedResourceLoader::new(),
                mode,
                options,
            )));

            self.render_function.set(Self::render_all);
            self.render_all(width, height, scale_factor);
        }

        fn render_all(&self, width: i32, height: i32, scale_factor: f32) {
            use pathfinder_canvas::*;
            use pathfinder_renderer::{concurrent::*, gpu::options::*, options::*};

            let framebuffer_size = Vector2I::new(width, height);

            let mut renderer = self.renderer.take().expect("Uninitialized renderer");
            renderer.options_mut().dest = DestFramebuffer::full_window(framebuffer_size);

            let mut canvas =
                Canvas::new(framebuffer_size.to_f32()).get_context_2d(CanvasFontContext {});

            let data_sets = self.data_sets.take();
            let base_color = self.base_color.get();

            canvas.set_line_width(scale_factor as f32);

            if self.obj().grid_visible() {
                self.draw_grid(
                    &mut canvas,
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

                self.plot_values(
                    &mut canvas,
                    width,
                    height,
                    scale_factor,
                    &values,
                    &base_color,
                );
            }

            self.draw_outline(&mut canvas, width, height, scale_factor, &base_color);

            canvas.into_canvas().into_scene().build_and_render(
                &mut renderer,
                BuildOptions::default(),
                executor::SequentialExecutor,
            );

            self.data_sets.set(data_sets);
            self.renderer.set(Some(renderer));
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
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            self.parent_realize();

            this.set_has_stencil_buffer(true);
        }
    }

    impl GLAreaImpl for GraphWidget {
        fn render(&self, _: &gdk::GLContext) -> bool {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            let scale_factor = this.scale_factor();
            let mut viewport_info: [gl::types::GLint; 4] = [0; 4];
            unsafe {
                gl::GetIntegerv(gl::VIEWPORT, &mut viewport_info[0]);
            }
            let width = viewport_info[2];
            let height = viewport_info[3];

            unsafe {
                gl::ClearColor(0.0, 0.0, 0.0, 0.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);
            }

            (self.render_function.get())(self, width, height, scale_factor as f32);

            true
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

    pub fn set_data_visible(&self, index: usize, visible: bool) {
        let mut data = self.imp().data_sets.take();
        if index < data.len() {
            data[index].visible = visible;
        }
        self.imp().data_sets.set(data);
    }

    pub fn add_data_point(&self, index: usize, value: f32) {
        let mut data = self.imp().data_sets.take();
        if index < data.len() {
            data[index].data_set.push(value);
            data[index].data_set.remove(0);

            if self.auto_scale() {
                self.scale(&mut data, value);
            }
        }
        self.imp().data_sets.set(data);

        if self.is_visible() {
            self.queue_render();
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
                max_y = round_up_to_next_power_of_two(max_y as u64) as f32 * -1.;
            } else {
                max_y = round_up_to_next_power_of_two(max_y as u64) as f32;
            }
        }

        self.set_value_range_max(max_y);
    }
}
