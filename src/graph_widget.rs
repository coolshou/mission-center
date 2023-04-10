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

use gtk::{
    gdk::{prelude::*, GLContext, GLTexture},
    glib,
    glib::{once_cell::unsync::OnceCell, translate::ToGlibPtr},
    prelude::*,
    subclass::prelude::*,
    Snapshot,
};

extern "C" {
    fn gdk_wayland_display_get_egl_display(
        instance: *mut gtk::gdk::ffi::GdkDisplay,
    ) -> *mut core::ffi::c_void;
}

mod eglext {
    pub const EGL_GL_TEXTURE_2D: egl::EGLint = 0x30B1;
    pub const EGL_CONTEXT_MAJOR_VERSION_KHR: egl::EGLint = 0x3098;
    pub const EGL_CONTEXT_MINOR_VERSION_KHR: egl::EGLint = 0x30FB;

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
        egl_display: Cell<*mut core::ffi::c_void>,
        egl_context: Cell<*mut core::ffi::c_void>,
        gdk_gl_context: OnceCell<GLContext>,
        framebuffer: Cell<gl_rs::types::GLuint>,
    }

    impl Default for GraphWidget {
        fn default() -> Self {
            Self {
                egl_display: Cell::new(std::ptr::null_mut()),
                egl_context: Cell::new(std::ptr::null_mut()),
                gdk_gl_context: OnceCell::new(),
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
            use egl::*;

            const EGL_CONFIG_ATTRIBUTES: [EGLint; 19] = [
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                8,
                EGL_STENCIL_SIZE,
                8,
                EGL_DEPTH_SIZE,
                0,
                EGL_SAMPLES,
                2,
                EGL_RENDERABLE_TYPE,
                EGL_OPENGL_BIT,
                EGL_CONFORMANT,
                EGL_OPENGL_BIT,
                EGL_NONE,
            ];

            const EGL_CONTEXT_ATTRIBUTES: [EGLint; 5] = [
                eglext::EGL_CONTEXT_MAJOR_VERSION_KHR,
                3,
                eglext::EGL_CONTEXT_MINOR_VERSION_KHR,
                2,
                EGL_NONE,
            ];

            self.parent_realize();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            let surface = this.native().unwrap().surface(); //gtk_native_get_surface (GTK_NATIVE (gtk_widget_get_root (widget)));
            let gdk_gl_context = surface.create_gl_context().unwrap();
            gdk_gl_context.set_forward_compatible(true);
            gdk_gl_context.realize().unwrap();
            self.gdk_gl_context.set(gdk_gl_context).unwrap();

            let display = this.display();
            unsafe {
                self.egl_display.set(gdk_wayland_display_get_egl_display(
                    display.to_glib_none().0,
                ));
                let egl_config =
                    choose_config(self.egl_display.get(), &EGL_CONFIG_ATTRIBUTES, 1).unwrap();
                self.egl_context.set(
                    create_context(
                        self.egl_display.get(),
                        egl_config,
                        EGL_NO_CONTEXT,
                        &EGL_CONTEXT_ATTRIBUTES,
                    )
                    .unwrap(),
                );

                dbg!(egl::make_current(
                    self.egl_display.get(),
                    egl::EGL_NO_SURFACE,
                    egl::EGL_NO_SURFACE,
                    self.egl_context.get(),
                ));

                unsafe {
                    let mut framebuffer = 0;
                    gl_rs::GenFramebuffers(1, &mut framebuffer);
                    if gl_rs::GetError() != gl_rs::NO_ERROR {
                        panic!("Failed to create framebuffer");
                    }

                    self.framebuffer.set(framebuffer);
                }

                dbg!(self.egl_context.get());
            }
        }

        fn snapshot(&self, snapshot: &Snapshot) {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::GraphWidget>();

            dbg!(egl::make_current(
                self.egl_display.get(),
                egl::EGL_NO_SURFACE,
                egl::EGL_NO_SURFACE,
                self.egl_context.get(),
            ));

            dbg!(egl::get_error());

            let width = this.allocated_width();
            let height = this.allocated_height();

            unsafe {
                use gl_rs::*;

                let mut tex = 0;
                GenTextures(1, &mut tex);
                dbg!(GetError());
                BindTexture(TEXTURE_2D, tex);
                dbg!(GetError());
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
                dbg!(GetError());
                BindFramebuffer(FRAMEBUFFER, self.framebuffer.get());
                dbg!(GetError());
                FramebufferTexture2D(FRAMEBUFFER, COLOR_ATTACHMENT0, TEXTURE_2D, tex, 0);
                dbg!(GetError());
                if CheckFramebufferStatus(FRAMEBUFFER) != FRAMEBUFFER_COMPLETE {
                    panic!("Framebuffer is not complete!");
                }

                gl_rs::Viewport(0, 0, width, height);
                dbg!(GetError());
                gl_rs::ClearColor(1.0, 0.0, 0.0, 1.0);
                dbg!(GetError());
                gl_rs::Clear(gl_rs::COLOR_BUFFER_BIT);
                dbg!(GetError());

                let image = eglext::eglCreateImage(
                    self.egl_display.get(),
                    self.egl_context.get(),
                    eglext::EGL_GL_TEXTURE_2D as _,
                    core::mem::transmute(tex as types::GLintptr),
                    std::ptr::null(),
                );
                dbg!(egl::get_error());

                BindTexture(TEXTURE_2D, 0);
                DeleteTextures(1, &tex);

                egl::bind_api(egl::EGL_OPENGL_API);
                dbg!(egl::get_error());
                self.gdk_gl_context.get().unwrap().make_current();

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
