/* sys_info_v2/cpu_info.rs
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

use super::GathererSupervisor;

pub type StaticInfo = super::gatherer::CpuStaticInfo;

impl GathererSupervisor {
    pub fn cpu_static_info(&mut self) -> StaticInfo {
        use super::gatherer::SharedDataContent;
        use gtk::glib::*;

        let mut result = Default::default();

        self.execute(
            super::gatherer::Message::GetCpuStaticInfo,
            |gatherer, _| {
                let shared_memory = match gatherer.shared_memory() {
                    Ok(shm) => shm,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Unable to to access shared memory: {}",
                            e
                        );
                        return false;
                    }
                };

                match shared_memory.content {
                    SharedDataContent::CpuStaticInfo(ref static_info) => {
                        result = static_info.clone();
                        true
                    }
                    SharedDataContent::Processes(_) => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is Processes instead of CpuStaticInfo; encountered when reading installed apps from gatherer",
                        );
                        false
                    }
                    SharedDataContent::Apps(_) => {
                        g_critical!(
                            "MissionCenter::ProcInfo",
                            "Shared data content is Apps instead of CpuStaticInfo; encountered when reading processes from gatherer", 
                        );
                        false
                    }
                    SharedDataContent::AppPIDs(_) => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is AppPIDs instead of CpuStaticInfo; encountered when reading installed apps from gatherer",
                        );
                        false
                    }
                    SharedDataContent::Monostate => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is Monostate instead of CpuStaticInfo; encountered when reading installed apps from gatherer",
                        );
                        false
                    }
                }
            },
        );

        result
    }
}

#[derive(Debug, Clone)]
pub struct DynamicInfo {
    pub utilization_percent: f32,
    pub utilization_percent_per_core: Vec<f32>,
    pub current_frequency_mhz: u64,
    pub temperature: Option<f32>,
    pub process_count: u32,
    pub thread_count: u32,
    pub handle_count: u32,
    pub uptime_seconds: u64,
}

unsafe impl Send for DynamicInfo {}

impl DynamicInfo {
    pub fn load(system: &mut sysinfo::System) -> Self {
        use sysinfo::*;

        system.refresh_cpu_specifics(CpuRefreshKind::new().with_cpu_usage().with_frequency());

        let logical_core_count = num_cpus::get();
        let mut utilization_percent_per_core = vec![];
        utilization_percent_per_core.reserve(logical_core_count);
        for cpu in system.cpus() {
            utilization_percent_per_core.push(cpu.cpu_usage());
        }

        Self {
            utilization_percent: system.global_cpu_info().cpu_usage(),
            utilization_percent_per_core,
            current_frequency_mhz: system.global_cpu_info().frequency(),
            temperature: Self::temperature(),
            process_count: Self::process_count(),
            thread_count: Self::thread_count(),
            handle_count: Self::handle_count(),
            uptime_seconds: system.uptime(),
        }
    }

    fn temperature() -> Option<f32> {
        use gtk::glib::*;

        let dir = match std::fs::read_dir("/sys/class/hwmon") {
            Ok(d) => d,
            Err(e) => {
                g_critical!(
                    "MissionCenter::CpuInfo",
                    "Failed to open `/sys/class/hwmon`: {}",
                    e
                );
                return None;
            }
        };

        for mut entry in dir
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|path| path.is_dir())
        {
            let mut name = entry.clone();
            name.push("name");

            let name = match std::fs::read_to_string(name) {
                Ok(name) => name.trim().to_lowercase(),
                Err(_) => continue,
            };
            if name != "k10temp" && name != "coretemp" {
                continue;
            }

            entry.push("temp1_input");
            let temp = match std::fs::read_to_string(&entry) {
                Ok(temp) => temp,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Failed to read temperature from `{}`: {}",
                        entry.display(),
                        e
                    );
                    continue;
                }
            };

            return Some(match temp.trim().parse::<u32>() {
                Ok(temp) => (temp as f32) / 1000.,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::CpuInfo",
                        "Failed to parse temperature from `{}`: {}",
                        entry.display(),
                        e
                    );
                    continue;
                }
            });
        }

        None
    }

    fn process_count() -> u32 {
        use gtk::glib::*;

        let mut cmd = cmd!("ls -d /proc/[1-9]* | wc -l");

        if let Ok(output) = cmd.output() {
            std::str::from_utf8(output.stdout.as_slice()).map_or(0, |s| match s.trim().parse() {
                Ok(count) => count,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "Failed to get process count, host command output ({}) parsing failed: {}",
                        s,
                        e
                    );
                    0
                }
            })
        } else {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to get process count, host command execution failed"
            );

            return 0;
        }
    }

    fn thread_count() -> u32 {
        use gtk::glib::*;

        // https://askubuntu.com/questions/88972/how-to-get-from-terminal-total-number-of-threads-per-process-and-total-for-al
        let mut cmd = cmd!("count() { printf %s\\\\n \"$#\" ; } ; count /proc/[0-9]*/task/[0-9]*");

        if let Ok(output) = cmd.output() {
            if output.stderr.len() > 0 {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to get thread count, host command execution failed: {}",
                    String::from_utf8_lossy(output.stderr.as_slice())
                );
                return 0;
            }

            std::str::from_utf8(output.stdout.as_slice()).map_or(0, |s| match s.trim().parse() {
                Ok(count) => count,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::SysInfo",
                        "Failed to get thread count, host command output ({}) parsing: {}",
                        s,
                        e
                    );
                    0
                }
            })
        } else {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to get thread count, host command execution failed"
            );

            0
        }
    }

    fn handle_count() -> u32 {
        use gtk::glib::*;

        let mut cmd = cmd!("cat /proc/sys/fs/file-nr");

        if let Ok(output) = cmd.output() {
            if output.stderr.len() > 0 {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Failed to get handle count, host command execution failed: {}",
                    String::from_utf8_lossy(output.stderr.as_slice())
                );
                return 0;
            }

            std::str::from_utf8(output.stdout.as_slice()).map_or(0, |s| {
                let s = match s.split_whitespace()
                    .next() {
                    Some(s) => s,
                    None => {
                        g_critical!(
                                "MissionCenter::SysInfo",
                                "Failed to get handle count, host command output ({}) empty or parsing failed",
                                s
                            );
                        return 0;
                    }
                };

                match s.trim().parse() {
                    Ok(count) => count,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Failed to get handle count, host command output ({}) parsing failed: {}",
                            s,
                            e
                        );
                        0
                    }
                }
            })
        } else {
            g_critical!(
                "MissionCenter::SysInfo",
                "Failed to get handle count, host command execution failed"
            );
            0
        }
    }
}

#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub static_info: StaticInfo,
    pub dynamic_info: DynamicInfo,
}

impl CpuInfo {
    pub fn new(system: &mut sysinfo::System, gatherer_supervisor: &mut GathererSupervisor) -> Self {
        let static_info = gatherer_supervisor.cpu_static_info();
        let dynamic_info = DynamicInfo::load(system);

        Self {
            static_info,
            dynamic_info,
        }
    }
}
