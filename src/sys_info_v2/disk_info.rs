/* sys_info_v2/disk_info.rs
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

#[derive(Debug, Default, Deserialize)]
struct LSBLKBlockDevice {
    name: String,
    mountpoints: Vec<Option<String>>,
    children: Option<Vec<Option<LSBLKBlockDevice>>>,
}

#[derive(Debug, Deserialize)]
struct LSBLKOutput {
    blockdevices: Vec<Option<LSBLKBlockDevice>>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DiskType {
    Unknown,
    HDD,
    SSD,
    NVMe,
    eMMC,
    iSCSI,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Disk {
    pub id: String,
    pub model: String,
    pub r#type: DiskType,
    pub capacity: u64,
    pub formatted: u64,
    pub system_disk: bool,

    pub busy_percent: f32,
    pub response_time_ms: f32,
    pub read_speed: u64,
    pub write_speed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiskStats {
    pub id: String,
    pub sectors_read: u64,
    pub sectors_written: u64,

    pub read_ios: u64,
    pub write_ios: u64,
    pub discard_ios: u64,
    pub flush_ios: u64,
    pub io_total_time_ms: u64,

    pub read_ticks_weighted_ms: u64,
    pub write_ticks_weighted_ms: u64,
    pub discard_ticks_weighted_ms: u64,
    pub flush_ticks_weighted_ms: u64,

    pub read_time_ms: std::time::Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiskInfo;

impl DiskInfo {
    pub fn load(disk_stats: &mut Vec<DiskStats>) -> Vec<Disk> {
        use gtk::glib::*;

        let mut result = vec![];

        let entries = match std::fs::read_dir("/sys/block") {
            Ok(e) => e,
            Err(e) => {
                g_critical!(
                    "MissionCenter::DiskInfo",
                    "Failed to refresh disk information, failed to read disk entries: {}",
                    e
                );
                return result;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    g_warning!("MissionCenter::SysInfo", "Failed to read disk entry: {}", e);
                    continue;
                }
            };
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(e) => {
                    g_warning!(
                        "MissionCenter::SysInfo",
                        "Failed to read disk entry file type: {}",
                        e
                    );
                    continue;
                }
            };

            let dir_name = if file_type.is_symlink() {
                let path = match entry.path().read_link() {
                    Err(e) => {
                        g_warning!(
                            "MissionCenter::SysInfo",
                            "Failed to read disk entry symlink: {}",
                            e
                        );
                        continue;
                    }
                    Ok(p) => {
                        let path = std::path::Path::new("/sys/block").join(p);
                        if !path.is_dir() {
                            continue;
                        }
                        path
                    }
                };

                match path.file_name() {
                    None => continue,
                    Some(dir_name) => dir_name.to_string_lossy().into_owned(),
                }
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

            let mut disk_index = None;
            for i in 0..disk_stats.len() {
                if disk_stats[i].id == dir_name {
                    disk_index = Some(i);
                    break;
                }
            }

            let r#type = if let Ok(v) =
                std::fs::read_to_string(format!("/sys/block/{}/queue/rotational", dir_name))
            {
                let v = v.trim().parse::<u8>().ok().map_or(u8::MAX, |v| v);
                if v == 0 {
                    if dir_name.starts_with("nvme") {
                        DiskType::NVMe
                    } else if dir_name.starts_with("mmc") {
                        DiskType::eMMC
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

            let vendor = std::fs::read_to_string(format!("/sys/block/{}/device/vendor", dir_name))
                .ok()
                .unwrap_or("".to_string());

            let model = std::fs::read_to_string(format!("/sys/block/{}/device/model", dir_name))
                .ok()
                .unwrap_or("".to_string());

            let model = vendor.trim().to_string() + " " + model.trim();

            let stats = std::fs::read_to_string(format!("/sys/block/{}/stat", dir_name));

            let stats = match stats.as_ref() {
                Err(e) => {
                    g_warning!(
                        "MissionCenter::SysInfo",
                        "Failed to read disk stat: {:?}",
                        e
                    );
                    ""
                }
                Ok(stats) => stats.trim(),
            };

            let mut read_ios = 0;
            let mut sectors_read = 0;
            let mut read_ticks_weighted_ms = 0;
            let mut write_ios = 0;
            let mut sectors_written = 0;
            let mut write_ticks_weighted_ms = 0;
            let mut io_total_time_ms: u64 = 0;
            let mut discard_ios = 0;
            let mut discard_ticks_weighted_ms = 0;
            let mut flush_ios = 0;
            let mut flush_ticks_weighted_ms = 0;

            const IDX_READ_IOS: usize = 0;
            const IDX_READ_SECTORS: usize = 2;
            const IDX_READ_TICKS: usize = 3;
            const IDX_WRITE_IOS: usize = 4;
            const IDX_WRITE_SECTORS: usize = 6;
            const IDX_WRITE_TICKS: usize = 7;
            const IDX_IO_TICKS: usize = 9;
            const IDX_DISCARD_IOS: usize = 11;
            const IDX_DISCARD_TICKS: usize = 14;
            const IDX_FLUSH_IOS: usize = 15;
            const IDX_FLUSH_TICKS: usize = 16;
            for (i, entry) in stats
                .split_whitespace()
                .enumerate()
                .map(|(i, v)| (i, v.trim()))
            {
                match i {
                    IDX_READ_IOS => read_ios = entry.parse::<u64>().unwrap_or(0),
                    IDX_READ_SECTORS => sectors_read = entry.parse::<u64>().unwrap_or(0),
                    IDX_READ_TICKS => read_ticks_weighted_ms = entry.parse::<u64>().unwrap_or(0),
                    IDX_WRITE_IOS => write_ios = entry.parse::<u64>().unwrap_or(0),
                    IDX_WRITE_SECTORS => sectors_written = entry.parse::<u64>().unwrap_or(0),
                    IDX_WRITE_TICKS => write_ticks_weighted_ms = entry.parse::<u64>().unwrap_or(0),
                    IDX_IO_TICKS => {
                        io_total_time_ms = entry.parse::<u64>().unwrap_or(0);
                    }
                    IDX_DISCARD_IOS => discard_ios = entry.parse::<u64>().unwrap_or(0),
                    IDX_DISCARD_TICKS => {
                        discard_ticks_weighted_ms = entry.parse::<u64>().unwrap_or(0)
                    }
                    IDX_FLUSH_IOS => flush_ios = entry.parse::<u64>().unwrap_or(0),
                    IDX_FLUSH_TICKS => {
                        flush_ticks_weighted_ms = entry.parse::<u64>().unwrap_or(0);
                        break;
                    }
                    _ => (),
                }
            }

            if let Some(disk_index) = disk_index {
                let disk_stat = &mut disk_stats[disk_index];

                let read_ticks_weighted_ms_prev =
                    if read_ticks_weighted_ms < disk_stat.read_ticks_weighted_ms {
                        read_ticks_weighted_ms
                    } else {
                        disk_stat.read_ticks_weighted_ms
                    };

                let write_ticks_weighted_ms_prev =
                    if write_ticks_weighted_ms < disk_stat.write_ticks_weighted_ms {
                        write_ticks_weighted_ms
                    } else {
                        disk_stat.write_ticks_weighted_ms
                    };

                let discard_ticks_weighted_ms_prev =
                    if discard_ticks_weighted_ms < disk_stat.discard_ticks_weighted_ms {
                        discard_ticks_weighted_ms
                    } else {
                        disk_stat.discard_ticks_weighted_ms
                    };

                let flush_ticks_weighted_ms_prev =
                    if flush_ticks_weighted_ms < disk_stat.flush_ticks_weighted_ms {
                        flush_ticks_weighted_ms
                    } else {
                        disk_stat.flush_ticks_weighted_ms
                    };

                let elapsed = disk_stat.read_time_ms.elapsed().as_secs_f32();

                let delta_read_ticks_weighted_ms =
                    read_ticks_weighted_ms - read_ticks_weighted_ms_prev;
                let delta_write_ticks_weighted_ms =
                    write_ticks_weighted_ms - write_ticks_weighted_ms_prev;
                let delta_discard_ticks_weighted_ms =
                    discard_ticks_weighted_ms - discard_ticks_weighted_ms_prev;
                let delta_flush_ticks_weighted_ms =
                    flush_ticks_weighted_ms - flush_ticks_weighted_ms_prev;
                let delta_ticks_weighted_ms = delta_read_ticks_weighted_ms
                    + delta_write_ticks_weighted_ms
                    + delta_discard_ticks_weighted_ms
                    + delta_flush_ticks_weighted_ms;

                // Arbitrary math is arbitrary
                let busy_percent = (delta_ticks_weighted_ms as f32 / (elapsed * 8.0)).min(100.);

                disk_stat.read_ticks_weighted_ms = read_ticks_weighted_ms;
                disk_stat.write_ticks_weighted_ms = write_ticks_weighted_ms;
                disk_stat.discard_ticks_weighted_ms = discard_ticks_weighted_ms;
                disk_stat.flush_ticks_weighted_ms = flush_ticks_weighted_ms;

                let io_time_ms_prev = if io_total_time_ms < disk_stat.io_total_time_ms {
                    io_total_time_ms
                } else {
                    disk_stat.io_total_time_ms
                };

                let read_ios_prev = if read_ios < disk_stat.read_ios {
                    read_ios
                } else {
                    disk_stat.read_ios
                };

                let write_ios_prev = if write_ios < disk_stat.write_ios {
                    write_ios
                } else {
                    disk_stat.write_ios
                };

                let discard_ios_prev = if discard_ios < disk_stat.discard_ios {
                    discard_ios
                } else {
                    disk_stat.discard_ios
                };

                let flush_ios_prev = if flush_ios < disk_stat.flush_ios {
                    flush_ios
                } else {
                    disk_stat.flush_ios
                };

                let delta_io_time_ms = io_total_time_ms - io_time_ms_prev;
                let delta_read_ios = read_ios - read_ios_prev;
                let delta_write_ios = write_ios - write_ios_prev;
                let delta_discard_ios = discard_ios - discard_ios_prev;
                let delta_flush_ios = flush_ios - flush_ios_prev;

                let delta_ios =
                    delta_read_ios + delta_write_ios + delta_discard_ios + delta_flush_ios;
                let response_time_ms = if delta_ios > 0 {
                    delta_io_time_ms as f32 / delta_ios as f32
                } else {
                    0.
                };

                disk_stat.read_ios = read_ios;
                disk_stat.write_ios = write_ios;
                disk_stat.discard_ios = discard_ios;
                disk_stat.flush_ios = flush_ios;
                disk_stat.io_total_time_ms = io_total_time_ms;

                let sectors_read_prev = if sectors_read < disk_stat.sectors_read {
                    sectors_read
                } else {
                    disk_stat.sectors_read
                };

                let sectors_written_prev = if sectors_written < disk_stat.sectors_written {
                    sectors_written
                } else {
                    disk_stat.sectors_written
                };

                let read_speed = ((sectors_read - sectors_read_prev) as f32 * 512.) / elapsed;
                let write_speed =
                    ((sectors_written - sectors_written_prev) as f32 * 512.) / elapsed;

                let read_speed = read_speed.round() as u64;
                let write_speed = write_speed.round() as u64;

                disk_stat.sectors_read = sectors_read;
                disk_stat.sectors_written = sectors_written;

                disk_stat.read_time_ms = std::time::Instant::now();

                result.push(Disk {
                    id: dir_name,
                    r#type,
                    model: model.trim().to_string(),
                    capacity,
                    formatted,
                    system_disk,

                    busy_percent,
                    response_time_ms,
                    read_speed,
                    write_speed,
                });
            } else {
                result.push(Disk {
                    id: dir_name.clone(),
                    r#type,
                    model: model.trim().to_string(),
                    capacity,
                    formatted,
                    system_disk,

                    busy_percent: 0.,
                    response_time_ms: 0.,
                    read_speed: 0,
                    write_speed: 0,
                });

                disk_stats.push(DiskStats {
                    id: dir_name,
                    read_time_ms: std::time::Instant::now(),
                    read_ticks_weighted_ms,
                    write_ticks_weighted_ms,
                    discard_ticks_weighted_ms,
                    flush_ticks_weighted_ms,
                    io_total_time_ms,
                    read_ios,
                    write_ios,
                    discard_ios,
                    flush_ios,
                    sectors_read,
                    sectors_written,
                })
            }
        }

        result
    }

    fn filesystem_info(device_name: &str) -> Option<(bool, u64)> {
        use gtk::glib::*;

        let entries = match std::fs::read_dir(format!("/sys/block/{}", device_name)) {
            Ok(e) => e,
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to read filesystem information for '{}': {}",
                    device_name,
                    e
                );

                return None;
            }
        };

        let is_root_device = Self::mount_points(&device_name)
            .iter()
            .map(|v| v.as_str())
            .any(|v| v == "/");
        let mut formatted_size = 0_u64;
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    g_warning!(
                        "MissionCenter::SysInfo",
                        "Failed to read some filesystem information for '{}': {}",
                        device_name,
                        e
                    );
                    continue;
                }
            };

            let part_name = entry.file_name();
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

        let lsblk_out = if let Ok(output) = cmd!("lsblk -o NAME,MOUNTPOINTS --json").output() {
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

        let mut lsblk_out = match serde_json::from_slice::<LSBLKOutput>(lsblk_out.as_slice()) {
            Ok(v) => v,
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to refresh block device information, host command execution failed: {}",
                    e
                );
                return vec![];
            }
        };

        let mut mount_points = vec![];
        for block_device in lsblk_out
            .blockdevices
            .iter_mut()
            .filter_map(|bd| bd.as_mut())
        {
            let block_device = core::mem::take(block_device);
            if block_device.name != device_name {
                continue;
            }

            let children = match block_device.children {
                None => break,
                Some(c) => c,
            };

            fn find_mount_points(
                mut block_devices: Vec<Option<LSBLKBlockDevice>>,
                mount_points: &mut Vec<String>,
            ) {
                for block_device in block_devices.iter_mut().filter_map(|bd| bd.as_mut()) {
                    let mut block_device = core::mem::take(block_device);

                    for mountpoint in block_device
                        .mountpoints
                        .iter_mut()
                        .filter_map(|mp| mp.as_mut())
                    {
                        mount_points.push(core::mem::take(mountpoint));
                    }

                    if let Some(children) = block_device.children {
                        find_mount_points(children, mount_points);
                    }
                }
            }

            find_mount_points(children, &mut mount_points);
            break;
        }

        mount_points
    }
}
