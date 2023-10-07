/* sys_info_v2/mod.rs
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

mod app_info;
mod cpu_info;
mod disk_info;
mod gatherer;
mod gpu_info;
mod mem_info;
mod net_info;
mod proc_info;

#[allow(dead_code)]
pub type CpuStaticInfo = cpu_info::StaticInfo;
#[allow(dead_code)]
pub type CpuDynamicInfo = cpu_info::DynamicInfo;

pub type MemInfo = mem_info::MemInfo;
#[allow(dead_code)]
pub type MemoryDevice = mem_info::MemoryDevice;

pub type Disk = disk_info::Disk;
pub type DiskInfo = disk_info::DiskInfo;
pub type DiskType = disk_info::DiskType;

pub type NetInfo = net_info::NetInfo;
pub type NetworkDevice = net_info::NetworkDevice;
pub type NetDeviceType = net_info::NetDeviceType;

pub type GpuStaticInfo = gpu_info::StaticInfo;
pub type GpuDynamicInfo = gpu_info::DynamicInfo;

pub type App = app_info::App;
pub type Process = proc_info::Process;
pub type Pid = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSpeed {
    VerySlow = 4,
    Slow = 3,
    Normal = 2,
    Fast = 1,
}

#[derive(Debug)]
pub enum TerminateType {
    Normal,
    Force,
}

impl From<i32> for UpdateSpeed {
    fn from(value: i32) -> Self {
        use gtk::glib::*;

        match value {
            1 => Self::Fast,
            2 => Self::Normal,
            3 => Self::Slow,
            4 => Self::VerySlow,
            _ => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Invalid update speed value: {}, defaulting to Normal",
                    value
                );
                Self::Normal
            }
        }
    }
}

#[derive(Debug)]
pub struct Readings {
    pub cpu_static_info: CpuStaticInfo,
    pub cpu_dynamic_info: CpuDynamicInfo,
    pub mem_info: MemInfo,
    pub disks: Vec<Disk>,
    pub network_devices: Vec<NetworkDevice>,
    pub gpu_static_info: Vec<gpu_info::StaticInfo>,
    pub gpu_dynamic_info: Vec<gpu_info::DynamicInfo>,

    pub running_apps: std::collections::HashMap<String, App>,
    pub process_tree: Process,
}

#[derive(Default)]
pub struct SysInfoV2 {}

impl SysInfoV2 {
    pub fn new() -> Self {
        Self::default()
    }
}
