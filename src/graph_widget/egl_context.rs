use std::fmt::{Debug, Display};

use egl::EGLDisplay;

use crate::graph_widget::render_target::RenderTarget;

const EGL_CONFIG_ATTRIBUTES: [egl::EGLint; 17] = [
    egl::EGL_RED_SIZE,
    8,
    egl::EGL_GREEN_SIZE,
    8,
    egl::EGL_BLUE_SIZE,
    8,
    egl::EGL_ALPHA_SIZE,
    8,
    egl::EGL_STENCIL_SIZE,
    8,
    egl::EGL_DEPTH_SIZE,
    0,
    egl::EGL_RENDERABLE_TYPE,
    egl::EGL_OPENGL_BIT,
    egl::EGL_CONFORMANT,
    egl::EGL_OPENGL_BIT,
    egl::EGL_NONE,
];

const EGL_CONTEXT_ATTRIBUTES: [egl::EGLint; 5] = [
    ext::EGL_CONTEXT_MAJOR_VERSION_KHR,
    3,
    ext::EGL_CONTEXT_MINOR_VERSION_KHR,
    2,
    egl::EGL_NONE,
];

mod ext {
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
    }
}

extern "C" {
    fn gdk_wayland_display_get_egl_display(
        instance: *mut gtk::gdk::ffi::GdkDisplay,
    ) -> *mut core::ffi::c_void;
}

pub struct EGLImage {
    egl_display: EGLDisplay,
    egl_image: *mut core::ffi::c_void,
}

impl EGLImage {
    pub fn image_handle(&mut self) -> &mut core::ffi::c_void {
        unsafe { &mut *self.egl_image }
    }
}

impl Drop for EGLImage {
    fn drop(&mut self) {
        unsafe {
            ext::eglDestroyImage(self.egl_display, self.egl_image);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EGLContext {
    pub egl_display: *mut core::ffi::c_void,
    pub egl_context: *mut core::ffi::c_void,
}

#[derive(Debug)]
pub enum EGLContextError {
    UnsupportedPlatform,
    ContextCreationFailed(i32),
    MakeCurrentFailed(i32),
    ImageCreationFailed(i32),
}

impl Display for EGLContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for EGLContextError {}

impl EGLContext {
    pub fn new(native: &gtk::Native) -> Result<Self, EGLContextError> {
        use egl::*;
        use gtk::{glib::translate::ToGlibPtr, traits::WidgetExt};

        bind_api(egl::EGL_OPENGL_ES_API);
        let error = egl::get_error();
        if error != egl::EGL_SUCCESS {
            return Err(EGLContextError::ContextCreationFailed(error));
        }

        let display = native.display();
        let egl_display = unsafe { gdk_wayland_display_get_egl_display(display.to_glib_none().0) };

        if egl_display.is_null() {
            return Err(EGLContextError::UnsupportedPlatform);
        }

        let egl_config =
            if let Some(egl_config) = choose_config(egl_display, &EGL_CONFIG_ATTRIBUTES, 1) {
                egl_config
            } else {
                return Err(EGLContextError::ContextCreationFailed(get_error() as i32));
            };

        let egl_context = if let Some(egl_context) = create_context(
            egl_display,
            egl_config,
            EGL_NO_CONTEXT,
            &EGL_CONTEXT_ATTRIBUTES,
        ) {
            egl_context
        } else {
            return Err(EGLContextError::ContextCreationFailed(get_error() as i32));
        };

        Ok(Self {
            egl_display,
            egl_context,
        })
    }

    pub fn make_current(&self) -> Result<(), EGLContextError> {
        use egl::*;

        bind_api(egl::EGL_OPENGL_ES_API);
        let error = get_error();
        if error != egl::EGL_SUCCESS {
            return Err(EGLContextError::MakeCurrentFailed(error));
        }

        let result = make_current(
            self.egl_display,
            EGL_NO_SURFACE,
            EGL_NO_SURFACE,
            self.egl_context,
        );

        if !result {
            Err(EGLContextError::MakeCurrentFailed(get_error() as i32))
        } else {
            Ok(())
        }
    }

    pub fn create_image(&self, render_target: &RenderTarget) -> Result<EGLImage, EGLContextError> {
        let image = unsafe {
            ext::eglCreateImage(
                self.egl_display,
                self.egl_context,
                ext::EGL_GL_TEXTURE_2D as _,
                core::mem::transmute(render_target.texture_id() as gl_rs::types::GLintptr),
                std::ptr::null(),
            )
        };
        if image.is_null() {
            return Err(EGLContextError::ImageCreationFailed(egl::get_error()));
        }

        Ok(EGLImage {
            egl_display: self.egl_display,
            egl_image: image,
        })
    }
}

impl Drop for EGLContext {
    fn drop(&mut self) {
        egl::destroy_context(self.egl_display, self.egl_context);
    }
}
