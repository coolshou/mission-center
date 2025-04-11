/* models/processes.rs
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

use std::collections::HashMap;

use gtk::gio;
use gtk::prelude::*;

use magpie_types::processes::Process;

use crate::apps_page::row_model::{ContentType, RowModel, RowModelBuilder};

pub fn update(
    process_map: &HashMap<u32, Process>,
    pid: &u32,
    list: &gio::ListStore,
    app_icons: &HashMap<u32, String>,
    icon: &str,
    use_merged_stats: bool,
    models: &mut HashMap<u32, RowModel>,
) {
    let Some(process) = process_map.get(&pid) else {
        return;
    };

    let pretty_name = if process.exe.is_empty() {
        if let Some(cmd) = process.cmd.first() {
            cmd.split_ascii_whitespace()
                .next()
                .and_then(|s| s.split('/').last())
                .unwrap_or(&process.name)
        } else {
            &process.name
        }
    } else {
        let exe_name = process.exe.split('/').last().unwrap_or(&process.name);
        if exe_name.starts_with("wine") {
            if process.cmd.is_empty() {
                process.name.as_str()
            } else {
                process.cmd[0]
                    .split("\\")
                    .last()
                    .unwrap_or(&process.name)
                    .split("/")
                    .last()
                    .unwrap_or(&process.name)
            }
        } else {
            exe_name
        }
    };

    let row_model = if let Some(index) = list.find_with_equal_func(|obj| {
        let Some(row_model) = obj.downcast_ref::<RowModel>() else {
            return false;
        };
        row_model.pid() == process.pid
    }) {
        unsafe {
            list.item(index)
                .and_then(|obj| obj.downcast().ok())
                .unwrap_unchecked()
        }
    } else {
        let row_model = RowModelBuilder::new()
            .content_type(ContentType::Process)
            .id(&process.pid.to_string())
            .build();
        list.append(&row_model);
        row_model
    };

    let prev_children = row_model.children();
    let mut to_remove = Vec::with_capacity(prev_children.n_items() as _);
    for i in (0..prev_children.n_items()).rev() {
        let Some(child) = prev_children
            .item(i)
            .and_then(|obj| obj.downcast::<RowModel>().ok())
        else {
            to_remove.push(i);
            continue;
        };

        if process.children.contains(&child.pid()) {
            continue;
        }

        to_remove.push(i);
    }
    for i in to_remove {
        prev_children.remove(i);
    }

    let merged_usage_stats = process.merged_usage_stats(&process_map);
    let usage_stats = if use_merged_stats {
        &merged_usage_stats
    } else {
        &process.usage_stats
    };

    let icon = if let Some(icon) = app_icons.get(&process.pid) {
        icon.as_str()
    } else {
        icon
    };

    row_model.set_name(pretty_name);
    row_model.set_icon(icon);
    row_model.set_pid(process.pid);
    row_model.set_cpu_usage(usage_stats.cpu_usage);
    row_model.set_memory_usage(usage_stats.memory_usage);
    row_model.set_shared_memory_usage(usage_stats.shared_memory_usage);
    row_model.set_disk_usage(usage_stats.disk_usage);
    row_model.set_network_usage(usage_stats.network_usage);
    row_model.set_gpu_usage(usage_stats.gpu_usage);
    row_model.set_gpu_memory_usage(usage_stats.gpu_memory_usage);

    for child in &process.children {
        update(
            process_map,
            child,
            &row_model.children(),
            app_icons,
            icon,
            use_merged_stats,
            models,
        );
    }

    models.insert(process.pid, row_model);
}
