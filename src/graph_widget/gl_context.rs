use std::fmt::Debug;

const EGL_CONFIG_ATTRIBUTES: [egl::EGLint; 19] = [
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
    egl::EGL_SAMPLES,
    2,
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

        pub fn glEGLImageTargetTexture2DOES(target: u32, image: *mut core::ffi::c_void);
    }
}

extern "C" {
    fn gdk_wayland_display_get_egl_display(
        instance: *mut gtk::gdk::ffi::GdkDisplay,
    ) -> *mut core::ffi::c_void;
}

#[derive(Debug, PartialEq, Eq)]
pub struct GLContext {
    pub egl_display: *mut core::ffi::c_void,
    pub egl_context: *mut core::ffi::c_void,
}

#[derive(Debug)]
pub enum GLContextError {
    UnsupportedPlatform,
    EGLContextCreationFailed(i32),
}

impl GLContext {
    pub fn new(native: &gtk::Native) -> Result<Self, GLContextError> {
        use egl::*;
        use gtk::{glib::translate::ToGlibPtr, traits::WidgetExt};

        let display = native.display();
        let egl_display = unsafe { gdk_wayland_display_get_egl_display(display.to_glib_none().0) };

        if egl_display.is_null() {
            return Err(GLContextError::UnsupportedPlatform);
        }

        let egl_config =
            if let Some(egl_config) = choose_config(egl_display, &EGL_CONFIG_ATTRIBUTES, 1) {
                egl_config
            } else {
                return Err(GLContextError::EGLContextCreationFailed(get_error() as i32));
            };

        let egl_context = if let Some(egl_context) = create_context(
            egl_display,
            egl_config,
            EGL_NO_CONTEXT,
            &EGL_CONTEXT_ATTRIBUTES,
        ) {
            egl_context
        } else {
            return Err(GLContextError::EGLContextCreationFailed(get_error() as i32));
        };

        Ok(Self {
            egl_display,
            egl_context,
        })
    }

    pub fn make_current(&self) {
        use egl::*;

        make_current(
            self.egl_display,
            EGL_NO_SURFACE,
            EGL_NO_SURFACE,
            self.egl_context,
        );
    }
}

impl Drop for GLContext {
    fn drop(&mut self) {
        use egl::*;

        unsafe {
            destroy_context(self.egl_display, self.egl_context);
        }
    }
}
