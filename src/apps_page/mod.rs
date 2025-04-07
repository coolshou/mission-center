/* apps_page/mod.rs
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

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use adw::prelude::*;
use gtk::glib::g_critical;
use gtk::{gio, glib, subclass::prelude::*};

use magpie_types::apps::icon::Icon;
use magpie_types::processes::Process;

use crate::magpie_client::App;
use crate::settings;

use columns::*;
use row_model::{ContentType, RowModel, RowModelBuilder, SectionType};

mod columns;
mod models;
mod row_model;

pub const CSS_CELL_USAGE_LOW: &[u8] = b"cell { background-color: rgba(246, 211, 45, 0.3); }";
pub const CSS_CELL_USAGE_MEDIUM: &[u8] = b"cell { background-color: rgba(230, 97, 0, 0.3); }";
pub const CSS_CELL_USAGE_HIGH: &[u8] = b"cell { background-color: rgba(165, 29, 45, 0.3); }";

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/page.ui")]
    pub struct AppsPage {
        #[template_child]
        pub content: TemplateChild<adw::Clamp>,
        #[template_child]
        pub column_view: TemplateChild<gtk::ColumnView>,
        #[template_child]
        pub name_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub pid_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub cpu_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub memory_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub shared_memory_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub drive_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_usage_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_memory_column: TemplateChild<gtk::ColumnViewColumn>,

        pub apps_section: RowModel,
        pub processes_section: RowModel,

        pub root_process: Cell<u32>,
        pub app_icons: RefCell<HashMap<u32, String>>,

        pub use_merged_stats: Cell<bool>,
    }

    impl Default for AppsPage {
        fn default() -> Self {
            Self {
                content: TemplateChild::default(),
                column_view: TemplateChild::default(),
                name_column: TemplateChild::default(),
                pid_column: TemplateChild::default(),
                cpu_column: TemplateChild::default(),
                memory_column: TemplateChild::default(),
                shared_memory_column: TemplateChild::default(),
                drive_column: TemplateChild::default(),
                gpu_usage_column: TemplateChild::default(),
                gpu_memory_column: TemplateChild::default(),

                apps_section: RowModelBuilder::new()
                    .name("Apps")
                    .content_type(ContentType::SectionHeader)
                    .section_type(SectionType::Apps)
                    .build(),
                processes_section: RowModelBuilder::new()
                    .name("Processes")
                    .content_type(ContentType::SectionHeader)
                    .section_type(SectionType::Processes)
                    .build(),

                root_process: Cell::new(1),
                app_icons: RefCell::new(HashMap::new()),

                use_merged_stats: Cell::new(false),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppsPage {
        const NAME: &'static str = "AppsPage";
        type Type = super::AppsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppsPage {
        fn constructed(&self) {
            self.parent_constructed();

            let settings = settings!();

            self.content.set_maximum_size(i32::MAX);

            self.name_column
                .set_factory(Some(&name_list_item_factory()));
            self.name_column
                .set_sorter(Some(&name_sorter(&self.column_view)));

            self.pid_column.set_factory(Some(&pid_list_item_factory()));
            self.pid_column
                .set_sorter(Some(&pid_sorter(&self.column_view)));

            self.cpu_column.set_factory(Some(&cpu_list_item_factory()));
            self.cpu_column
                .set_sorter(Some(&cpu_sorter(&self.column_view)));

            self.memory_column
                .set_factory(Some(&memory_list_item_factory()));
            self.memory_column
                .set_sorter(Some(&memory_sorter(&self.column_view)));

            self.shared_memory_column
                .set_factory(Some(&shared_memory_list_item_factory()));
            self.shared_memory_column
                .set_sorter(Some(&shared_memory_sorter(&self.column_view)));

            self.drive_column
                .set_factory(Some(&drive_list_item_factory()));
            self.drive_column
                .set_sorter(Some(&drive_sorter(&self.column_view)));

            self.gpu_usage_column
                .set_factory(Some(&gpu_list_item_factory()));
            self.gpu_usage_column
                .set_sorter(Some(&gpu_sorter(&self.column_view)));

            self.gpu_memory_column
                .set_factory(Some(&gpu_memory_list_item_factory()));
            self.gpu_memory_column
                .set_sorter(Some(&gpu_memory_sorter(&self.column_view)));

            let column_view_title = self.column_view.first_child();
            adjust_view_header_alignment(column_view_title);

            self.use_merged_stats
                .set(settings.boolean("apps-page-merged-process-stats"));
            settings.connect_changed(Some("apps-page-merged-process-stats"), {
                let this = self.obj().downgrade();
                move |settings, _| {
                    if let Some(this) = this.upgrade() {
                        this.imp()
                            .use_merged_stats
                            .set(settings.boolean("apps-page-merged-process-stats"));
                    }
                }
            });
        }
    }

    impl WidgetImpl for AppsPage {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl BoxImpl for AppsPage {}
}

glib::wrapper! {
    pub struct AppsPage(ObjectSubclass<imp::AppsPage>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl AppsPage {
    pub fn set_initial_readings(&self, readings: &mut crate::magpie_client::Readings) -> bool {
        let imp = self.imp();

        // Set up the models here since we need access to the main application window
        // which is not yet available in the constructor.
        let base_model = models::base_model(&imp.apps_section, &imp.processes_section);
        let tree_list_model = models::tree_list_model(base_model);
        let filter_list_model = models::filter_list_model(tree_list_model);
        let sort_list_model = models::sort_list_model(filter_list_model, &imp.column_view);

        imp.column_view
            .set_model(Some(&gtk::SingleSelection::new(Some(sort_list_model))));

        columns::update_column_titles(
            &imp.cpu_column,
            &imp.memory_column,
            &imp.drive_column,
            &imp.gpu_usage_column,
            &imp.gpu_memory_column,
            readings,
        );

        let mut process_model_map = HashMap::new();
        let root_process = readings.running_processes.keys().min().unwrap_or(&1);
        if let Some(init) = readings.running_processes.get(root_process) {
            for child in &init.children {
                update_processes(
                    &readings.running_processes,
                    child,
                    &imp.processes_section.children(),
                    &imp.app_icons.borrow(),
                    "application-x-executable-symbolic",
                    imp.use_merged_stats.get(),
                    &mut process_model_map,
                );
            }
        }
        imp.root_process.set(*root_process);

        update_apps(
            &readings.running_apps,
            &readings.running_processes,
            &process_model_map,
            &mut imp.app_icons.borrow_mut(),
            imp.apps_section.children(),
            imp.use_merged_stats.get(),
        );

        true
    }

    pub fn update_readings(&self, readings: &mut crate::magpie_client::Readings) -> bool {
        let imp = self.imp();

        columns::update_column_titles(
            &imp.cpu_column,
            &imp.memory_column,
            &imp.drive_column,
            &imp.gpu_usage_column,
            &imp.gpu_memory_column,
            readings,
        );

        let mut process_model_map = HashMap::new();
        let root_process = imp.root_process.get();
        if let Some(init) = readings.running_processes.get(&root_process) {
            for child in &init.children {
                update_processes(
                    &readings.running_processes,
                    child,
                    &imp.processes_section.children(),
                    &imp.app_icons.borrow(),
                    "application-x-executable-symbolic",
                    imp.use_merged_stats.get(),
                    &mut process_model_map,
                );
            }
        }

        update_apps(
            &readings.running_apps,
            &readings.running_processes,
            &process_model_map,
            &mut imp.app_icons.borrow_mut(),
            imp.apps_section.children(),
            imp.use_merged_stats.get(),
        );

        true
    }

    pub fn get_running_apps(&self) -> HashMap<String, App> {
        HashMap::new()
    }
}

fn update_apps(
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

        if app_map.contains_key(app.app_id().as_str()) {
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

        let row_model = if let Some(index) = list.find_with_equal_func(|obj| {
            let Some(row_model) = obj.downcast_ref::<RowModel>() else {
                return false;
            };
            row_model.app_id() == app.id
        }) {
            unsafe {
                list.item(index)
                    .and_then(|obj| obj.downcast().ok())
                    .unwrap_unchecked()
            }
        } else {
            let row_model = RowModelBuilder::new()
                .content_type(ContentType::App)
                .app_id(&app.id)
                .build();
            list.append(&row_model);
            row_model
        };

        let Some(primary_process) = process_map.get(&primary_pid) else {
            g_critical!(
                "MissionCenter::AppsPage",
                "Failed to find primary PID {} for app {}",
                primary_pid,
                app.name
            );
            continue;
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

        if let Some(process_model) = process_model_map.get(&primary_pid) {
            let app_children = row_model.children();
            let process_children = process_model.children();

            let mut to_remove = Vec::with_capacity(app_children.n_items() as _);
            for i in 0..app_children.n_items() {
                let Some(child) = app_children.item(i) else {
                    continue;
                };

                if process_children.find(&child).is_some() {
                    continue;
                }

                to_remove.push(i);
            }
            for i in to_remove {
                app_children.remove(i);
            }

            for i in 0..process_children.n_items() {
                let Some(child) = process_children.item(i) else {
                    continue;
                };

                if app_children.find(&child).is_some() {
                    continue;
                }
                app_children.append(&child);
            }
        }
    }
}

fn update_processes(
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
        update_processes(
            process_map,
            child,
            row_model.children(),
            app_icons,
            icon,
            use_merged_stats,
            models,
        );
    }

    models.insert(process.pid, row_model);
}
