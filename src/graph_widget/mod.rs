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
    subclass::prelude::*, Snapshot,
};

mod egl_context;
mod render_target;
mod skia_plotter_backend;

mod imp {
    use super::*;

    pub struct GraphWidget {
        gdk_gl_context: OnceCell<gdk::GLContext>,

        egl_context: OnceCell<egl_context::EGLContext>,
        render_target: OnceCell<render_target::RenderTarget>,

        skia_context: std::cell::Cell<Option<skia_safe::gpu::DirectContext>>,
    }

    impl GraphWidget {
        fn realize(&self) -> Result<(), Box<dyn std::error::Error>> {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            // Obtain a native surface from the GtkWidget
            let native = this
                .native()
                .ok_or("Failed to get native surface".to_owned())?;

            // Create a GDK GL context used as a bridge between GTK and EGL
            let gdk_gl_context = native.surface().create_gl_context()?;
            gdk_gl_context.set_forward_compatible(true);
            gdk_gl_context.realize()?;
            self.gdk_gl_context
                .set(gdk_gl_context)
                .map_err(|_| "GDK GL context initialized twice".to_owned())?;

            // Create an EGL context used to do the actual rendering
            let egl_context = egl_context::EGLContext::new(&native)?;
            self.egl_context
                .set(egl_context)
                .map_err(|_| "EGL context initialized twice".to_owned())?;

            let egl_context = unsafe { self.egl_context.get().unwrap_unchecked() };
            egl_context.make_current()?;

            // Create an offscreen render target
            self.render_target
                .set(render_target::RenderTarget::new()?)
                .map_err(|_| "Render target initialized twice".to_owned())?;

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
                gdk_gl_context: OnceCell::new(),
                egl_context: OnceCell::new(),
                render_target: OnceCell::new(),
                skia_context: std::cell::Cell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GraphWidget {
        const NAME: &'static str = "GraphWidget";
        type Type = super::GraphWidget;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for GraphWidget {}

    impl WidgetImpl for GraphWidget {
        fn realize(&self) {
            self.parent_realize();
            self.realize().expect("Failed to realize widget");
        }

        #[allow(mutable_transmutes)]
        fn snapshot(&self, snapshot: &Snapshot) {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            let width = this.allocated_width();
            let height = this.allocated_height();

            let egl_context = self.egl_context.get().expect("EGL context not initialized");
            let render_target = self
                .render_target
                .get()
                .expect("Render target not initialized");

            egl_context
                .make_current()
                .expect("Failed to make EGL context current");

            render_target
                .resize(width, height)
                .expect("Failed to resize render target");
            let mut image = {
                use skia_safe::*;

                render_target.bind().expect("Failed to bind render target");

                unsafe {
                    gl_rs::ClearColor(0.0, 0.0, 0.0, 0.0);
                    gl_rs::Clear(gl_rs::COLOR_BUFFER_BIT);
                }

                let skia_render_target = gpu::BackendRenderTarget::new_gl(
                    (width, height),
                    0,
                    8,
                    gpu::gl::FramebufferInfo {
                        fboid: self.render_target.get().unwrap().framebuffer_id(),
                        format: gpu::gl::Format::RGBA8.into(),
                    },
                );

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

                self.render_target
                    .get()
                    .unwrap()
                    .unbind(self.egl_context.get().unwrap())
                    .unwrap()
            };

            self.gdk_gl_context.get().unwrap().make_current();

            let egl_image_texture = self
                .render_target
                .get()
                .unwrap()
                .export_as_texture(&mut image)
                .unwrap();

            let gdk_gl_texture = unsafe {
                gdk::GLTexture::with_release_func(
                    self.gdk_gl_context.get().unwrap(),
                    egl_image_texture.id(),
                    width,
                    height,
                    move || {
                        drop(egl_image_texture);
                        drop(image);
                    },
                )
            };
            snapshot.append_texture(
                &gdk_gl_texture,
                &gtk::graphene::Rect::new(0., 0., width as _, height as _),
            );
        }
    }
}

glib::wrapper! {
    pub struct GraphWidget(ObjectSubclass<imp::GraphWidget>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}
