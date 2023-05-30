use std::fmt::Display;

use raw_sync::Timeout;

use shm::*;
pub use shm::{
    GPUInfo, GPUInfoDynamicInfo, GPUInfoDynamicInfoValid, GPUInfoStaticInfo, GPUInfoStaticInfoValid,
};

mod shm {
    const MAX_DEVICE_NAME: usize = 128;
    const MAX_GPU_COUNT: usize = 8;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub enum GPUInfoStaticInfoValid {
        DeviceNameValid = 0,
        MaxPcieGenValid,
        MaxPcieLinkWidthValid,
        TemperatureShutdownThresholdValid,
        TemperatureSlowdownThresholdValid,
        StaticInfoCount,
    }

    const GPU_INFO_STATIC_INFO_COUNT: usize = GPUInfoStaticInfoValid::StaticInfoCount as usize;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUInfoStaticInfo {
        pub device_name: [i8; MAX_DEVICE_NAME],
        pub max_pcie_gen: u32,
        pub max_pcie_link_width: u32,
        pub temperature_shutdown_threshold: u32,
        pub temperature_slowdown_threshold: u32,
        pub integrated_graphics: u8,
        pub valid: [u8; (GPU_INFO_STATIC_INFO_COUNT + 7) / 8],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub enum GPUInfoDynamicInfoValid {
        GpuClockSpeedValid = 0,
        GpuClockSpeedMaxValid,
        MemClockSpeedValid,
        MemClockSpeedMaxValid,
        GpuUtilRateValid,
        MemUtilRateValid,
        EncoderRateValid,
        DecoderRateValid,
        TotalMemoryValid,
        FreeMemoryValid,
        UsedMemoryValid,
        PcieLinkGenValid,
        PcieLinkWidthValid,
        PcieRxValid,
        PcieTxValid,
        FanSpeedValid,
        GpuTempValid,
        PowerDrawValid,
        PowerDrawMaxValid,
        DynamicInfoCount,
    }

    const GPU_INFO_DYNAMIC_INFO_COUNT: usize = GPUInfoDynamicInfoValid::DynamicInfoCount as usize;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUInfoDynamicInfo {
        // Device clock speed in MHz
        pub gpu_clock_speed: u32,
        // Maximum clock speed in MHz
        pub gpu_clock_speed_max: u32,
        // Memory clock speed in MHz
        pub mem_clock_speed: u32,
        // Maximum clock speed in MHz
        pub mem_clock_speed_max: u32,
        // GPU utilization rate in %
        pub gpu_util_rate: u32,
        // MEM utilization rate in %
        pub mem_util_rate: u32,
        // Encoder utilization rate in %
        pub encoder_rate: u32,
        // Decoder utilization rate in %
        pub decoder_rate: u32,
        // Total memory (bytes)
        pub total_memory: u64,
        // Unallocated memory (bytes)
        pub free_memory: u64,
        // Allocated memory (bytes)
        pub used_memory: u64,
        // PCIe link generation used
        pub pcie_link_gen: u32,
        // PCIe line width used
        pub pcie_link_width: u32,
        // PCIe throughput in KB/s
        pub pcie_rx: u32,
        // PCIe throughput in KB/s
        pub pcie_tx: u32,
        // Fan speed percentage
        pub fan_speed: u32,
        // GPU temperature °celsius
        pub gpu_temp: u32,
        // Power usage in milliwatts
        pub power_draw: u32,
        // Max power usage in milliwatts
        pub power_draw_max: u32,
        // True if encode and decode is shared (Intel)
        pub encode_decode_shared: u8,
        pub valid: [u8; (GPU_INFO_DYNAMIC_INFO_COUNT + 7) / 8],
    }

    pub struct SharedMemoryHeader {
        pub is_initialized: std::sync::atomic::AtomicU8,
        pub rw_lock: raw_sync::locks::RwLock,
        _padding: [u8; 64],
    }

    #[derive(Debug, Copy, Clone)]
    pub struct GPUInfo {
        pub static_info: GPUInfoStaticInfo,
        pub dynamic_info: GPUInfoDynamicInfo,
    }

    pub struct SharedMemoryData {
        pub header: SharedMemoryHeader,
        pub gpu_count: usize,
        pub gpu_info: [GPUInfo; MAX_GPU_COUNT],
    }
}

pub struct SharedMemoryReadGuard<'a> {
    _lock: raw_sync::locks::ReadLockGuard<'a>,
    pub data: &'a [GPUInfo],
}

