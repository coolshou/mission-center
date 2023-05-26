use std::fs;

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
        use gtk::glib::*;

        let entries = std::fs::read_dir("/sys/block");
        if entries.is_err() {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to refresh disk information, failed to read disk entries: {:?}",
                entries.err()
            );
            return;
        }

        for entry in entries.unwrap() {
            if entry.is_err() {
                g_warning!(
                    "MissionCenter::SysInfo",
                    "Failed to read disk entry: {:?}",
                    entry.err()
                );
                continue;
            }

            let entry = entry.unwrap();
            let file_type = entry.file_type();
            if file_type.is_err() {
                g_warning!(
                    "MissionCenter::SysInfo",
                    "Failed to read disk entry file type: {:?}",
                    file_type.err()
                );
                continue;
            }

            let file_type = file_type.unwrap();

            let dir_name = if file_type.is_symlink() {
                let path = entry.path().read_link();
                if path.is_err() {
                    g_warning!(
                        "MissionCenter::SysInfo",
                        "Failed to read disk entry symlink: {:?}",
                        path.err()
                    );
                    continue;
                }

                let path = std::path::Path::new("/sys/block").join(path.unwrap());
                if !path.is_dir() {
                    continue;
                }

                let dir_name = path.file_name();
                if dir_name.is_none() {
                    continue;
                }

                dir_name.unwrap().to_string_lossy().into_owned()
            } else if file_type.is_dir() {
                entry.file_name().to_string_lossy().into_owned()
            } else {
                continue;
            };

            if dir_name.starts_with("loop")
                || dir_name.starts_with("ram")
                || dir_name.starts_with("zram")
                || dir_name.starts_with("sr")
                || dir_name.starts_with("fd")
                || dir_name.starts_with("md")
                || dir_name.starts_with("dm")
                || dir_name.starts_with("zd")
            {
                continue;
            }

            let r#type = if let Ok(v) =
                fs::read_to_string(format!("/sys/block/{}/queue/rotational", dir_name))
            {
                let v = v.trim().parse::<u8>().ok().map_or(u8::MAX, |v| v);
                if v == 0 {
                    if dir_name.starts_with("nvme") {
                        DiskType::NVMe
                    } else {
                        DiskType::SSD
                    }
                } else {
                    match v {
                        1 => DiskType::HDD,
                        _ => DiskType::Unknown,
                    }
                }
            } else {
                DiskType::Unknown
            };

            let mut found = false;
            for i in 0..self.disks.len() {
                if self.disks[i].name == dir_name {
                    found = true;
                    break;
                }
            }

            if !found {
                self.disks.push(Disk {
                    name: dir_name,
                    r#type,
                    capacity: 0,
                    formatted: 0,
                    system_disk: false,
                    busy_percent: 0.0,
                    response_time_ms: 0,
                    read_speed: 0,
                    write_speed: 0,
                });
            }
        }
    }

    pub fn disks(&self) -> &[Disk] {
        &self.disks
    }
}
