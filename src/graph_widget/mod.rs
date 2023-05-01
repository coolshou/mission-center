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

use gtk::{gdk, gdk::prelude::*, glib, prelude::*, subclass::prelude::*};

mod skia_plotter_backend;

mod imp {
    use super::*;

    pub struct GraphWidget {
        skia_context: std::cell::Cell<Option<skia_safe::gpu::DirectContext>>,

        pub data: std::cell::Cell<Vec<f32>>,
    }

    impl Default for GraphWidget {
        fn default() -> Self {
            Self {
                skia_context: std::cell::Cell::new(None),
                data: std::cell::Cell::new(vec![0.0; 100]),
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

            let backend = skia_plotter_backend::SkiaPlotterBackend::new(
                canvas,
                width as _,
                height as _,
                scale_factor,
            );
            let root = plotters::drawing::IntoDrawingArea::into_drawing_area(backend);

            let data = unsafe { &*(self.data.as_ptr()) };
            let mut chart = ChartBuilder::on(&root)
                .set_all_label_area_size(0)
                .build_cartesian_2d(0..data.len(), 0_f32..100_f32)?;

            chart
                .configure_mesh()
                .max_light_lines(1)
                .x_labels((0.02 * width as f32).floor() as _)
                .y_labels((0.02 * height as f32).floor() as _)
                .draw()?;

            chart.draw_series(
                AreaSeries::new(
                    (0..).zip(data.iter()).map(|(x, y)| (x, *y)),
                    0.0,
                    &RED.mix(0.2),
                )
                .border_style(&RED),
            )?;

            root.present()?;

            Ok(())
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GraphWidget {
        const NAME: &'static str = "GraphWidget";
        type Type = super::GraphWidget;
        type ParentType = gtk::GLArea;
    }

    impl ObjectImpl for GraphWidget {}

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
        @extends gtk::Widget, gtk::GLArea,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl GraphWidget {
    pub fn add_data_point(&mut self, value: f32) {
        let data = unsafe { &mut *(self.imp().data.as_ptr()) };
        data.push(value);
        data.remove(0);

        self.queue_render();
    }
}
