use std::ops::{Deref, DerefMut};

struct SharedMemoryHeader {
    pub is_initialized: std::sync::atomic::AtomicU8,
    pub rw_lock: raw_sync::locks::Mutex,
    _reserved: [u8; 128],
}

struct SharedMemoryData<T: Sized> {
    header: SharedMemoryHeader,
    sentinel: [u8; 0],
    content: T,
}

pub struct SharedMemoryGuard<'a, T: Sized> {
    _lock: raw_sync::locks::LockGuard<'a>,
    data: &'a mut T,
}

impl<T: Sized> Deref for SharedMemoryGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T: Sized> DerefMut for SharedMemoryGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

pub struct SharedMemory<T: Sized> {
    _shm_handle: shared_memory::Shmem,
    lock: Box<dyn raw_sync::locks::LockImpl>,
    data: *mut SharedMemoryData<T>,
}

impl<T: Sized> SharedMemory<T> {
    pub fn new<P: AsRef<std::path::Path>>(
        file_link: P,
        replace_existing: bool,
    ) -> anyhow::Result<Self> {
        use raw_sync::locks::*;
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

        let lock = if shm_handle.is_owner() {
            shm_data.header.is_initialized.store(0, Ordering::Relaxed);

            let (lock, bytes_used) = unsafe {
                Mutex::new(
                    (&mut shm_data.header.rw_lock) as *mut _ as *mut u8,
                    (&mut shm_data.sentinel) as *mut _ as *mut u8,
                )
                .expect("Failed to create mutex")
            };
            assert!(bytes_used < (core::mem::size_of::<SharedMemoryHeader>() - 8));
            shm_data.header.is_initialized.store(1, Ordering::Relaxed);

            lock
        } else {
            while shm_data.header.is_initialized.load(Ordering::Relaxed) != 1 {}
            let (lock, bytes_used) = unsafe {
                Mutex::from_existing(
                    (&mut shm_data.header.rw_lock) as *mut _ as *mut u8,
                    (&mut shm_data.sentinel) as *mut _ as *mut u8,
                )
                .expect("Failed to reuse existing mutex")
            };
            assert!(bytes_used < (core::mem::size_of::<SharedMemoryHeader>() - 8));

            lock
        };

        Ok(Self {
            _shm_handle: shm_handle,
            lock,
            data: shm_data,
        })
    }

    pub fn lock(&mut self, timeout: raw_sync::Timeout) -> Option<SharedMemoryGuard<T>> {
        let lock = self.lock.try_lock(timeout);
        if lock.is_err() {
            return None;
        }
        let data = unsafe { &mut *self.data };
        Some(SharedMemoryGuard {
            _lock: unsafe { lock.unwrap_unchecked() },
            data: &mut data.content,
        })
    }
}
