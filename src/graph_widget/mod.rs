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

mod eglext {
    extern "C" {
        pub fn glEGLImageTargetTexture2DOES(target: u32, image: *mut core::ffi::c_void);
    }
}

mod imp {
    use super::*;

    pub struct GraphWidget {
        gdk_gl_context: OnceCell<gdk::GLContext>,

        egl_context: OnceCell<egl_context::EGLContext>,
        render_target: OnceCell<render_target::RenderTarget>,
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

            Ok(())
        }
    }

    impl Default for GraphWidget {
        fn default() -> Self {
            Self {
                gdk_gl_context: OnceCell::new(),
                egl_context: OnceCell::new(),
                render_target: OnceCell::new(),
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

        fn snapshot(&self, snapshot: &Snapshot) {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            let width = this.allocated_width();
            let height = this.allocated_height();

            self.egl_context.get().unwrap().make_current().unwrap();
            self.render_target
                .get()
                .unwrap()
                .resize(width, height)
                .unwrap();

            let mut image = {
                self.render_target.get().unwrap().bind().unwrap();

                unsafe {
                    use gl_rs::*;

                    ClearColor(0.0, 1.0, 1.0, 1.0);
                    dbg!(GetError());
                    Clear(gl_rs::COLOR_BUFFER_BIT);
                    dbg!(GetError());
                }

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
