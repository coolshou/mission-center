use std::ffi::c_void;
use std::fmt::{Debug, Display};

use super::egl_context::*;

pub struct Texture {
    texture: u32,
}

impl Texture {
    pub fn id(&self) -> u32 {
        self.texture
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl_rs::DeleteTextures(1, &self.texture);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct RenderTarget {
    framebuffer: u32,
    texture: u32,
    glEGLImageTargetTexture2DOES: unsafe extern "C" fn(target: u32, image: *mut c_void),
}

#[derive(Debug)]
pub enum RenderTargetError {
    GLError(u32),
    EGLContextError(EGLContextError),
    FailedToLoadGLLibrary,
    MissingGlEGLImageTargetTexture2DOES,
    FramebufferIncomplete,
}

impl Display for RenderTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for RenderTargetError {}

impl From<EGLContextError> for RenderTargetError {
    fn from(e: EGLContextError) -> Self {
        Self::EGLContextError(e)
    }
}

impl RenderTarget {
    pub fn new() -> Result<Self, RenderTargetError> {
        let lib = minidl::Library::load("libGL.so.1\0")
            .map_err(|_| RenderTargetError::FailedToLoadGLLibrary)?;
        let fn_ptr: *const c_void = unsafe { lib.sym("glEGLImageTargetTexture2DOES\0").unwrap() };
        if fn_ptr.is_null() {
            return Err(RenderTargetError::MissingGlEGLImageTargetTexture2DOES);
        }

        let mut framebuffer = 0;
        let error = unsafe {
            gl_rs::GenFramebuffers(1, &mut framebuffer);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let mut texture = 0;
        let error = unsafe {
            gl_rs::GenTextures(1, &mut texture);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        Ok(Self {
            framebuffer,
            texture,
            glEGLImageTargetTexture2DOES: unsafe { std::mem::transmute(fn_ptr) },
        })
    }

    pub fn bind(&self) -> Result<(), RenderTargetError> {
        let error = unsafe {
            gl_rs::BindTexture(gl_rs::TEXTURE_2D, self.texture);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::BindFramebuffer(gl_rs::FRAMEBUFFER, self.framebuffer);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::FramebufferTexture2D(
                gl_rs::FRAMEBUFFER,
                gl_rs::COLOR_ATTACHMENT0,
                gl_rs::TEXTURE_2D,
                self.texture,
                0,
            );
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        if unsafe { gl_rs::CheckFramebufferStatus(gl_rs::FRAMEBUFFER) }
            != gl_rs::FRAMEBUFFER_COMPLETE
        {
            return Err(RenderTargetError::FramebufferIncomplete);
        }

        Ok(())
    }

    pub fn unbind(&self, context: &EGLContext) -> Result<EGLImage, RenderTargetError> {
        let image = context.create_image(self)?;

        let error = unsafe {
            gl_rs::BindTexture(gl_rs::TEXTURE_2D, 0);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::BindFramebuffer(gl_rs::FRAMEBUFFER, 0);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        Ok(image)
    }

    pub fn export_as_texture(&self, image: &mut EGLImage) -> Result<Texture, RenderTargetError> {
        let mut texture = 0;

        let error = unsafe {
            gl_rs::GenTextures(1, &mut texture);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::BindTexture(gl_rs::TEXTURE_2D, texture);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::TexParameteri(
                gl_rs::TEXTURE_2D,
                gl_rs::TEXTURE_MIN_FILTER,
                gl_rs::LINEAR as _,
            );
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::TexParameteri(
                gl_rs::TEXTURE_2D,
                gl_rs::TEXTURE_MAG_FILTER,
                gl_rs::LINEAR as _,
            );
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::TexParameteri(
                gl_rs::TEXTURE_2D,
                gl_rs::TEXTURE_WRAP_S,
                gl_rs::CLAMP_TO_EDGE as _,
            );
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::TexParameteri(
                gl_rs::TEXTURE_2D,
                gl_rs::TEXTURE_WRAP_T,
                gl_rs::CLAMP_TO_EDGE as _,
            );
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            (self.glEGLImageTargetTexture2DOES)(gl_rs::TEXTURE_2D, image.image_handle());
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::BindTexture(gl_rs::TEXTURE_2D, 0);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        Ok(Texture { texture })
    }

    pub fn resize(&self, width: i32, height: i32) -> Result<(), RenderTargetError> {
        let error = unsafe {
            gl_rs::BindTexture(gl_rs::TEXTURE_2D, self.texture);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::TexImage2D(
                gl_rs::TEXTURE_2D,
                0,
                gl_rs::RGBA as _,
                width,
                height,
                0,
                gl_rs::RGBA,
                gl_rs::UNSIGNED_BYTE,
                std::ptr::null_mut(),
            );
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        let error = unsafe {
            gl_rs::Viewport(0, 0, width, height);
            gl_rs::GetError()
        };
        if error != gl_rs::NO_ERROR {
            return Err(RenderTargetError::GLError(error));
        }

        Ok(())
    }

    pub fn texture_id(&self) -> u32 {
        self.texture
    }
}

impl Drop for RenderTarget {
    fn drop(&mut self) {
        unsafe {
            gl_rs::DeleteTextures(1, &self.texture);
            gl_rs::DeleteFramebuffers(1, &self.framebuffer);
        }
    }
}
