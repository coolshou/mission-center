#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DiskType {
    Unknown,
    HDD,
    SSD,
    NVMe,
    iSCSI,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Disk {
    pub name: String,
    pub r#type: DiskType,
    pub capacity: u64,
    pub formatted: u64,
    pub system_disk: bool,

    pub busy_percent: f32,
    pub response_time_ms: u64,
    pub read_speed: u64,
    pub write_speed: u64,
}

pub struct DiskInfo {
    disks: Vec<Disk>,
}

impl DiskInfo {
    pub fn new() -> Self {
        Self { disks: vec![] }
    }

    pub fn refresh(&mut self) {
        self.disks.clear();
        self.disks.push(Disk {
            name: "/dev/disk0".to_string(),
            r#type: DiskType::SSD,
            capacity: 100_000_000_000,
            formatted: 100_000_000_000,
            system_disk: true,
            busy_percent: 40.0,
            response_time_ms: 10,
            read_speed: 124_200_934,
            write_speed: 28_234_989,
        });
    }

    pub fn disks(&self) -> &[Disk] {
        &self.disks
    }
}
