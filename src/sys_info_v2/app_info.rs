/* sys_info_v2/app_info.rs
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

pub type Stats = super::gatherer::AppStats;

#[derive(Debug, Default, Clone)]
pub struct App {
    base: super::gatherer::AppDescriptor,
    pub pids: Vec<u32>,
}

impl App {
    pub fn new(base: super::gatherer::AppDescriptor) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.base.name
    }

    #[inline]
    pub fn icon(&self) -> Option<&str> {
        self.base.icon.as_deref()
    }

    #[inline]
    pub fn id(&self) -> &str {
        self.base.id.as_str()
    }

    #[inline]
    pub fn command(&self) -> &str {
        self.base.command.as_str()
    }

    #[inline]
    pub fn stats(&self) -> &Stats {
        &self.base.stats
    }
}

impl super::GathererSupervisor {
    pub fn apps(&mut self) -> std::collections::HashMap<String, App> {
        use super::gatherer::SharedDataContent;
        use gtk::glib::*;

        let mut running_apps = vec![];
        self.execute(
            super::gatherer::Message::GetApps,
            |gatherer, process_restarted| {
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
                    SharedDataContent::Apps(ref apps) => {
                        if process_restarted {
                            running_apps.clear();
                        }

                        for app in &apps.apps {
                            running_apps.push(App::new(app.clone()));
                        }
                        apps.is_complete
                    }
                    SharedDataContent::Processes(_) => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is Processes instead of Apps; encountered when reading installed apps from gatherer",
                        );
                        false
                    }
                    SharedDataContent::AppPIDs(_) => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is AppPIDs instead of Apps; encountered when reading installed apps from gatherer",
                        );
                        false
                    }
                    SharedDataContent::CpuStaticInfo(_) => {
                        g_critical!(
                            "MissionCenter::ProcInfo",
                            "Shared data content is CpuStaticInfo instead of Apps; encountered when reading processes from gatherer", 
                        );
                        false
                    }
                    SharedDataContent::Monostate => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is Monostate instead of Apps; encountered when reading installed apps from gatherer",
                        );
                        false
                    }
                }
            },
        );

        let mut result = std::collections::HashMap::new();
        if running_apps.is_empty() {
            return result;
        }

        let mut current_app_index = 0_usize;
        let mut current_app = running_apps[current_app_index].clone();
        self.execute(
            super::gatherer::Message::GetAppPIDs,
            |gatherer, process_restarted| {
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
                    SharedDataContent::AppPIDs(ref pids) => {
                        if process_restarted {
                            g_critical!("MissionCenter::AppInfo", "Gatherer process restarted while reading app PIDs from it, incomplete data will be shown");
                            return true;
                        }

                        for pid in &pids.pids {
                            if *pid == 0 {
                                result.insert(current_app.id().to_string(), core::mem::take(&mut current_app));

                                current_app_index += 1;
                                if current_app_index >= running_apps.len() {
                                    break;
                                }
                                current_app = running_apps[current_app_index].clone();
                                continue;
                            }

                            current_app.pids.push(*pid);
                        }
                        pids.is_complete
                    }
                    SharedDataContent::Processes(_) => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is Processes instead of AppPIDs; encountered when reading installed apps from gatherer",
                        );
                        false
                    }
                    SharedDataContent::Apps(_) => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is Apps instead of AppPIDs; encountered when reading installed apps from gatherer",
                        );
                        false
                    }
                    SharedDataContent::CpuStaticInfo(_) => {
                        g_critical!(
                            "MissionCenter::ProcInfo",
                            "Shared data content is CpuStaticInfo instead of AppPIDs; encountered when reading processes from gatherer", 
                        );
                        false
                    }
                    SharedDataContent::Monostate => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is Monostate instead of AppPIDs; encountered when reading installed apps from gatherer",
                        );
                        false
                    }
                }
            },
        );

        result
    }
}
