use std::collections::HashMap;

use adw::prelude::*;
use glib::g_critical;
use gtk::{gio, glib};

use magpie_types::apps::icon::Icon;
use magpie_types::apps::App;
use magpie_types::processes::Process;

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
    for i in to_remove {
        list.remove(i);
    }

    for app in app_map.values() {
        // Find the first process that has any children. This is most likely the root
        // of the App's process tree.
        let mut primary_pid = 0;
        for (index, pid) in app.pids.iter().enumerate() {
            if let Some(process) = process_map.get(pid) {
                if process.children.len() > 0 || index == app.pids.len() - 1 {
                    primary_pid = process.pid;
                    break;
                }
            }
        }

        if primary_pid == 0 {
            g_critical!(
                "MissionCenter::AppsPage",
                "Failed to find primary PID for app {}",
                app.name
            );
            continue;
        }

        let Some(primary_process) = process_map.get(&primary_pid) else {
            g_critical!(
                "MissionCenter::AppsPage",
                "Failed to find primary PID {} for app {}",
                primary_pid,
                app.name
            );
            continue;
        };

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
            if let Some(process_model) = process_model_map.get(&primary_pid) {
                row_model.set_children(process_model.children());
            }
            list.append(&row_model);
            row_model
        };

        let usage_stats = if use_merged_stats {
            primary_process.merged_usage_stats(&process_map)
        } else {
            primary_process.usage_stats
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

        app_icons.insert(primary_pid, icon.to_string());

        row_model.set_name(app.name.as_str());
        row_model.set_icon(icon);
        row_model.set_pid(primary_pid);
        row_model.set_cpu_usage(usage_stats.cpu_usage);
        row_model.set_memory_usage(usage_stats.memory_usage);
        row_model.set_shared_memory_usage(usage_stats.shared_memory_usage);
        row_model.set_disk_usage(usage_stats.disk_usage);
        row_model.set_network_usage(usage_stats.network_usage);
        row_model.set_gpu_usage(usage_stats.gpu_usage);
        row_model.set_gpu_memory_usage(usage_stats.gpu_memory_usage);
    }
}
