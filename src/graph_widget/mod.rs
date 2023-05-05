/* graph_widget/mod.rs
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

mod skia_plotter_backend;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::GraphWidget)]
    pub struct GraphWidget {
        #[property(get, set)]
        grid_visible: Cell<bool>,
        #[property(get, set)]
        scroll: Cell<bool>,
        #[property(get, set)]
        base_color: Cell<gdk::RGBA>,

        skia_context: Cell<Option<skia_safe::gpu::DirectContext>>,
        pub(crate) data_points: Cell<Vec<f32>>,

        scroll_offset: Cell<f32>,
        base_color_rgb: Cell<[u8; 3]>,
        base_color_rgbf: Cell<[f32; 3]>,
    }

    impl Default for GraphWidget {
        fn default() -> Self {
            Self {
                grid_visible: Cell::new(true),
                scroll: Cell::new(false),
                base_color: Cell::new(gdk::RGBA::new(0., 0., 0., 1.)),

                skia_context: Cell::new(None),
                data_points: Cell::new(vec![0.0; 60]),

                scroll_offset: Cell::new(0.),
                base_color_rgb: Cell::new([0, 0, 0]),
                base_color_rgbf: Cell::new([0., 0., 0.]),
            }
        }
    }

    impl GraphWidget {
        fn realize(&self) -> Result<(), Box<dyn std::error::Error>> {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            this.make_current();

            let interface = skia_safe::gpu::gl::Interface::new_native();
            self.skia_context.set(Some(
                skia_safe::gpu::DirectContext::new_gl(interface, None)
                    .ok_or("Failed to create Skia DirectContext with OpenGL interface")?,
            ));

            this.set_auto_render(false);

            Ok(())
        }

        pub fn render_graph(
            &self,
            canvas: &mut skia_safe::canvas::Canvas,
            width: i32,
            height: i32,
            scale_factor: f32,
        ) -> Result<(), Box<dyn std::error::Error>> {
            use plotters::prelude::*;

            let data_points = unsafe { &*(self.data_points.as_ptr()) };

            let rgb = self.base_color_rgb.get();
            let rgbf = self.base_color_rgbf.get();

            let mut outer_paint = skia_safe::Paint::new(
                skia_safe::Color4f::new(rgbf[0], rgbf[1], rgbf[2], 1.0),
                None,
            );
            outer_paint.set_stroke_width(scale_factor);
            outer_paint.set_style(skia_safe::paint::Style::Stroke);

            let mut inner_paint = skia_safe::Paint::new(
                skia_safe::Color4f::new(rgbf[0], rgbf[1], rgbf[2], 0.2),
                None,
            );
            inner_paint.set_stroke_width(scale_factor);
            inner_paint.set_style(skia_safe::paint::Style::Stroke);

            let outer_color = RGBAColor(rgb[0], rgb[1], rgb[2], 1.0);
            let inner_color = RGBAColor(rgb[0], rgb[1], rgb[2], 0.2);

            self.draw_outline(canvas, width, height, scale_factor, &outer_paint);

            if self.obj().grid_visible() {
                self.draw_grid(
                    canvas,
                    width,
                    height,
                    scale_factor,
                    data_points.len(),
                    &inner_paint,
                );
            }

            let backend = skia_plotter_backend::SkiaPlotterBackend::new(
                canvas,
                width as _,
                height as _,
                scale_factor,
            );
            let root = plotters::drawing::IntoDrawingArea::into_drawing_area(backend);

            let mut chart = ChartBuilder::on(&root)
                .set_all_label_area_size(0)
                .build_cartesian_2d(0..data_points.len() - 1, 0_f32..100_f32)?;

            chart
                .configure_mesh()
                .disable_mesh()
                .disable_axes()
                .draw()?;

            chart.draw_series(
                AreaSeries::new(
                    (0..).zip(data_points.iter()).map(|(x, y)| (x, *y)),
                    0.0,
                    &inner_color,
                )
                .border_style(&outer_color),
            )?;

            root.present()?;

            Ok(())
        }

        fn draw_outline(
            &self,
            canvas: &mut skia_safe::canvas::Canvas,
            width: i32,
            height: i32,
            scale_factor: f32,
            paint: &skia_safe::Paint,
        ) {
            let boundary = skia_safe::Rect::new(
                scale_factor,
                scale_factor,
                width as f32 - scale_factor,
                height as f32 - scale_factor,
            );
            canvas.draw_rect(&boundary, &paint);
        }

        fn draw_grid(
            &self,
            canvas: &mut skia_safe::canvas::Canvas,
            width: i32,
            height: i32,
            scale_factor: f32,
            data_point_count: usize,
            paint: &skia_safe::Paint,
        ) {
            // Draw horizontal lines
            let horizontal_line_count = 10;

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
            let mut vertical_line_count = 6;

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

            if pspec.name() == "base-color" || pspec.name() == "base_color" {
                let base_color = self.obj().base_color();
                self.base_color_rgb.set([
                    ((base_color.red() * 255.) % 256.).floor() as u8,
                    ((base_color.green() * 255.) % 256.).floor() as u8,
                    ((base_color.blue() * 255.) % 256.).floor() as u8,
                ]);
                self.base_color_rgbf
                    .set([base_color.red(), base_color.green(), base_color.blue()]);
            }
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
            use skia_safe::*;

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
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
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

    pub fn add_data_point(&mut self, value: f32) {
        let mut data = self.imp().data_points.take();
        data.push(value);
        data.remove(0);
        self.imp().data_points.set(data);

        self.queue_render();
    }
}
