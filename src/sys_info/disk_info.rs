/* sys_info/disk_info.rs
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

use serde::Deserialize;

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

            let mut found = false;
            for i in 0..self.disks.len() {
                if self.disks[i].name == dir_name {
                    found = true;
                    break;
                }
            }

            if found {
                continue;
            }

            let r#type = if let Ok(v) =
                std::fs::read_to_string(format!("/sys/block/{}/queue/rotational", dir_name))
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

            let capacity =
                if let Ok(v) = std::fs::read_to_string(format!("/sys/block/{}/size", dir_name)) {
                    v.trim().parse::<u64>().ok().map_or(u64::MAX, |v| v * 512)
                } else {
                    u64::MAX
                };

            let fs_info = Self::filesystem_info(&dir_name);
            let (system_disk, formatted) = if let Some(v) = fs_info { v } else { (false, 0) };

            self.disks.push(Disk {
                name: dir_name,
                r#type,
                capacity,
                formatted,
                system_disk,
                busy_percent: 0.0,

                response_time_ms: 0,
                read_speed: 0,
                write_speed: 0,
            });
        }
    }

    pub fn disks(&self) -> &[Disk] {
        &self.disks
    }

    fn filesystem_info(device_name: &str) -> Option<(bool, u64)> {
        use gtk::glib::*;

        let entries = std::fs::read_dir(format!("/sys/block/{}", device_name));
        if entries.is_err() {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to read filesystem information for '{}': {:?}",
                device_name,
                entries.err()
            );

            return None;
        }

        let _mount_points = Self::mount_points(&device_name);

        let is_root_device = Self::mount_points(&device_name)
            .iter()
            .map(|v| v.as_str())
            .any(|v| v == "/");
        let mut formatted_size = 0_u64;
        for entry in entries.unwrap() {
            if entry.is_err() {
                g_warning!(
                    "MissionCenter::SysInfo",
                    "Failed to read some filesystem information for '{}': {:?}",
                    device_name,
                    entry.err()
                );
                continue;
            }

            let part_name = entry.unwrap().file_name();
            let part_name = part_name.to_string_lossy();
            if !part_name.starts_with(device_name) {
                continue;
            }
            std::fs::read_to_string(format!("/sys/block/{}/{}/size", &device_name, part_name))
                .ok()
                .map(|v| v.trim().parse::<u64>().ok().map_or(0, |v| v * 512))
                .map(|v| {
                    formatted_size += v;
                });
        }

        Some((is_root_device, formatted_size))
    }

    fn mount_points(device_name: &str) -> Vec<String> {
        use gtk::glib::*;

        let is_flatpak = *super::IS_FLATPAK;
        let mut cmd = if is_flatpak {
            let mut cmd = std::process::Command::new(super::FLATPAK_SPAWN_CMD);
            cmd.arg("--host").arg("lsblk");

            cmd
        } else {
            std::process::Command::new("lsblk")
        };
        cmd.arg("-o").arg("NAME,MOUNTPOINTS").arg("--json");

        let lsblk_out = if let Ok(output) = cmd.output() {
            if output.stderr.len() > 0 {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to refresh block device information, host command execution failed: {}",
                    std::str::from_utf8(output.stderr.as_slice()).unwrap_or("Unknown error")
                );
                return vec![];
            }

            output.stdout
        } else {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to refresh block device information, host command execution failed"
            );
            return vec![];
        };

        let lsblk_out = serde_json::from_slice::<LSBLKOutput>(lsblk_out.as_slice());
        if lsblk_out.is_err() {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to refresh block device information, host command execution failed: {}",
                lsblk_out.err().unwrap()
            );
            return vec![];
        }

        let lsblk_out = lsblk_out.unwrap();

        let mut mount_points = vec![];
        for block_device in lsblk_out.blockdevices {
            if block_device.is_none() {
                continue;
            }

            let block_device = block_device.unwrap();
            if block_device.name != device_name {
                continue;
            }

            let children = block_device.children;
            if children.is_none() {
                break;
            }

            fn find_mount_points(
                block_devices: Vec<Option<LSBLKBlockDevice>>,
                mount_points: &mut Vec<String>,
            ) {
                for block_device in block_devices {
                    if block_device.is_none() {
                        continue;
                    }
                    let block_device = block_device.unwrap();

                    for mountpoint in block_device.mountpoints {
                        if mountpoint.is_none() {
                            continue;
                        }

                        mount_points.push(mountpoint.unwrap());
                    }

                    if let Some(children) = block_device.children {
                        find_mount_points(children, mount_points);
                    }
                }
            }

            find_mount_points(children.unwrap(), &mut mount_points);
            break;
        }

        mount_points
    }
}

#[derive(Debug, Deserialize)]
struct LSBLKBlockDevice {
    name: String,
    mountpoints: Vec<Option<String>>,
    children: Option<Vec<Option<LSBLKBlockDevice>>>,
}

#[derive(Debug, Deserialize)]
struct LSBLKOutput {
    blockdevices: Vec<Option<LSBLKBlockDevice>>,
}