pub struct SharedMemoryWriteGuard<'a> {
    _lock: raw_sync::locks::LockGuard<'a>,
    pub data: (&'a mut usize, &'a mut [GPUInfo]),
}

pub struct SharedMemory {
    _shm_handle: shared_memory::Shmem,
    lock: Box<dyn raw_sync::locks::LockImpl>,
    data: *mut SharedMemoryData,
}

impl SharedMemory {
    pub fn new<P: AsRef<std::path::Path> + Display>(file_link: P, force_new: bool) -> Option<Self> {
        use raw_sync::locks::*;
        use shared_memory::*;
        use std::sync::atomic::*;

        assert_eq!(core::mem::align_of::<SharedMemoryHeader>(), 8);

        let shm_handle = match ShmemConf::new()
            .size(core::mem::size_of::<SharedMemoryData>())
            .flink(&file_link)
            .create()
        {
            Ok(m) => m,
            Err(ShmemError::LinkExists) => {
                if force_new {
                    std::fs::remove_file(&file_link)
                        .expect("Unable to remove shared memory file link");
                    match ShmemConf::new()
                        .size(core::mem::size_of::<SharedMemoryData>())
                        .flink(&file_link)
                        .create()
                    {
                        Ok(shm) => shm,
                        Err(err) => {
                            eprintln!(
                                "Unable to create shared memory file link {} : {}",
                                file_link, err
                            );
                            return None;
                        }
                    }
                } else {
                    ShmemConf::new().flink(&file_link).open().unwrap()
                }
            }
            Err(e) => {
                eprintln!(
                    "Unable to create or open shared memory file link {} : {}",
                    file_link, e
                );
                return None;
            }
        };

        let data = shm_handle.as_ptr() as *mut SharedMemoryData;
        let shm_data = unsafe { &mut *data };

        let rw_lock = if shm_handle.is_owner() {
            shm_data.header.is_initialized.store(0, Ordering::Relaxed);
            let (lock, bytes_used) = unsafe {
                RwLock::new(
                    (&mut shm_data.header.rw_lock) as *mut _ as *mut u8, // Base address of the lock
                    (&mut shm_data.gpu_info) as *mut _ as *mut u8, // Address of data protected by lock
                )
                .expect("Unable to create lock")
            };
            assert!(
                bytes_used
                    < (core::mem::size_of::<SharedMemoryHeader>()
                        - core::mem::size_of::<AtomicU8>())
            );

            {
                let l = lock
                    .try_lock(Timeout::Infinite)
                    .expect("Unable to lock shared memory");
                shm_data.gpu_count = 0;
                drop(l);
            }
            shm_data.header.is_initialized.store(1, Ordering::Relaxed);

            lock
        } else {
            while shm_data.header.is_initialized.load(Ordering::Relaxed) != 1 {}
            let (lock, bytes_used) = unsafe {
                RwLock::from_existing(
                    (&mut shm_data.header.rw_lock) as *mut _ as *mut u8, // Base address of the lock
                    (&mut shm_data.gpu_info) as *mut _ as *mut u8, // Address of data protected by lock
                )
                .expect("Unable to obtain existing lock")
            };
            assert!(
                bytes_used
                    < (core::mem::size_of::<SharedMemoryHeader>()
                        - core::mem::size_of::<AtomicU8>())
            );
            lock
        };

        Some(Self {
            _shm_handle: shm_handle,
            lock: rw_lock,
            data,
        })
    }

    pub fn read(&self) -> Option<SharedMemoryReadGuard> {
        let lock = self.lock.try_rlock(raw_sync::Timeout::Infinite);
        if lock.is_err() {
            eprintln!("Unable to obtain read lock: {}", lock.err().unwrap());
            return None;
        }

        let data = unsafe { &*self.data };
        let gpu_info =
            unsafe { core::slice::from_raw_parts(data.gpu_info.as_ptr(), data.gpu_count) };

        Some(SharedMemoryReadGuard {
            _lock: lock.unwrap(),
            data: gpu_info,
        })
    }

    pub fn write(&self) -> Option<SharedMemoryWriteGuard> {
        let lock = self.lock.try_lock(raw_sync::Timeout::Infinite);
        if lock.is_err() {
            eprintln!("Unable to obtain write lock: {}", lock.err().unwrap());
            return None;
        }

        let data = unsafe { &mut *self.data };

        Some(SharedMemoryWriteGuard {
            _lock: lock.unwrap(),
            data: (&mut data.gpu_count, data.gpu_info.as_mut_slice()),
        })
    }
}
