/* models/apps.rs
 *
 * Copyright 2025 Mission Center Developers
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

use std::collections::{HashMap, HashSet};

use adw::prelude::*;
use glib::g_critical;
use gtk::{gio, glib};

use magpie_types::apps::icon::Icon;
use magpie_types::apps::App;
use magpie_types::processes::{Process, ProcessUsageStats};

use crate::apps_page::row_model::{ContentType, RowModel, RowModelBuilder};

pub fn update(
    app_map: &HashMap<String, App>,
    process_map: &HashMap<u32, Process>,
    process_model_map: &HashMap<u32, RowModel>,
    app_icons: &mut HashMap<u32, String>,
    list: &gio::ListStore,
    use_merged_stats: bool,
) {
    app_icons.clear();

    let mut to_remove = Vec::with_capacity(list.n_items() as _);
    for i in (0..list.n_items()).rev() {
        let Some(app) = list.item(i).and_then(|obj| obj.downcast::<RowModel>().ok()) else {
            to_remove.push(i);
            continue;
        };

        if app_map.contains_key(app.id().as_str()) {
            continue;
        }

        to_remove.push(i);
    }
    for i in &to_remove {
        list.remove(*i);
    }

    for app in app_map.values() {
        let primary_processes = primary_processes(app, process_map);
        if primary_processes.is_empty() {
            g_critical!(
                "MissionCenter::AppsPage",
                "Failed to find primary PID for app {}",
                app.name
            );
            continue;
        }

        let row_model = if let Some(index) = list.find_with_equal_func(|obj| {
            let Some(row_model) = obj.downcast_ref::<RowModel>() else {
                return false;
            };
            row_model.id() == app.id
        }) {
            unsafe {
                list.item(index)
                    .and_then(|obj| obj.downcast().ok())
                    .unwrap_unchecked()
            }
        } else {
            let row_model = RowModelBuilder::new()
                .content_type(ContentType::App)
                .id(&app.id)
                .build();
            list.append(&row_model);
            row_model
        };

        let icon = app
            .icon
            .as_ref()
            .map(|i| match &i.icon {
                Some(Icon::Path(p)) => p,
                Some(Icon::Id(i)) => i,
                _ => "application-x-executable",
            })
            .unwrap_or("application-x-executable");

        to_remove.clear();
        let app_children = row_model.children();
        for i in (0..app_children.n_items()).rev() {
            let Some(child) = app_children
                .item(i)
                .and_then(|obj| obj.downcast::<RowModel>().ok())
            else {
                to_remove.push(i);
                continue;
            };

            if primary_processes.iter().any(|p| p.pid == child.pid()) {
                continue;
            }

            to_remove.push(i);
        }
        for i in &to_remove {
            app_children.remove(*i);
        }

        let mut usage_stats = ProcessUsageStats::default();
        for process in &primary_processes {
            usage_stats.merge(
                &(if use_merged_stats {
                    process.merged_usage_stats(&process_map)
                } else {
                    process.usage_stats
                }),
            );

            app_icons.insert(process.pid, icon.to_string());

            if app_children
                .find_with_equal_func(|p| {
                    if let Some(obj) = p.downcast_ref::<RowModel>() {
                        obj.pid() == process.pid
                    } else {
                        false
                    }
                })
                .is_some()
            {
                continue;
            }

            if let Some(row_model) = process_model_map.get(&process.pid) {
                app_children.append(row_model);
            }
        }

        row_model.set_name(app.name.as_str());
        row_model.set_icon(icon);
        row_model.set_cpu_usage(usage_stats.cpu_usage);
        row_model.set_memory_usage(usage_stats.memory_usage);
        row_model.set_shared_memory_usage(usage_stats.shared_memory_usage);
        row_model.set_disk_usage(usage_stats.disk_usage);
        row_model.set_network_usage(usage_stats.network_usage);
        row_model.set_gpu_usage(usage_stats.gpu_usage);
        row_model.set_gpu_memory_usage(usage_stats.gpu_memory_usage);
    }
}

fn primary_processes<'a>(app: &App, process_map: &'a HashMap<u32, Process>) -> Vec<&'a Process> {
    let mut secondary_processes = HashSet::new();
    for app_pid in app.pids.iter() {
        if let Some(process) = process_map.get(app_pid) {
            for child in &process.children {
                if app.pids.contains(child) {
                    secondary_processes.insert(*child);
                }
            }
        }
    }

    let mut primary_processes = Vec::new();
    for app_pid in app.pids.iter() {
        if let Some(process) = process_map.get(app_pid) {
            if !secondary_processes.contains(&process.pid) {
                primary_processes.push(process);
            }
        }
    }

    if primary_processes.is_empty() {
        for (index, pid) in app.pids.iter().enumerate() {
            if let Some(process) = process_map.get(pid) {
                if process.children.len() > 0 || index == app.pids.len() - 1 {
                    primary_processes.push(process);
                    break;
                }
            }
        }
    }

    primary_processes
}
