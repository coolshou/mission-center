/* apps_page/view_models
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

use std::cell::Cell;

use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use crate::i18n::*;

mod column_header;
mod list_items;
mod stat_column;
mod view_models;

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
        pub cpu_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub memory_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub disk_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_column: TemplateChild<gtk::ColumnViewColumn>,

        pub column_header_name: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_cpu: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_memory: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_disk: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_gpu: Cell<Option<column_header::ColumnHeader>>,

        pub sort_order: Cell<gtk::SorterChange>,

        pub apps_model: Cell<gio::ListStore>,
        pub processes_root_model: Cell<gio::ListStore>,

        pub apps: Cell<std::collections::HashMap<String, crate::sys_info_v2::App>>,
        pub process_tree: Cell<crate::sys_info_v2::Process>,
    }

    impl Default for AppsPage {
        fn default() -> Self {
            use crate::sys_info_v2::Process;
            use std::collections::HashMap;

            Self {
                column_view: TemplateChild::default(),
                name_column: TemplateChild::default(),
                cpu_column: TemplateChild::default(),
                memory_column: TemplateChild::default(),
                disk_column: TemplateChild::default(),
                gpu_column: TemplateChild::default(),

                column_header_name: Cell::new(None),
                column_header_cpu: Cell::new(None),
                column_header_memory: Cell::new(None),
                column_header_disk: Cell::new(None),
                column_header_gpu: Cell::new(None),

                sort_order: Cell::new(gtk::SorterChange::Inverted),

                apps_model: Cell::new(gio::ListStore::new(view_models::ViewModel::static_type())),
                processes_root_model: Cell::new(gio::ListStore::new(
                    view_models::ViewModel::static_type(),
                )),

                apps: Cell::new(HashMap::new()),
                process_tree: Cell::new(Process::default()),
            }
        }
    }

    impl AppsPage {
        pub fn update_app_model(&self) {
            use std::collections::BTreeSet;
            use view_models::{AppModel, ContentVariant, ViewModel};

            let model = self.apps_model.take();
            let apps = self.apps.take();

            let mut to_remove = BTreeSet::new();
            for i in 0..model.n_items() {
                let current = model.item(i).unwrap().downcast::<ViewModel>();
                if current.is_err() {
                    continue;
                }
                let current = current.unwrap();

                if !apps.contains_key(current.name().as_str()) {
                    to_remove.insert(i);
                }
            }

            for (i, to_remove_i) in to_remove.iter().enumerate() {
                model.remove((*to_remove_i as usize - i) as _);
            }

            for (name, app) in &apps {
                let pos = if model.n_items() > 0 {
                    model.find_with_equal_func(|current| {
                        let current = current.downcast_ref::<ViewModel>();
                        if current.is_none() {
                            return false;
                        }
                        let current = current.unwrap();

                        current.name().as_str() == name
                    })
                } else {
                    None
                };

                if pos.is_none() {
                    let view_model = ViewModel::new(
                        &app.name,
                        ContentVariant::App(AppModel::new(
                            app.icon
                                .as_ref()
                                .unwrap_or(&"application-x-executable".to_string())
                                .as_str(),
                            app.stats.cpu_usage,
                            app.stats.memory_usage,
                            app.stats.disk_usage,
                            app.stats.network_usage,
                            app.stats.gpu_usage,
                        )),
                    );
                    model.append(&view_model);
                } else {
                    let model: gio::ListModel = model.clone().into();
                    let view_model = model
                        .item(pos.unwrap())
                        .unwrap()
                        .downcast::<ViewModel>()
                        .unwrap();

                    let model = view_model.content_model::<AppModel>();
                    assert!(model.is_some());
                    let model = model.unwrap();

                    model.set_cpu_usage(app.stats.cpu_usage);
                    model.set_memory_usage(app.stats.memory_usage);
                    model.set_disk_usage(app.stats.disk_usage);
                    model.set_network_usage(app.stats.network_usage);
                    model.set_gpu_usage(app.stats.gpu_usage);
                }
            }

            self.apps.set(apps);
            self.apps_model.set(model);
        }

        pub fn update_processes_models(&self) {
            use view_models::{ContentVariant, ProcessModel, ViewModel};

            fn update_model(model: gio::ListStore, process: &crate::sys_info_v2::Process) {
                let mut to_remove = Vec::new();
                for i in 0..model.n_items() {
                    let current = model.item(i).unwrap().downcast::<ViewModel>();
                    if current.is_err() {
                        continue;
                    }
                    let current = current.unwrap();
                    let process_model = current.content_model::<ProcessModel>();
                    if process_model.is_none() {
                        continue;
                    }
                    let process_model = process_model.unwrap();

                    if !process.children.contains_key(&(process_model.pid())) {
                        to_remove.push(i);
                    }
                }

                for (i, to_remove_i) in to_remove.iter().enumerate() {
                    let to_remove_i = (*to_remove_i as usize - i) as _;
                    model.remove(to_remove_i);
                }

                for (pid, child) in &process.children {
                    let pos = if model.n_items() > 0 {
                        model.find_with_equal_func(|current| {
                            let current = current.downcast_ref::<ViewModel>();
                            if current.is_none() {
                                return false;
                            }
                            let current = current.unwrap();
                            let process_model = current.content_model::<ProcessModel>();
                            if process_model.is_none() {
                                return false;
                            }
                            process_model.unwrap().pid() == *pid
                        })
                    } else {
                        None
                    };

                    let entry_name = if child.exe != std::path::Path::new("") {
                        child.exe.as_path().file_name().unwrap().to_str().unwrap()
                    } else {
                        child.name.as_str()
                    };

                    let child_model = if pos.is_none() {
                        let view_model = ViewModel::new(
                            entry_name,
                            ContentVariant::Process(ProcessModel::new(
                                *pid,
                                child.stats.cpu_usage,
                                child.stats.memory_usage,
                                child.stats.disk_usage,
                                child.stats.network_usage,
                                child.stats.gpu_usage,
                            )),
                        );

                        model.append(&view_model);
                        let model = view_model.content_model::<ProcessModel>();
                        assert!(model.is_some());
                        model.as_ref().unwrap().children().clone()
                    } else {
                        let view_model = model
                            .item(pos.unwrap())
                            .unwrap()
                            .downcast::<ViewModel>()
                            .unwrap();

                        let model = view_model.content_model::<ProcessModel>();
                        assert!(model.is_some());
                        let model = model.unwrap();

                        model.set_cpu_usage(child.stats.cpu_usage);
                        model.set_memory_usage(child.stats.memory_usage);
                        model.set_disk_usage(child.stats.disk_usage);
                        model.set_network_usage(child.stats.network_usage);
                        model.set_gpu_usage(child.stats.gpu_usage);

                        model.children().clone()
                    };

                    update_model(child_model, child);
                }
            }

            let process_tree = self.process_tree.take();
            let processes_root_model = self.processes_root_model.take();

            update_model(processes_root_model.clone(), &process_tree);

            self.processes_root_model.set(processes_root_model);
            self.process_tree.set(process_tree);
        }

        pub fn column_compare_entries_by(
            &self,
            lhs: &glib::Object,
            rhs: &glib::Object,
            compare_fn: fn(&view_models::ViewModel, &view_models::ViewModel) -> std::cmp::Ordering,
        ) -> std::cmp::Ordering {
            use std::cmp::*;
            use view_models::{ContentType, SectionHeaderModel, SectionType, ViewModel};

            let lhs = lhs.downcast_ref::<ViewModel>();
            if lhs.is_none() {
                return Ordering::Equal.into();
            }
            let lhs = lhs.unwrap();

            let rhs = rhs.downcast_ref::<ViewModel>();
            if rhs.is_none() {
                return Ordering::Equal.into();
            }
            let rhs = rhs.unwrap();

            if lhs.content_type() == ContentType::SectionHeader as i32 {
                let lhs = lhs.content_model::<SectionHeaderModel>();
                if lhs.is_none() {
                    return Ordering::Equal.into();
                }
                let lhs = lhs.unwrap();
                if lhs.section_type() == SectionType::Apps {
                    return Ordering::Greater.into();
                }

                if rhs.content_type() == ContentType::App as i32 {
                    return Ordering::Less.into();
                }

                if rhs.content_type() == ContentType::Process as i32 {
                    return Ordering::Greater.into();
                }
            }

            if rhs.content_type() == ContentType::SectionHeader as i32 {
                let rhs = rhs.content_model::<SectionHeaderModel>();
                if rhs.is_none() {
                    return Ordering::Equal.into();
                }
                let rhs = rhs.unwrap();
                if rhs.section_type() == SectionType::Apps {
                    return Ordering::Less.into();
                }

                if lhs.content_type() == ContentType::App as i32 {
                    return Ordering::Greater.into();
                }

                if lhs.content_type() == ContentType::Process as i32 {
                    return Ordering::Less.into();
                }
            }

            if lhs.content_type() == ContentType::App as i32 {
                if rhs.content_type() == ContentType::App as i32 {
                    return (compare_fn)(lhs, rhs).into();
                }

                if rhs.content_type() == ContentType::Process as i32 {
                    return Ordering::Greater.into();
                }
            }

            if lhs.content_type() == ContentType::Process as i32 {
                if rhs.content_type() == ContentType::Process as i32 {
                    return (compare_fn)(lhs, rhs).into();
                }

                if rhs.content_type() == ContentType::App as i32 {
                    return Ordering::Less.into();
                }
            }

            Ordering::Equal.into()
        }

        pub fn set_up_root_model(&self) -> gio::ListStore {
            use view_models::*;

            let apps_section_header = ViewModel::new(
                &i18n("Apps"),
                ContentVariant::SectionHeader(SectionHeaderModel::new(SectionType::Apps)),
            );

            let processes_section_header = ViewModel::new(
                &i18n("Processes"),
                ContentVariant::SectionHeader(SectionHeaderModel::new(SectionType::Processes)),
            );

            let root_model = gio::ListStore::new(ViewModel::static_type());
            root_model.append(&apps_section_header);
            root_model.append(&processes_section_header);

            root_model
        }

        pub fn set_up_tree_model(&self, model: gio::ListModel) -> gtk::TreeListModel {
            use view_models::*;

            let this = self.obj().downgrade();
            gtk::TreeListModel::new(model, false, true, move |model_entry| {
                let view_model = model_entry.downcast_ref::<ViewModel>();
                if view_model.is_none() {
                    return None;
                }
                let view_model = view_model.unwrap();

                let this = this.upgrade();
                if this.is_none() {
                    return None;
                }
                let this = this.unwrap();
                let this = this.imp();

                let content_type: ContentType =
                    unsafe { core::mem::transmute(view_model.content_type() as u8) };

                if content_type == ContentType::SectionHeader {
                    let model = view_model.content_model::<SectionHeaderModel>().unwrap();

                    if model.section_type() == SectionType::Apps {
                        let apps_model = this.apps_model.take();
                        this.apps_model.set(apps_model.clone());

                        return Some(apps_model.into());
                    }

                    if model.section_type() == SectionType::Processes {
                        let processes_model = this.processes_root_model.take();
                        this.processes_root_model.set(processes_model.clone());

                        return Some(processes_model.into());
                    }

                    return None;
                }

                if content_type == ContentType::Process {
                    let model = view_model.content_model::<ProcessModel>().unwrap();
                    return Some(model.children().clone().into());
                }

                None
            })
        }

        pub fn set_up_filter_model(&self, model: gio::ListModel) -> gtk::FilterListModel {
            use glib::g_critical;
            use view_models::*;

            let window = crate::MissionCenterApplication::default_instance()
                .and_then(|app| app.active_window())
                .and_then(|window| window.downcast::<crate::window::MissionCenterWindow>().ok());
            if window.is_none() {
                g_critical!(
                    "MissionCenter::AppsPage",
                    "Failed to get MissionCenterWindow instance; searching and filtering will not function"
                );
            }
            let window = window.unwrap();

            let window_clone = window.clone();
            let filter = gtk::CustomFilter::new(move |obj| {
                use textdistance::{Algorithm, Levenshtein};

                let window = window_clone.imp();

                if !window.search_button.is_active() {
                    return true;
                }

                if window.search_entry.text().is_empty() {
                    return true;
                }

                let view_model = obj
                    .downcast_ref::<gtk::TreeListRow>()
                    .and_then(|row| row.item())
                    .and_then(|item| item.downcast::<ViewModel>().ok());
                if view_model.is_none() {
                    return false;
                }
                let view_model = view_model.unwrap();
                if view_model.content_type() == ContentType::SectionHeader as i32 {
                    return true;
                }

                let entry_name = view_model.name().to_lowercase();
                let search_query = window.search_entry.text().to_lowercase();

                if entry_name.contains(&search_query) {
                    return true;
                }

                if search_query.contains(&entry_name) {
                    return true;
                }

                let str_distance = Levenshtein::default()
                    .for_str(&entry_name, &search_query)
                    .ndist();
                if str_distance <= 0.6 {
                    return true;
                }

                if view_model.content_type() == ContentType::Process as i32 {
                    fn find_child(model: &ProcessModel, query_string: &str) -> bool {
                        let children = model.children();

                        for i in 0..children.n_items() {
                            let child = children.item(i).unwrap();
                            let view_model = child.downcast_ref::<ViewModel>().unwrap();

                            let child_name = view_model.name().to_lowercase();

                            if query_string.contains(&child_name) {
                                return true;
                            }

                            if child_name.contains(query_string) {
                                return true;
                            }

                            let str_distance = Levenshtein::default()
                                .for_str(&child_name, query_string)
                                .ndist();
                            if str_distance <= 0.6 {
                                return true;
                            }
                        }

                        for i in 0..children.n_items() {
                            let child = children.item(i).unwrap();
                            let view_model = child.downcast_ref::<ViewModel>().unwrap();
                            let process_model = view_model.content_model::<ProcessModel>();
                            if process_model.is_none() {
                                return false;
                            }
                            let process_model = process_model.unwrap();

                            return find_child(&process_model, query_string);
                        }

                        false
                    }

                    let process_model = view_model.content_model::<ProcessModel>();
                    if process_model.is_none() {
                        return false;
                    }
                    let process_model = process_model.unwrap();
                    return find_child(&process_model, &search_query);
                }

                false
            });

            let filter_clone = filter.clone();
            window.imp().search_entry.connect_search_changed(move |_| {
                filter_clone.changed(gtk::FilterChange::Different)
            });

            gtk::FilterListModel::new(Some(model), Some(filter))
        }

        pub fn set_up_sort_model(&self, model: gio::ListModel) -> gtk::SortListModel {
            let this = self.obj().downgrade();
            self.name_column
                .set_sorter(Some(&gtk::CustomSorter::new(move |lhs, rhs| {
                    use std::cmp::Ordering;

                    let this = this.upgrade();
                    if this.is_none() {
                        return Ordering::Equal.into();
                    }
                    let this = this.unwrap();

                    this.imp()
                        .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                            lhs.name().to_lowercase().cmp(&rhs.name().to_lowercase())
                        })
                        .into()
                })));

            let this = self.obj().downgrade();
            self.cpu_column
                .set_sorter(Some(&gtk::CustomSorter::new(move |lhs, rhs| {
                    use std::cmp::Ordering;

                    let this = this.upgrade();
                    if this.is_none() {
                        return Ordering::Equal.into();
                    }
                    let this = this.unwrap();

                    this.imp()
                        .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                            let lhs = lhs
                                .content()
                                .and_then(|content| Some(content.property::<f32>("cpu-usage")))
                                .unwrap_or(0.);
                            let rhs = rhs
                                .content()
                                .and_then(|content| Some(content.property::<f32>("cpu-usage")))
                                .unwrap_or(0.);

                            lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                        })
                        .into()
                })));

            let this = self.obj().downgrade();
            self.memory_column
                .set_sorter(Some(&gtk::CustomSorter::new(move |lhs, rhs| {
                    use std::cmp::Ordering;

                    let this = this.upgrade();
                    if this.is_none() {
                        return Ordering::Equal.into();
                    }
                    let this = this.unwrap();

                    this.imp()
                        .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                            let lhs = lhs
                                .content()
                                .and_then(|content| Some(content.property::<f32>("memory-usage")))
                                .unwrap_or(0.);
                            let rhs = rhs
                                .content()
                                .and_then(|content| Some(content.property::<f32>("memory-usage")))
                                .unwrap_or(0.);

                            lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                        })
                        .into()
                })));

            let this = self.obj().downgrade();
            self.disk_column
                .set_sorter(Some(&gtk::CustomSorter::new(move |lhs, rhs| {
                    use std::cmp::Ordering;

                    let this = this.upgrade();
                    if this.is_none() {
                        return Ordering::Equal.into();
                    }
                    let this = this.unwrap();

                    this.imp()
                        .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                            let lhs = lhs
                                .content()
                                .and_then(|content| Some(content.property::<f32>("disk-usage")))
                                .unwrap_or(0.);
                            let rhs = rhs
                                .content()
                                .and_then(|content| Some(content.property::<f32>("disk-usage")))
                                .unwrap_or(0.);

                            lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                        })
                        .into()
                })));

            let this = self.obj().downgrade();
            self.gpu_column
                .set_sorter(Some(&gtk::CustomSorter::new(move |lhs, rhs| {
                    use std::cmp::Ordering;

                    let this = this.upgrade();
                    if this.is_none() {
                        return Ordering::Equal.into();
                    }
                    let this = this.unwrap();

                    this.imp()
                        .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                            let lhs = lhs
                                .content()
                                .and_then(|content| Some(content.property::<f32>("gpu-usage")))
                                .unwrap_or(0.);
                            let rhs = rhs
                                .content()
                                .and_then(|content| Some(content.property::<f32>("gpu-usage")))
                                .unwrap_or(0.);

                            lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                        })
                        .into()
                })));

            let tree_list_sorter = gtk::TreeListRowSorter::new(self.column_view.sorter());
            gtk::SortListModel::new(Some(model), Some(tree_list_sorter))
        }

        pub fn set_up_view_model(&self) {
            let root_model = self.set_up_root_model();
            let tree_model = self.set_up_tree_model(root_model.into());
            let filter_model = self.set_up_filter_model(tree_model.into());
            let sort_model = self.set_up_sort_model(filter_model.into());

            self.column_view
                .set_model(Some(&gtk::SingleSelection::new(Some(sort_model))));
        }

        pub fn configure_column_header(
            &self,
            column_header: &gtk::Widget,
            name: &str,
            heading: &str,
            align: gtk::Align,
        ) -> (Option<gtk::Widget>, column_header::ColumnHeader) {
            let column_view_box = column_header
                .first_child()
                .unwrap()
                .downcast::<gtk::Box>()
                .unwrap();
            column_view_box.first_child().unwrap().set_visible(false);

            let header = column_header::ColumnHeader::new(heading, name, align);
            column_view_box.append(&header);

            (column_header.next_sibling(), header)
        }

        pub fn update_column_headers(&self, readings: &crate::sys_info_v2::Readings) {
            let column_header_cpu = self.column_header_cpu.take();
            if let Some(column_header_cpu) = &column_header_cpu {
                column_header_cpu.set_heading(format!(
                    "{}%",
                    readings.cpu_info.dynamic_info.utilization_percent.round()
                ));
            }
            self.column_header_cpu.set(column_header_cpu);

            let column_header_memory = self.column_header_memory.take();
            if let Some(column_header_memory) = &column_header_memory {
                let used = readings.mem_info.mem_total - readings.mem_info.mem_available;
                column_header_memory.set_heading(format!(
                    "{}%",
                    ((used * 100) as f32 / readings.mem_info.mem_total as f32).round()
                ));
            }
            self.column_header_memory.set(column_header_memory);

            let column_header_disk = self.column_header_disk.take();
            if let Some(column_header_disk) = &column_header_disk {
                let total_busy_percent = readings
                    .disks
                    .iter()
                    .map(|disk| disk.busy_percent)
                    .sum::<f32>();

                if readings.disks.len() == 0 {
                    column_header_disk.set_heading("0%");
                } else {
                    column_header_disk.set_heading(format!(
                        "{}%",
                        (total_busy_percent / readings.disks.len() as f32).round()
                    ));
                }
            }
            self.column_header_disk.set(column_header_disk);

            let column_header_gpu = self.column_header_gpu.take();
            if let Some(column_header_gpu) = &column_header_gpu {
                let total_busy_percent = readings
                    .gpus
                    .iter()
                    .map(|gpu| gpu.dynamic_info.util_percent as f32)
                    .sum::<f32>();

                if readings.gpus.len() == 0 {
                    column_header_gpu.set_heading("0%");
                } else {
                    column_header_gpu.set_heading(format!(
                        "{}%",
                        (total_busy_percent / readings.gpus.len() as f32).round()
                    ));
                }
            }
            self.column_header_gpu.set(column_header_gpu);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppsPage {
        const NAME: &'static str = "AppsPage";
        type Type = super::AppsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            list_items::ListItem::ensure_type();
            list_items::AppEntry::ensure_type();
            list_items::ProcessEntry::ensure_type();
            list_items::SectionHeaderEntry::ensure_type();

            view_models::ViewModel::ensure_type();
            view_models::AppModel::ensure_type();
            view_models::ProcessModel::ensure_type();
            view_models::SectionHeaderModel::ensure_type();

            column_header::ColumnHeader::ensure_type();
            stat_column::StatColumn::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppsPage {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for AppsPage {
        fn realize(&self) {
            self.parent_realize();

            let list_item_widget = self.column_view.first_child().unwrap();

            let column_view_title = list_item_widget.first_child().unwrap();
            let (column_view_title, column_header_name) = self.configure_column_header(
                &column_view_title,
                &i18n("Name"),
                "",
                gtk::Align::Start,
            );
            let (column_view_title, column_header_cpu) = self.configure_column_header(
                &column_view_title.unwrap(),
                &i18n("CPU"),
                "0%",
                gtk::Align::End,
            );
            let (column_view_title, column_header_memory) = self.configure_column_header(
                &column_view_title.unwrap(),
                &i18n("Memory"),
                "0%",
                gtk::Align::End,
            );
            let (column_view_title, column_header_disk) = self.configure_column_header(
                &column_view_title.unwrap(),
                &i18n("Disk"),
                "0%",
                gtk::Align::End,
            );
            let (_, column_header_gpu) = self.configure_column_header(
                &column_view_title.unwrap(),
                &i18n("GPU"),
                "0%",
                gtk::Align::End,
            );

            self.column_header_name.set(Some(column_header_name));
            self.column_header_cpu.set(Some(column_header_cpu));
            self.column_header_memory.set(Some(column_header_memory));
            self.column_header_disk.set(Some(column_header_disk));
            self.column_header_gpu.set(Some(column_header_gpu));
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
    pub fn set_initial_readings(&self, readings: &mut crate::sys_info_v2::Readings) -> bool {
        use std::collections::HashMap;

        let this = self.imp();

        let mut apps = HashMap::new();
        std::mem::swap(&mut apps, &mut readings.running_apps);
        this.apps.set(apps);

        let mut process_tree = crate::sys_info_v2::Process::default();
        std::mem::swap(&mut process_tree, &mut readings.process_tree);
        this.process_tree.set(process_tree);

        this.set_up_view_model();

        this.update_app_model();
        this.update_processes_models();
        this.update_column_headers(readings);

        true
    }

    pub fn update_readings(&self, readings: &mut crate::sys_info_v2::Readings) -> bool {
        let this = self.imp();

        let mut apps = this.apps.take();
        std::mem::swap(&mut apps, &mut readings.running_apps);
        this.apps.set(apps);

        let mut process_tree = this.process_tree.take();
        std::mem::swap(&mut process_tree, &mut readings.process_tree);
        this.process_tree.set(process_tree);

        this.update_app_model();
        this.update_processes_models();
        this.update_column_headers(readings);

        true
    }
}
