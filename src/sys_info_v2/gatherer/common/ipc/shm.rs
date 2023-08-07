use std::ops::{Deref, DerefMut};

struct SharedMemoryHeader {
    pub is_initialized: std::sync::atomic::AtomicU8,
}

struct SharedMemoryData<T: Sized> {
    header: SharedMemoryHeader,
    content: T,
}

pub struct SharedMemoryContent<'a, T: Sized> {
    data: &'a mut T,
}

impl<T: Sized> Deref for SharedMemoryContent<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T: Sized> DerefMut for SharedMemoryContent<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SharedMemoryError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ShmemError(#[from] shared_memory::ShmemError),
}

pub struct SharedMemory<T: Sized> {
    _shm_handle: shared_memory::Shmem,
    data: *mut SharedMemoryData<T>,
}

impl<T: Sized> SharedMemory<T> {
    pub fn new<P: AsRef<std::path::Path>>(
        file_link: P,
        replace_existing: bool,
    ) -> Result<Self, SharedMemoryError> {
        use shared_memory::*;
        use std::sync::atomic::*;

        let shm_handle = match ShmemConf::new()
            .size(core::mem::size_of::<SharedMemoryData<T>>())
            .flink(&file_link)
            .create()
        {
            Ok(m) => m,
            Err(ShmemError::LinkExists) => {
                if replace_existing {
                    if let Ok(dev_shm_file) = std::fs::read(&file_link) {
                        let dev_shm_file_path = format!("/dev/shm{}", unsafe {
                            std::str::from_utf8_unchecked(&dev_shm_file)
                        });
                        let _ = std::fs::remove_file(&dev_shm_file_path);
                    }

                    std::fs::remove_file(&file_link)?;
                    ShmemConf::new()
                        .size(core::mem::size_of::<SharedMemoryData<T>>())
                        .flink(&file_link)
                        .create()?
                } else {
                    ShmemConf::new().flink(&file_link).open()?
                }
            }
            Err(e) => return Err(e.into()),
        };

        let shm_data = unsafe { &mut *(shm_handle.as_ptr() as *mut SharedMemoryData<T>) };

        if shm_handle.is_owner() {
            shm_data.header.is_initialized.store(0, Ordering::Relaxed);
            shm_data.header.is_initialized.store(1, Ordering::Relaxed);
        } else {
            while shm_data.header.is_initialized.load(Ordering::Relaxed) != 1 {}
        }

        Ok(Self {
            _shm_handle: shm_handle,
            data: shm_data,
        })
    }

    pub unsafe fn acquire(&mut self) -> SharedMemoryContent<T> {
        let data = &mut *self.data;
        SharedMemoryContent {
            data: &mut data.content,
        }
    }
}
