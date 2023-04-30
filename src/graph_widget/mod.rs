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

        pub fn render_graph(
            &self,
            canvas: &mut skia_safe::canvas::Canvas,
            width: i32,
            height: i32,
        ) -> Result<(), Box<dyn std::error::Error>> {
            use plotters::prelude::*;
            use rand::SeedableRng;
            use rand_distr::{Distribution, Normal};
            use rand_xorshift::XorShiftRng;

            let data: Vec<_> = {
                let norm_dist = Normal::new(500.0, 100.0).unwrap();
                let mut x_rand = XorShiftRng::from_seed(*b"MyFragileSeed123");
                let x_iter = norm_dist.sample_iter(&mut x_rand);
                x_iter
                    .filter(|x| *x < 1500.0)
                    .take(100)
                    .zip(0..)
                    .map(|(x, b)| x + (b as f64).powf(1.2))
                    .collect()
            };

            let backend =
                skia_plotter_backend::SkiaPlotterBackend::new(canvas, width as _, height as _);
            let root = plotters::drawing::IntoDrawingArea::into_drawing_area(backend);

            root.fill(&WHITE)?;

            let mut chart = ChartBuilder::on(&root)
                .set_label_area_size(LabelAreaPosition::Left, 60)
                .set_label_area_size(LabelAreaPosition::Bottom, 60)
                .caption("Area Chart Demo", ("sans-serif", 40))
                .build_cartesian_2d(0..(data.len() - 1), 0.0..1500.0)?;

            chart
                .configure_mesh()
                .disable_x_mesh()
                .disable_y_mesh()
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
        fn render(&self, _: &gdk::GLContext) -> bool {
            use skia_safe::*;

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
                    .expect("Skia context not initialized")
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

            self.render_graph(surface.canvas(), width, height)
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
