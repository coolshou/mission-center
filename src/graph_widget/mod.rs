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

use gtk::{
    gdk,
    gdk::prelude::*,
    glib,
    glib::{once_cell::unsync::OnceCell, translate::ToGlibPtr},
    prelude::*,
    subclass::prelude::*,
    Snapshot,
};

mod gl_context;
mod render_target;

mod eglext {
    pub const EGL_GL_TEXTURE_2D: egl::EGLint = 0x30B1;

    extern "C" {
        pub fn eglCreateImage(
            dpy: *mut core::ffi::c_void,
            ctx: *mut core::ffi::c_void,
            target: u32,
            buffer: *mut core::ffi::c_void,
            attrib_list: *const i32,
        ) -> *mut core::ffi::c_void;

        pub fn eglDestroyImage(
            dpy: *mut core::ffi::c_void,
            image: *mut core::ffi::c_void,
        ) -> egl::EGLBoolean;

        pub fn glEGLImageTargetTexture2DOES(target: u32, image: *mut core::ffi::c_void);
    }
}

mod imp {
    use super::*;

    pub struct GraphWidget {
        gdk_gl_context: OnceCell<gdk::GLContext>,

        gl_context: OnceCell<gl_context::GLContext>,
        framebuffer: Cell<gl_rs::types::GLuint>,
    }

    impl GraphWidget {
        fn create_gdk_gl_context(&self) -> Result<gdk::GLContext, glib::Error> {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            let surface = if let Some(native) = this.native() {
                Ok(native.surface())
            } else {
                Err(glib::Error::new(
                    gdk::GLError::NotAvailable,
                    "Failed to get native surface",
                ))
            }?;

            let gdk_gl_context = surface.create_gl_context()?;
            gdk_gl_context.set_forward_compatible(true);
            gdk_gl_context.realize()?;

            Ok(gdk_gl_context)
        }

        fn create_offscreen_framebuffer(
            &self,
        ) -> Result<gl_rs::types::GLuint, gl_rs::types::GLenum> {
            self.gl_context.get().unwrap().make_current();

            Ok(unsafe {
                let mut framebuffer = 0;
                gl_rs::GenFramebuffers(1, &mut framebuffer);
                let error = gl_rs::GetError();
                if error != gl_rs::NO_ERROR {
                    return Err(error);
                }

                framebuffer
            })
        }

        fn create_offscreen_texture(
            &self,
            width: i32,
            height: i32,
        ) -> Result<gl_rs::types::GLuint, String> {
            self.gl_context.get().unwrap().make_current();

            let mut tex = 0;
            unsafe {
                use gl_rs::*;

                GenTextures(1, &mut tex);
                let error = GetError();
                if error != NO_ERROR {
                    return Err(format!("GenTextures: {}", error));
                }

                BindTexture(TEXTURE_2D, tex);
                let error = GetError();
                if error != NO_ERROR {
                    return Err(format!("BindTexture: {}", error));
                }

                TexImage2D(
                    TEXTURE_2D,
                    0,
                    RGBA as _,
                    width,
                    height,
                    0,
                    RGBA,
                    UNSIGNED_BYTE,
                    std::ptr::null_mut(),
                );
                let error = GetError();
                if error != NO_ERROR {
                    return Err(format!("TexImage2D: {}", error));
                }
            }

            Ok(tex)
        }
    }

    impl Default for GraphWidget {
        fn default() -> Self {
            Self {
                gdk_gl_context: OnceCell::new(),
                gl_context: OnceCell::new(),
                framebuffer: Cell::new(u32::MAX),
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

            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            let gdk_gl_context = self
                .create_gdk_gl_context()
                .expect("Failed to create GL context");
            self.gdk_gl_context
                .set(gdk_gl_context)
                .expect("Failed to set GL context");

            let native = this.native().expect("Failed to get native surface");
            let gl_context =
                gl_context::GLContext::new(&native).expect("Failed to create GL context");
            self.gl_context
                .set(gl_context)
                .expect("Failed to store GL context");

            self.gl_context.get().unwrap().make_current();
            let framebuffer = self
                .create_offscreen_framebuffer()
                .expect("Failed to create framebuffer");
            self.framebuffer.set(framebuffer);
        }

        fn snapshot(&self, snapshot: &Snapshot) {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            let width = this.allocated_width();
            let height = this.allocated_height();

            unsafe {
                use gl_rs::*;

                self.gl_context.get().unwrap().make_current();

                let texture = self
                    .create_offscreen_texture(this.allocated_width(), this.allocated_height())
                    .expect("Failed to create offscreen texture");

                BindTexture(TEXTURE_2D, texture);
                BindFramebuffer(FRAMEBUFFER, texture);
                dbg!(GetError());
                FramebufferTexture2D(FRAMEBUFFER, COLOR_ATTACHMENT0, TEXTURE_2D, texture, 0);
                dbg!(GetError());
                if CheckFramebufferStatus(FRAMEBUFFER) != FRAMEBUFFER_COMPLETE {
                    panic!("Framebuffer is not complete!");
                }

                gl_rs::Viewport(0, 0, width, height);
                dbg!(GetError());
                gl_rs::ClearColor(0.0, 1.0, 1.0, 1.0);
                dbg!(GetError());
                gl_rs::Clear(gl_rs::COLOR_BUFFER_BIT);
                dbg!(GetError());

                let image = eglext::eglCreateImage(
                    self.gl_context.get().unwrap().egl_display,
                    self.gl_context.get().unwrap().egl_context,
                    eglext::EGL_GL_TEXTURE_2D as _,
                    core::mem::transmute(texture as types::GLintptr),
                    std::ptr::null(),
                );
                dbg!(egl::get_error());

                BindTexture(TEXTURE_2D, 0);

                egl::bind_api(egl::EGL_OPENGL_API);
                dbg!(egl::get_error());
                self.gdk_gl_context.get().unwrap().make_current();

                let mut tex = 0;
                GenTextures(1, &mut tex);
                dbg!(GetError());
                BindTexture(TEXTURE_2D, tex);
                dbg!(GetError());
                TexParameteri(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as _);
                dbg!(GetError());
                TexParameteri(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as _);
                dbg!(GetError());
                TexParameteri(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as _);
                dbg!(GetError());
                TexParameteri(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as _);
                dbg!(GetError());
                eglext::glEGLImageTargetTexture2DOES(TEXTURE_2D, image);
                dbg!(GetError());
                BindTexture(TEXTURE_2D, 0);
                dbg!(GetError());
                let texture = gtk::gdk::GLTexture::new(
                    self.gdk_gl_context.get().unwrap(),
                    tex,
                    width,
                    height,
                );

                snapshot.append_texture(
                    &texture,
                    &gtk::graphene::Rect::new(0., 0., width as _, height as _),
                );

                eglext::eglDestroyImage(self.gl_context.get().unwrap().egl_display, image);
                dbg!(egl::get_error());
            }
        }
    }
}

glib::wrapper! {
    pub struct GraphWidget(ObjectSubclass<imp::GraphWidget>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}
