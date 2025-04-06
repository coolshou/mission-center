/* apps_page/mod.rs
 *
 * Copyright 2024 Romeo Calota
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

use std::cell::Cell;
use std::collections::HashMap;

use adw::prelude::*;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use magpie_types::apps::icon::Icon;

use crate::i18n;
use crate::magpie_client::App;

use columns::{
    cpu_list_item_factory, drive_list_item_factory, gpu_list_item_factory,
    gpu_memory_list_item_factory, memory_list_item_factory, name_list_item_factory,
    pid_list_item_factory, shared_memory_list_item_factory,
};
use row_model::{ContentType, RowModel, RowModelBuilder};

mod columns;
mod model;
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
    }

    impl Default for AppsPage {
        fn default() -> Self {
            Self {
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
                    .build(),
                processes_section: RowModelBuilder::new()
                    .name("Processes")
                    .content_type(ContentType::SectionHeader)
                    .build(),

                root_process: Cell::new(1),
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

            let model = gio::ListStore::new::<RowModel>();
            model.append(&self.apps_section);
            model.append(&self.processes_section);

            let tree_model = gtk::TreeListModel::new(model, false, true, move |model_entry| {
                let Some(row_model) = model_entry.downcast_ref::<RowModel>() else {
                    return None;
                };
                Some(row_model.children().clone().into())
            });

            self.column_view
                .set_model(Some(&gtk::SingleSelection::new(Some(tree_model))));

            self.name_column
                .set_factory(Some(&name_list_item_factory()));
            self.pid_column.set_factory(Some(&pid_list_item_factory()));
            self.cpu_column.set_factory(Some(&cpu_list_item_factory()));
            self.memory_column
                .set_factory(Some(&memory_list_item_factory()));
            self.shared_memory_column
                .set_factory(Some(&shared_memory_list_item_factory()));
            self.drive_column
                .set_factory(Some(&drive_list_item_factory()));
            self.gpu_usage_column
                .set_factory(Some(&gpu_list_item_factory()));
            self.gpu_memory_column
                .set_factory(Some(&gpu_memory_list_item_factory()));

            let mut column_view_title =
                self.column_view.first_child().and_then(|w| w.first_child());
            loop {
                let Some(view_title) = column_view_title.take() else {
                    break;
                };
                column_view_title = view_title.next_sibling();

                let Some(container) = view_title.first_child() else {
                    continue;
                };

                let Some(label) = container
                    .first_child()
                    .and_then(|l| l.downcast::<gtk::Label>().ok())
                else {
                    continue;
                };

                if label.label().starts_with(&i18n("Name")) {
                    continue;
                }

                container.set_halign(gtk::Align::End);
                label.set_halign(gtk::Align::End);
                label.set_justify(gtk::Justification::Right);
            }
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

        columns::update_column_titles(
            &imp.cpu_column,
            &imp.memory_column,
            &imp.drive_column,
            &imp.gpu_usage_column,
            &imp.gpu_memory_column,
            readings,
        );

        for app in readings.running_apps.values() {
            let icon = app
                .icon
                .as_ref()
                .and_then(|i| i.icon.clone())
                .map(|i| match i {
                    Icon::Empty(_) => String::new(),
                    Icon::Path(p) => p,
                    Icon::Id(i) => i,
                    Icon::Data(_) => String::new(),
                })
                .unwrap_or(String::new());
            let row_model = RowModelBuilder::new()
                .content_type(ContentType::App)
                .name(app.name.as_str())
                .pid(app.pids[0])
                .cpu_usage(20.)
                .memory_usage(20000000)
                .shared_memory_usage(200000000)
                .disk_usage(10000000.)
                .gpu_usage(20.)
                .gpu_mem_usage(200000000)
                .icon(icon.as_str())
                .build();
            imp.apps_section.children().append(&row_model)
        }

        let root_process = readings.running_processes.keys().min().unwrap_or(&1);
        update_processes(
            &readings.running_processes,
            root_process,
            &imp.processes_section.children(),
        );
        imp.root_process.set(*root_process);

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

        let root_process = imp.root_process.get();
        update_processes(
            &readings.running_processes,
            &root_process,
            &imp.processes_section.children(),
        );

        true
    }

    pub fn get_running_apps(&self) -> HashMap<String, App> {
        HashMap::new()
    }
}

fn update_processes(
    process_map: &HashMap<u32, magpie_types::processes::Process>,
    pid: &u32,
    list: &gio::ListStore,
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
    let usage_stats = if false
    /* this.use_merge_stats.get() */
    {
        &merged_usage_stats
    } else {
        &process.usage_stats
    };

    row_model.set_name(pretty_name);
    row_model.set_icon("application-x-executable-symbolic");
    row_model.set_pid(process.pid);
    row_model.set_cpu_usage(usage_stats.cpu_usage);
    row_model.set_memory_usage(usage_stats.memory_usage);
    row_model.set_shared_memory_usage(usage_stats.shared_memory_usage);
    row_model.set_disk_usage(usage_stats.disk_usage);
    row_model.set_network_usage(usage_stats.network_usage);
    row_model.set_gpu_usage(usage_stats.gpu_usage);
    row_model.set_gpu_memory_usage(usage_stats.gpu_memory_usage);

    for child in &process.children {
        update_processes(process_map, child, row_model.children())
    }
}
