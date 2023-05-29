#[allow(unused)]
mod ffi {
    const PDEV_LEN: usize = 16;
    const MAX_DEVICE_NAME: usize = 128;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct ListHead {
        pub next: *mut ListHead,
        pub prev: *mut ListHead,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUVendor {
        pub list: ListHead,

        pub init: Option<fn() -> u8>,
        pub shutdown: Option<fn()>,

        pub last_error_string: Option<fn() -> *const i8>,

        pub get_device_handles: Option<fn(devices: *mut ListHead, count: *mut u32) -> u8>,

        pub populate_static_info: Option<fn(gpu_info: *mut GPUInfo) -> u8>,
        pub refresh_dynamic_info: Option<fn(gpu_info: *mut GPUInfo) -> u8>,

        pub refresh_running_processes: Option<fn(gpu_info: *mut GPUInfo) -> u8>,

        pub name: *mut i8,
    }

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

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct GPUInfo {
        pub list: ListHead,
        pub vendor: *mut GPUVendor,
        pub static_info: GPUInfoStaticInfo,
        pub dynamic_info: GPUInfoDynamicInfo,
        pub processes_count: u32,
        // pub processes: *mut GPUProcess,
        // pub processes_array_size: u32,
        // pub pdev: [i8; PDEV_LEN],
    }

    extern "C" {
        pub fn gpuinfo_init_info_extraction(
            monitored_dev_count: *mut u32,
            devices: *mut ListHead,
        ) -> u8;

        pub fn gpuinfo_populate_static_infos(devices: *mut ListHead) -> u8;
        pub fn gpuinfo_refresh_dynamic_info(devices: *mut ListHead) -> u8;
    }
}

pub unsafe fn print_gpus() {
    let mut all_dev_count: u32 = 0;
    let mut monitored_gpus: ffi::ListHead = ffi::ListHead {
        next: std::ptr::null_mut(),
        prev: std::ptr::null_mut(),
    };
    monitored_gpus.next = &mut monitored_gpus;
    monitored_gpus.prev = &mut monitored_gpus;

    ffi::gpuinfo_init_info_extraction(&mut all_dev_count, &mut monitored_gpus);
    ffi::gpuinfo_populate_static_infos(&mut monitored_gpus);
    ffi::gpuinfo_refresh_dynamic_info(&mut monitored_gpus);

    let mut device: *mut ffi::ListHead = monitored_gpus.next;
    while device != (&mut monitored_gpus) {
        let dev: *mut ffi::GPUInfo = core::mem::transmute(device);
        dbg!(&(*dev));

        let device_name = std::ffi::CStr::from_ptr((*dev).static_info.device_name.as_ptr());
        dbg!(device_name);

        device = (*device).next;
    }
}
