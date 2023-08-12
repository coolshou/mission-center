/* sys_info_v2/mem_info.rs
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

#[derive(Default, Debug, Eq, PartialEq)]
pub struct MemoryDevice {
    pub size: usize,
    pub form_factor: String,
    pub locator: String,
    pub bank_locator: String,
    pub ram_type: String,
    pub speed: usize,
    pub rank: u8,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct MemInfo {
    pub mem_total: usize,
    pub mem_free: usize,
    pub mem_available: usize,
    pub buffers: usize,
    pub cached: usize,
    pub swap_cached: usize,
    pub active: usize,
    pub inactive: usize,
    pub active_anon: usize,
    pub inactive_anon: usize,
    pub active_file: usize,
    pub inactive_file: usize,
    pub unevictable: usize,
    pub m_locked: usize,
    pub swap_total: usize,
    pub swap_free: usize,
    pub zswap: usize,
    pub zswapped: usize,
    pub dirty: usize,
    pub writeback: usize,
    pub anon_pages: usize,
    pub mapped: usize,
    pub sh_mem: usize,
    pub k_reclaimable: usize,
    pub slab: usize,
    pub s_reclaimable: usize,
    pub s_unreclaim: usize,
    pub kernel_stack: usize,
    pub page_tables: usize,
    pub sec_page_tables: usize,
    pub nfs_unstable: usize,
    pub bounce: usize,
    pub writeback_tmp: usize,
    pub commit_limit: usize,
    pub committed_as: usize,
    pub vmalloc_total: usize,
    pub vmalloc_used: usize,
    pub vmalloc_chunk: usize,
    pub percpu: usize,
    pub hardware_corrupted: usize,
    pub anon_huge_pages: usize,
    pub shmem_huge_pages: usize,
    pub shmem_pmd_mapped: usize,
    pub file_huge_pages: usize,
    pub file_pmd_mapped: usize,
    pub cma_total: usize,
    pub cma_free: usize,
    pub huge_pages_total: usize,
    pub huge_pages_free: usize,
    pub huge_pages_rsvd: usize,
    pub huge_pages_surp: usize,
    pub hugepagesize: usize,
    pub hugetlb: usize,
    pub direct_map4k: usize,
    pub direct_map2m: usize,
    pub direct_map1g: usize,
}

impl MemInfo {
    pub fn load() -> Option<Self> {
        use gtk::glib::*;

        let meminfo = if let Ok(output) = cmd!("cat /proc/meminfo").output() {
            if output.stderr.len() > 0 {
                g_critical!(
                    "MissionCenter::MemInfo",
                    "Failed to refresh memory information, host command execution failed: {}",
                    String::from_utf8_lossy(output.stderr.as_slice())
                );
                return None;
            }

            String::from_utf8_lossy(output.stdout.as_slice()).into_owned()
        } else {
            g_critical!(
                "MissionCenter::MemInfo",
                "Failed to refresh memory information, host command execution failed"
            );

            return None;
        };

        let mut this = Self::default();

        for line in meminfo.trim().lines() {
            let mut split = line.split_whitespace();
            let key = split.next().map_or("", |s| s);
            let value = split
                .next()
                .map_or("", |s| s)
                .parse::<usize>()
                .map_or(0, |v| v)
                * 1024;

            match key {
                "MemTotal:" => this.mem_total = value,
                "MemFree:" => this.mem_free = value,
                "MemAvailable:" => this.mem_available = value,
                "Buffers:" => this.buffers = value,
                "Cached:" => this.cached = value,
                "SwapCached:" => this.swap_cached = value,
                "Active:" => this.active = value,
                "Inactive:" => this.inactive = value,
                "Active(anon):" => this.active_anon = value,
                "Inactive(anon):" => this.inactive_anon = value,
                "Active(file):" => this.active_file = value,
                "Inactive(file):" => this.inactive_file = value,
                "Unevictable:" => this.unevictable = value,
                "Mlocked:" => this.m_locked = value,
                "SwapTotal:" => this.swap_total = value,
                "SwapFree:" => this.swap_free = value,
                "ZSwap:" => this.zswap = value,
                "ZSwapTotal:" => this.zswapped = value,
                "Dirty:" => this.dirty = value,
                "Writeback:" => this.writeback = value,
                "AnonPages:" => this.anon_pages = value,
                "Mapped:" => this.mapped = value,
                "Shmem:" => this.sh_mem = value,
                "KReclaimable:" => this.k_reclaimable = value,
                "Slab:" => this.slab = value,
                "SReclaimable:" => this.s_reclaimable = value,
                "SUnreclaim:" => this.s_unreclaim = value,
                "KernelStack:" => this.kernel_stack = value,
                "PageTables:" => this.page_tables = value,
                "SecMemTables:" => this.sec_page_tables = value,
                "NFS_Unstable:" => this.nfs_unstable = value,
                "Bounce:" => this.bounce = value,
                "WritebackTmp:" => this.writeback_tmp = value,
                "CommitLimit:" => this.commit_limit = value,
                "Committed_AS:" => this.committed_as = value,
                "VmallocTotal:" => this.vmalloc_total = value,
                "VmallocUsed:" => this.vmalloc_used = value,
                "VmallocChunk:" => this.vmalloc_chunk = value,
                "Percpu:" => this.percpu = value,
                "HardwareCorrupted:" => this.hardware_corrupted = value,
                "AnonHugePages:" => this.anon_huge_pages = value,
                "ShmemHugePages:" => this.shmem_huge_pages = value,
                "ShmemPmdMapped:" => this.shmem_pmd_mapped = value,
                "FileHugePages:" => this.file_huge_pages = value,
                "FilePmdMapped:" => this.file_pmd_mapped = value,
                "CmaTotal:" => this.cma_total = value,
                "CmaFree:" => this.cma_free = value,
                "HugePages_Total:" => this.huge_pages_total = value / 1024,
                "HugePages_Free:" => this.huge_pages_free = value / 1024,
                "HugePages_Rsvd:" => this.huge_pages_rsvd = value / 1024,
                "HugePages_Surp:" => this.huge_pages_surp = value / 1024,
                "Hugepagesize:" => this.hugepagesize = value,
                "Hugetlb:" => this.hugetlb = value,
                "DirectMap4k:" => this.direct_map4k = value,
                "DirectMap2M:" => this.direct_map2m = value,
                "DirectMap1G:" => this.direct_map1g = value,
                _ => (),
            }
        }

        Some(this)
    }

    pub fn load_memory_device_info() -> Option<Vec<MemoryDevice>> {
        use super::FLATPAK_APP_PATH;
        use gtk::glib::*;
        use std::process::*;

        let is_flatpak = *super::IS_FLATPAK;
        let mut cmd = if !is_flatpak {
            let mut cmd = Command::new("pkexec");
            cmd.arg("dmidecode").arg("--type").arg("17");
            cmd
        } else {
            let cmd = cmd_flatpak_host!(format!(
                "pkexec {}/bin/dmidecode --type 17",
                &*FLATPAK_APP_PATH
            ));
            cmd
        };

        let cmd_output = match cmd.output() {
            Ok(output) => {
                if output.stderr.len() > 0 {
                    g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to read memory device information, host command execution failed: {}",
                    std::str::from_utf8(output.stderr.as_slice()).unwrap_or("Unknown error")
                );
                    return None;
                }

                match std::str::from_utf8(output.stdout.as_slice()) {
                    Ok(out) => out.to_owned(),
                    Err(err) => {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Failed to read memory device information, host command execution failed: {:?}",
                            err
                        );
                        return None;
                    }
                }
            }
            Err(err) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to read memory device information, host command execution failed: {:?}",
                    err
                );
                return None;
            }
        };

        let mut result = vec![];

        let mut index = 0;
        let mut speed_fallback = 0;
        let mut output_str = cmd_output.as_str();
        loop {
            if index >= output_str.len() {
                break;
            }

            let to_parse = output_str.trim();
            let mem_dev_str = "Memory Device";
            index = match to_parse.find(mem_dev_str) {
                None => {
                    break;
                }
                Some(index) => index,
            };
            index += mem_dev_str.len();
            if index < output_str.len() {
                output_str = output_str[index..].trim();
            }

            let mut mem_dev = Some(MemoryDevice::default());

            for line in to_parse[index..].trim().lines() {
                let mut split = line.trim().split(":");
                let key = split.next().map_or("", |s| s).trim();
                let value = split.next().map_or("", |s| s).trim();

                if key == mem_dev_str && value == "" {
                    break;
                }

                let md = match mem_dev.as_mut() {
                    Some(mem_dev) => mem_dev,
                    None => {
                        break;
                    }
                };

                match key {
                    "Size" => {
                        if value.to_lowercase() == "no module installed" {
                            #[allow(dropping_references)]
                            drop(md);

                            mem_dev = None;
                            break;
                        }

                        let (value, unit) = {
                            let mut split = value.trim().split_whitespace();
                            (
                                split.next().map_or("", |s| s),
                                split.next().map_or("", |s| s),
                            )
                        };
                        match unit.trim() {
                            "TB" => {
                                md.size =
                                    value.parse::<usize>().map_or(0, |s| s * 1024 * 1024 * 1024)
                            }
                            "GB" => {
                                md.size =
                                    value.parse::<usize>().map_or(0, |s| s * 1024 * 1024 * 1024)
                            }
                            "MB" => md.size = value.parse::<usize>().map_or(0, |s| s * 1024 * 1024),
                            "KB" => md.size = value.parse::<usize>().map_or(0, |s| s * 1024),
                            _ => md.size = value.parse::<usize>().map_or(0, |s| s),
                        }
                    }
                    "Form Factor" => md.form_factor = value.to_owned(),
                    "Locator" => md.locator = value.to_owned(),
                    "Bank Locator" => md.bank_locator = value.to_owned(),
                    "Type" => md.ram_type = value.to_owned(),
                    "Speed" => {
                        let value = value.trim_end_matches("MT/s").trim();
                        speed_fallback = value.parse::<usize>().map_or(0, |s| s)
                    }
                    "Configured Memory Speed" => {
                        let value = value.trim_end_matches("MT/s").trim();
                        md.speed = value.parse::<usize>().map_or(0, |s| s)
                    }
                    "Rank" => md.rank = value.parse::<u8>().map_or(0, |s| s),
                    _ => (),
                }
            }

            match mem_dev {
                Some(mut mem_dev) => {
                    if mem_dev.speed == 0 {
                        mem_dev.speed = speed_fallback;
                    }
                    result.push(mem_dev);
                }
                _ => {}
            }
        }

        Some(result)
    }
}
