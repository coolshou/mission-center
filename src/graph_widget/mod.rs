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

use gtk::{
    gdk, gdk::prelude::*, glib, glib::once_cell::unsync::OnceCell, prelude::*,
    subclass::prelude::*, GLArea, Snapshot,
};

mod skia_plotter_backend;

mod imp {
    use super::*;

    pub struct GraphWidget {
        skia_context: std::cell::Cell<Option<skia_safe::gpu::DirectContext>>,
    }

    impl GraphWidget {
        fn realize(&self) -> Result<(), Box<dyn std::error::Error>> {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            this.make_current();
            let interface = skia_safe::gpu::gl::Interface::new_native();

            self.skia_context
                .set(skia_safe::gpu::DirectContext::new_gl(interface, None));

            Ok(())
        }

        pub fn render(
            &self,
            canvas: &mut skia_safe::canvas::Canvas,
            width: i32,
            height: i32,
        ) -> Result<(), Box<dyn std::error::Error>> {
            use plotters::prelude::*;

            let backend =
                skia_plotter_backend::SkiaPlotterBackend::new(canvas, width as _, height as _);
            let root = plotters::drawing::IntoDrawingArea::into_drawing_area(backend);

            // After this point, we should be able to construct a chart context
            let mut chart = ChartBuilder::on(&root)
                // Set the caption of the chart
                // Set the size of the label region
                .x_label_area_size(0)
                .y_label_area_size(0)
                // Finally attach a coordinate on the drawing area and make a chart context
                .build_cartesian_2d(0_f32..60., 0_f32..100.)?;

            // Then we can draw a mesh
            chart
                .configure_mesh()
                // We can customize the maximum number of labels allowed for each axis
                .x_labels(60)
                .y_labels(100)
                // We can also change the format of the label text
                .draw()?;

            root.present()?;

            Ok(())
        }
    }

    impl Default for GraphWidget {
        fn default() -> Self {
            Self {
                skia_context: std::cell::Cell::new(None),
            }
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
        fn render(&self, context: &gdk::GLContext) -> bool {
            use skia_safe::*;

            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            let mut viewport_info: [gl_rs::types::GLint; 4] = [0; 4];
            unsafe {
                gl_rs::GetIntegerv(gl_rs::VIEWPORT, &mut viewport_info[0]);
            }

            let width = viewport_info[2];
            let height = viewport_info[3];
            unsafe {
                gl_rs::ClearColor(1.0, 0.0, 0.0, 1.0);
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
                    .expect("Skia context not initialized")
            };
            let mut surface = Surface::from_backend_render_target(
                skia_context,
                &skia_render_target,
                gpu::SurfaceOrigin::TopLeft,
                ColorType::RGBA8888,
                None,
                None,
            )
            .expect("Failed to create Skia surface");

            self.render(surface.canvas(), width, height)
                .expect("Failed to render");

            skia_context.flush_and_submit();

            return true;
        }
    }
}

glib::wrapper! {
    pub struct GraphWidget(ObjectSubclass<imp::GraphWidget>)
        @extends gtk::GLArea,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}
