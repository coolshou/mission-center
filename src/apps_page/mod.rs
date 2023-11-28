/* apps_page/mod.rs
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
mod list_item;
mod pid_column;
mod stat_column;
mod view_model;

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
        pub disk_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_column: TemplateChild<gtk::ColumnViewColumn>,

        #[template_child]
        pub context_menu: TemplateChild<gtk::PopoverMenu>,

        pub column_header_name: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_pid: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_cpu: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_memory: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_disk: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_gpu: Cell<Option<column_header::ColumnHeader>>,

        pub tree_list_sorter: Cell<Option<gtk::TreeListRowSorter>>,

        pub apps_model: Cell<gio::ListStore>,
        pub processes_root_model: Cell<gio::ListStore>,

        pub max_cpu_usage: Cell<f32>,
        pub max_memory_usage: Cell<f32>,
        pub max_disk_usage: Cell<f32>,

        pub apps: Cell<std::collections::HashMap<std::sync::Arc<str>, crate::sys_info_v2::App>>,
        pub process_tree: Cell<crate::sys_info_v2::Process>,
    }

    impl Default for AppsPage {
        fn default() -> Self {
            use crate::sys_info_v2::Process;
            use std::collections::HashMap;

            Self {
                column_view: TemplateChild::default(),
                name_column: TemplateChild::default(),
                pid_column: TemplateChild::default(),
                cpu_column: TemplateChild::default(),
                memory_column: TemplateChild::default(),
                disk_column: TemplateChild::default(),
                gpu_column: TemplateChild::default(),

                context_menu: TemplateChild::default(),

                column_header_name: Cell::new(None),
                column_header_pid: Cell::new(None),
                column_header_cpu: Cell::new(None),
                column_header_memory: Cell::new(None),
                column_header_disk: Cell::new(None),
                column_header_gpu: Cell::new(None),

                tree_list_sorter: Cell::new(None),

                apps_model: Cell::new(gio::ListStore::new::<view_model::ViewModel>()),
                processes_root_model: Cell::new(gio::ListStore::new::<view_model::ViewModel>()),

                max_cpu_usage: Cell::new(0.0),
                max_memory_usage: Cell::new(0.0),
                max_disk_usage: Cell::new(0.0),

                apps: Cell::new(HashMap::new()),
                process_tree: Cell::new(Process::default()),
            }
        }
    }

    impl AppsPage {
        fn find_process(
            root_process: &crate::sys_info_v2::Process,
            pid: u32,
        ) -> Option<&crate::sys_info_v2::Process> {
            if root_process.pid == pid {
                return Some(root_process);
            }

            match root_process.children.get(&pid) {
                Some(process) => return Some(process),
                None => {}
            }

            for (_, process) in &root_process.children {
                match Self::find_process(process, pid) {
                    Some(process) => return Some(process),
                    None => {}
                }
            }

            None
        }

        fn configure_actions(&self) {
            use crate::sys_info_v2::Process;
            use gtk::glib::*;

            fn find_pid(
                this: Option<super::AppsPage>,
                pid_and_bool: Option<&glib::Variant>,
            ) -> Option<u32> {
                let this = match this {
                    None => return None,
                    Some(this) => this,
                };

                let (mut pid, is_app) = match pid_and_bool.and_then(|p| p.get::<(u32, bool)>()) {
                    Some(pid) => pid,
                    None => {
                        g_critical!(
                            "MissionCenter::AppsPage",
                            "Invalid data encountered for 'pid' and 'is_app' parameter when stopping process",
                        );
                        return None;
                    }
                };

                // For some wierd reason when stopping the bwarp process for Flatpak apps, they
                // just continue running which makes it confusing for the end-user. So go through
                // the children of the bwrap process and find the first child that is not a bwrap
                // and terminate that instead.
                fn find_first_non_bwrap_child(root: &Process) -> Option<&Process> {
                    for (_, child) in &root.children {
                        if child.name.as_ref() != "bwrap" {
                            return Some(child);
                        }
                    }

                    for (_, child) in &root.children {
                        if let Some(child) = find_first_non_bwrap_child(child) {
                            return Some(child);
                        }
                    }

                    None
                }
                if is_app {
                    if let Some(process) =
                        AppsPage::find_process(unsafe { &*this.imp().process_tree.as_ptr() }, pid)
                    {
                        if process.name.as_ref() == "bwrap" {
                            if let Some(child) = find_first_non_bwrap_child(process) {
                                pid = child.pid;
                            }
                        }
                    }
                }

                Some(pid)
            }

            let this = self.obj();
            let this = this.as_ref();

            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("apps-page", Some(&actions));

            let app = crate::application::MissionCenterApplication::default_instance()
                .expect("Failed to get default MissionCenterApplication instance");

            let action = gio::SimpleAction::new("stop", Some(VariantTy::TUPLE));
            action.connect_activate({
                let app = app.downgrade();
                let this = self.obj().downgrade();
                move |_action, pid| {
                    let pid = match find_pid(this.upgrade(), pid) {
                        None => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to terminate app: Failed to find PID",
                            );
                            return;
                        }
                        Some(pid) => pid,
                    };

                    let app = match app.upgrade() {
                        None => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to terminate app PID '{}': Failed to get app instance",
                                pid,
                            );
                            return;
                        }
                        Some(app) => app,
                    };

                    let sys_info = match app.sys_info() {
                        Ok(si) => si,
                        Err(err) => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to terminate app PID '{}': {}",
                                pid,
                                err,
                            );
                            return;
                        }
                    };

                    sys_info.terminate_process(pid);
                }
            });
            actions.add_action(&action);

            let action = gio::SimpleAction::new("force-stop", Some(VariantTy::TUPLE));
            action.connect_activate({
                let app = app.downgrade();
                let this = self.obj().downgrade();
                move |_action, pid| {
                    let pid = match find_pid(this.upgrade(), pid) {
                        None => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to kill app: Failed to find PID",
                            );
                            return;
                        }
                        Some(pid) => pid,
                    };

                    let app = match app.upgrade() {
                        None => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to kill app PID '{}': Failed to get app instance",
                                pid,
                            );
                            return;
                        }
                        Some(app) => app,
                    };

                    let sys_info = match app.sys_info() {
                        Ok(si) => si,
                        Err(err) => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to kill app PID '{}': {}",
                                pid,
                                err,
                            );
                            return;
                        }
                    };

                    sys_info.kill_process(pid);
                }
            });
            actions.add_action(&action);
        }

        pub fn update_app_model(&self) {
            use crate::glib_clone;
            use gtk::glib::g_critical;
            use std::collections::BTreeSet;
            use view_model::{ContentType, ViewModel, ViewModelBuilder};

            let model = glib_clone!(self.apps_model);
            let apps = self.apps.take();
            let process_tree = self.process_tree.take();

            let mut to_remove = BTreeSet::new();
            for i in 0..model.n_items() {
                let current = model.item(i).unwrap().downcast::<ViewModel>();
                if current.is_err() {
                    continue;
                }
                let current = current.unwrap();

                if !apps.contains_key(current.id().as_str()) {
                    to_remove.insert(i);
                }
            }

            for (i, to_remove_i) in to_remove.iter().enumerate() {
                model.remove((*to_remove_i as usize - i) as _);
            }

            for (app_id, app) in &apps {
                let pos = if model.n_items() > 0 {
                    model.find_with_equal_func(|current| {
                        let current = current.downcast_ref::<ViewModel>();
                        if current.is_none() {
                            return false;
                        }
                        let current = current.unwrap();

                        current.id().as_str() == app_id.as_ref()
                    })
                } else {
                    None
                };

                if app.pids.is_empty() {
                    dbg!(&app);
                }

                // Find the first process that has any children. This is most likely the root
                // of the App's process tree.
                let (primary_process, primary_pid) = {
                    let mut primary_process = None;
                    let mut primary_pid = 0;
                    for (index, pid) in app.pids.iter().enumerate() {
                        if let Some(process) = Self::find_process(&process_tree, *pid) {
                            if process.children.len() > 0 || index == app.pids.len() - 1 {
                                primary_process = Some(process);
                                primary_pid = process.pid;
                                break;
                            }
                        }
                    }

                    (primary_process, primary_pid)
                };
                let view_model = if pos.is_none() {
                    let view_model = ViewModelBuilder::new()
                        .name(app.name.as_ref())
                        .id(app.id.as_ref())
                        .icon(
                            Option::as_ref(&app.icon)
                                .map(|i| i.as_ref())
                                .unwrap_or("application-x-executable"),
                        )
                        .pid(primary_pid)
                        .content_type(ContentType::App)
                        .expanded(false)
                        .cpu_usage(app.usage_stats.cpu_usage)
                        .memory_usage(app.usage_stats.memory_usage)
                        .disk_usage(app.usage_stats.disk_usage)
                        .network_usage(app.usage_stats.network_usage)
                        .gpu_usage(app.usage_stats.gpu_usage)
                        .max_cpu_usage(self.max_cpu_usage.get())
                        .max_memory_usage(self.max_memory_usage.get())
                        .build();
                    model.append(&view_model);

                    view_model
                } else {
                    let model: gio::ListModel = model.clone().into();
                    let view_model = model
                        .item(pos.unwrap())
                        .unwrap()
                        .downcast::<ViewModel>()
                        .unwrap();

                    // The app might have been stopped and restarted between updates, so always
                    // reset the primary PID, and repopulate the list of child processes.
                    view_model.set_pid(primary_pid);
                    view_model.set_cpu_usage(app.usage_stats.cpu_usage);
                    view_model.set_memory_usage(app.usage_stats.memory_usage);
                    view_model.set_disk_usage(app.usage_stats.disk_usage);
                    view_model.set_network_usage(app.usage_stats.network_usage);
                    view_model.set_gpu_usage(app.usage_stats.gpu_usage);

                    view_model
                };

                let children = view_model.children().clone();
                if let Some(process) = primary_process {
                    Self::update_process_model(self, children, process);
                } else {
                    children.remove_all();

                    g_critical!(
                        "MissionCenter::AppsPage",
                        "Failed to find process in process tree, for App {}",
                        app.name.as_ref()
                    );
                }
            }

            self.process_tree.set(process_tree);
            self.apps.set(apps);
        }

        pub fn update_processes_models(&self) {
            use crate::glib_clone;

            let process_tree = self.process_tree.take();
            let processes_root_model = glib_clone!(self.processes_root_model);

            Self::update_process_model(self, processes_root_model.clone(), &process_tree);

            self.process_tree.set(process_tree);
        }

        pub fn column_compare_entries_by(
            &self,
            lhs: &glib::Object,
            rhs: &glib::Object,
            compare_fn: fn(&view_model::ViewModel, &view_model::ViewModel) -> std::cmp::Ordering,
        ) -> std::cmp::Ordering {
            use std::cmp::*;
            use view_model::{ContentType, SectionType, ViewModel};

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

            let sort_order = self
                .column_view
                .sorter()
                .and_downcast_ref::<gtk::ColumnViewSorter>()
                .and_then(|sorter| Some(sorter.primary_sort_order()))
                .unwrap_or(gtk::SortType::Ascending);

            let (ord_less, ord_greater) = if sort_order == gtk::SortType::Ascending {
                (Ordering::Less, Ordering::Greater)
            } else {
                (Ordering::Greater, Ordering::Less)
            };

            if lhs.content_type() == ContentType::SectionHeader as u8 {
                if lhs.section_type() == SectionType::Apps as u8 {
                    return ord_less;
                }

                if lhs.section_type() == SectionType::Processes as u8 {
                    return if rhs.content_type() == ContentType::Process as u8 {
                        ord_less
                    } else {
                        ord_greater
                    };
                }
            }

            if rhs.content_type() == ContentType::SectionHeader as u8 {
                if rhs.section_type() == SectionType::Apps as u8 {
                    return ord_greater;
                }

                if rhs.section_type() == SectionType::Processes as u8 {
                    return if lhs.content_type() == ContentType::Process as u8 {
                        ord_greater
                    } else {
                        ord_less
                    };
                }
            }

            if lhs.content_type() == ContentType::App as u8 {
                if rhs.content_type() == ContentType::App as u8 {
                    return (compare_fn)(lhs, rhs);
                }

                if rhs.content_type() == ContentType::Process as u8 {
                    return ord_less;
                }
            }

            if lhs.content_type() == ContentType::Process as u8 {
                if rhs.content_type() == ContentType::Process as u8 {
                    return (compare_fn)(lhs, rhs);
                }

                if rhs.content_type() == ContentType::App as u8 {
                    return ord_greater;
                }
            }

            Ordering::Equal
        }

        pub fn set_up_root_model(&self) -> gio::ListStore {
            use view_model::{ContentType, SectionType, ViewModel, ViewModelBuilder};

            let apps_section_header = ViewModelBuilder::new()
                .name(&i18n("Apps"))
                .content_type(ContentType::SectionHeader)
                .section_type(SectionType::Apps)
                .show_expander(false)
                .build();

            let processes_section_header = ViewModelBuilder::new()
                .name(&i18n("Processes"))
                .content_type(ContentType::SectionHeader)
                .section_type(SectionType::Processes)
                .show_expander(false)
                .build();

            let root_model = gio::ListStore::new::<ViewModel>();
            root_model.append(&apps_section_header);
            root_model.append(&processes_section_header);

            root_model
        }

        pub fn set_up_tree_model(&self, model: gio::ListModel) -> gtk::TreeListModel {
            use crate::glib_clone;

            use view_model::{ContentType, SectionType, ViewModel};

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

                let content_type = view_model.content_type();

                if content_type == ContentType::SectionHeader as u8 {
                    if view_model.section_type() == SectionType::Apps as u8 {
                        let apps_model = glib_clone!(this.apps_model);
                        return Some(apps_model.into());
                    }

                    if view_model.section_type() == SectionType::Processes as u8 {
                        let processes_model = glib_clone!(this.processes_root_model);
                        return Some(processes_model.into());
                    }

                    return None;
                }

                if content_type == ContentType::Process as u8
                    || content_type == ContentType::App as u8
                {
                    return Some(view_model.children().clone().into());
                }

                None
            })
        }

        pub fn set_up_filter_model(&self, model: gio::ListModel) -> gtk::FilterListModel {
            use glib::g_critical;
            use view_model::{ContentType, ViewModel};

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

            let filter = gtk::CustomFilter::new({
                let window = window.downgrade();
                move |obj| {
                    use textdistance::{Algorithm, Levenshtein};

                    let window = match window.upgrade() {
                        None => return true,
                        Some(w) => w,
                    };
                    let window = window.imp();

                    if !window.search_button.is_active() {
                        return true;
                    }

                    if window.header_search_entry.text().is_empty() {
                        return true;
                    }

                    let view_model = match obj
                        .downcast_ref::<gtk::TreeListRow>()
                        .and_then(|row| row.item())
                        .and_then(|item| item.downcast::<ViewModel>().ok())
                    {
                        None => return false,
                        Some(vm) => vm,
                    };
                    if view_model.content_type() == ContentType::SectionHeader as u8 {
                        return true;
                    }

                    let entry_name = view_model.name().to_lowercase();
                    let search_query = window.header_search_entry.text().to_lowercase();

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

                    false
                }
            });

            window.imp().header_search_entry.connect_search_changed({
                let filter = filter.downgrade();
                move |_| {
                    if let Some(filter) = filter.upgrade() {
                        filter.changed(gtk::FilterChange::Different);
                    }
                }
            });

            gtk::FilterListModel::new(Some(model), Some(filter))
        }

        pub fn set_up_sort_model(&self, model: gio::ListModel) -> gtk::SortListModel {
            let this = self.obj().downgrade();

            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
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
            });
            self.name_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = lhs.property::<crate::sys_info_v2::Pid>("pid");
                        let rhs = rhs.property::<crate::sys_info_v2::Pid>("pid");

                        lhs.cmp(&rhs)
                    })
                    .into()
            });
            self.pid_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = lhs.property::<f32>("cpu-usage");
                        let rhs = rhs.property::<f32>("cpu-usage");

                        lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                    })
                    .into()
            });
            self.cpu_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = lhs.property::<f32>("memory-usage");
                        let rhs = rhs.property::<f32>("memory-usage");

                        lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                    })
                    .into()
            });
            self.memory_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = lhs.property::<f32>("disk-usage");
                        let rhs = rhs.property::<f32>("disk-usage");

                        lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                    })
                    .into()
            });
            self.disk_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = lhs.property::<f32>("gpu-usage");
                        let rhs = rhs.property::<f32>("gpu-usage");

                        lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                    })
                    .into()
            });
            self.gpu_column.set_sorter(Some(&sorter));

            let column_view_sorter = self.column_view.sorter();
            if let Some(column_view_sorter) = column_view_sorter.as_ref() {
                column_view_sorter.connect_changed({
                    let this = self.obj().downgrade();
                    move |sorter, _| {
                        use glib::g_critical;

                        let settings = match crate::MissionCenterApplication::default_instance()
                            .and_then(|app| app.settings())
                        {
                            None => {
                                g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to save column sorting, could not get settings instance from MissionCenterApplication"
                            );
                                return;
                            }
                            Some(s) => s,
                        };

                        let this = match this.upgrade() {
                            None => return,
                            Some(this) => this,
                        };

                        if let Some(sorter) = sorter.downcast_ref::<gtk::ColumnViewSorter>() {
                            let sort_column = sorter.primary_sort_column().as_ref().and_then(|c| Some(c.as_ptr() as usize)).unwrap_or_default();

                            let nc = this.imp().name_column.as_ptr() as usize;
                            let pc = this.imp().pid_column.as_ptr() as usize;
                            let cc = this.imp().cpu_column.as_ptr() as usize;
                            let mc = this.imp().memory_column.as_ptr() as usize;
                            let dc = this.imp().disk_column.as_ptr() as usize;
                            let gc = this.imp().gpu_column.as_ptr() as usize;

                            if let Err(e) = if sort_column == nc {
                                settings.set_enum("apps-page-sorting-column", 0)
                            } else if sort_column == pc {
                                settings.set_enum("apps-page-sorting-column", 1)
                            } else if sort_column == cc {
                                settings.set_enum("apps-page-sorting-column", 2)
                            } else if sort_column == mc {
                                settings.set_enum("apps-page-sorting-column", 3)
                            } else if sort_column == dc {
                                settings.set_enum("apps-page-sorting-column", 4)
                            } else if sort_column == gc {
                                settings.set_enum("apps-page-sorting-column", 5)
                            } else {
                                g_critical!(
                                        "MissionCenter::AppsPage",
                                        "Unknown column sorting encountered"
                                    );
                                Ok(())
                            }
                            {
                                g_critical!(
                                    "MissionCenter::AppsPage",
                                    "Failed to save column sorting: {}", e
                                );
                                return;
                            }

                            let sort_order = sorter.primary_sort_order();
                            if let Err(e) = settings.set_enum("apps-page-sorting-order", match sort_order {
                                gtk::SortType::Ascending => 0,
                                gtk::SortType::Descending => 1,
                                _ => 0
                            }) {
                                g_critical!(
                                    "MissionCenter::AppsPage",
                                    "Failed to save column sorting: {}", e
                                );
                                return;
                            }
                        }
                    }
                });
            }
            let tree_list_sorter = gtk::TreeListRowSorter::new(column_view_sorter);
            self.tree_list_sorter.set(Some(tree_list_sorter.clone()));

            gtk::SortListModel::new(Some(model), Some(tree_list_sorter))
        }

        pub fn set_up_view_model(&self) {
            let root_model = self.set_up_root_model();
            let tree_model = self.set_up_tree_model(root_model.into());
            let filter_model = self.set_up_filter_model(tree_model.into());
            let sort_model = self.set_up_sort_model(filter_model.into());

            self.column_view
                .set_model(Some(&gtk::SingleSelection::new(Some(sort_model))));

            let settings = match crate::MissionCenterApplication::default_instance()
                .and_then(|app| app.settings())
            {
                None => {
                    glib::g_critical!(
                        "MissionCenter::AppsPage",
                        "Failed to get column sorting, could not get settings instance from MissionCenterApplication"
                    );
                    return;
                }
                Some(s) => s,
            };

            let remember_sorting = settings.boolean("apps-page-remember-sorting");
            if remember_sorting {
                let column = settings.enum_("apps-page-sorting-column");
                let order = settings.enum_("apps-page-sorting-order");

                let column = match column {
                    0 => &self.name_column,
                    1 => &self.pid_column,
                    2 => &self.cpu_column,
                    3 => &self.memory_column,
                    4 => &self.disk_column,
                    5 => &self.gpu_column,
                    255 => return,
                    _ => {
                        glib::g_critical!(
                            "MissionCenter::AppsPage",
                            "Unknown column retrieved from settings, sorting by name as a fallback"
                        );
                        &self.name_column
                    }
                };

                let order = match order {
                    0 => gtk::SortType::Ascending,
                    1 => gtk::SortType::Descending,
                    255 => return,
                    _ => {
                        glib::g_critical!(
                            "MissionCenter::AppsPage",
                            "Unknown column sorting order retrieved from settings, sorting in ascending order as a fallback"
                        );
                        gtk::SortType::Ascending
                    }
                };

                self.column_view.sort_by_column(Some(column), order);
            }
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
                    readings
                        .cpu_dynamic_info
                        .overall_utilization_percent
                        .round()
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
                let avg = readings
                    .gpu_dynamic_info
                    .iter()
                    .map(|g| g.util_percent)
                    .sum::<u32>() as f32
                    / readings.gpu_dynamic_info.len() as f32;
                column_header_gpu.set_heading(format!("{:.0}%", avg.round()));
            }
            self.column_header_gpu.set(column_header_gpu);
        }

        fn update_process_model(
            this: &AppsPage,
            model: gio::ListStore,
            process: &crate::sys_info_v2::Process,
        ) {
            use crate::apps_page::view_model::{ContentType, ViewModel, ViewModelBuilder};

            let mut to_remove = Vec::new();
            for i in 0..model.n_items() {
                let current = model.item(i).unwrap().downcast::<ViewModel>();
                if current.is_err() {
                    continue;
                }
                let current = current.unwrap();

                if !process.children.contains_key(&(current.pid())) {
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
                        current.pid() == *pid
                    })
                } else {
                    None
                };

                let entry_name = if !child.exe.as_ref().is_empty() {
                    let entry_name = std::path::Path::new(child.exe.as_ref())
                        .file_name()
                        .map(|name| name.to_str().unwrap_or(child.name.as_ref()))
                        .unwrap_or(child.name.as_ref());
                    if entry_name.starts_with("wine") {
                        if child.cmd.is_empty() {
                            child.name.as_ref()
                        } else {
                            child.cmd[0]
                                .as_ref()
                                .split("\\")
                                .last()
                                .unwrap_or(child.name.as_ref())
                                .split("/")
                                .last()
                                .unwrap_or(child.name.as_ref())
                        }
                    } else {
                        entry_name
                    }
                } else {
                    child.name.as_ref()
                };

                let child_model = if pos.is_none() {
                    let view_model = ViewModelBuilder::new()
                        .name(entry_name)
                        .content_type(ContentType::Process)
                        .pid(*pid)
                        .cpu_usage(child.usage_stats.cpu_usage)
                        .memory_usage(child.usage_stats.memory_usage)
                        .disk_usage(child.usage_stats.disk_usage)
                        .network_usage(child.usage_stats.network_usage)
                        .gpu_usage(child.usage_stats.gpu_usage)
                        .max_cpu_usage(this.max_cpu_usage.get())
                        .max_memory_usage(this.max_memory_usage.get())
                        .build();

                    model.append(&view_model);
                    view_model.children().clone()
                } else {
                    let view_model = model
                        .item(pos.unwrap())
                        .unwrap()
                        .downcast::<ViewModel>()
                        .unwrap();

                    view_model.set_cpu_usage(child.usage_stats.cpu_usage);
                    view_model.set_memory_usage(child.usage_stats.memory_usage);
                    view_model.set_disk_usage(child.usage_stats.disk_usage);
                    view_model.set_network_usage(child.usage_stats.network_usage);
                    view_model.set_gpu_usage(child.usage_stats.gpu_usage);

                    view_model.children().clone()
                };

                Self::update_process_model(this, child_model, child);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppsPage {
        const NAME: &'static str = "AppsPage";
        type Type = super::AppsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            list_item::ListItem::ensure_type();

            view_model::ViewModel::ensure_type();

            column_header::ColumnHeader::ensure_type();
            pid_column::PidColumn::ensure_type();
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

            self.configure_actions();
        }
    }

    impl WidgetImpl for AppsPage {
        fn realize(&self) {
            self.parent_realize();

            let list_item_widget = self.column_view.first_child().unwrap();

            // Remove padding added in GTK 4.12
            list_item_widget.add_css_class("app-list-header");

            let column_view_title = list_item_widget.first_child().unwrap();
            let (column_view_title, column_header_name) = self.configure_column_header(
                &column_view_title,
                &i18n("Name"),
                "",
                gtk::Align::Start,
            );
            let (column_view_title, column_header_pid) = self.configure_column_header(
                &column_view_title.unwrap(),
                &i18n("PID"),
                "",
                gtk::Align::End,
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
            if let Some(column_view_title) = column_view_title {
                let (_, column_header_gpu) = self.configure_column_header(
                    &column_view_title,
                    &i18n("GPU"),
                    "0%",
                    gtk::Align::End,
                );

                self.column_header_gpu.set(Some(column_header_gpu));
            }

            self.column_header_name.set(Some(column_header_name));
            self.column_header_pid.set(Some(column_header_pid));
            self.column_header_cpu.set(Some(column_header_cpu));
            self.column_header_memory.set(Some(column_header_memory));
            self.column_header_disk.set(Some(column_header_disk));
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
    pub fn context_menu(&self) -> &gtk::PopoverMenu {
        &self.imp().context_menu
    }

    pub fn set_initial_readings(&self, readings: &mut crate::sys_info_v2::Readings) -> bool {
        use std::collections::HashMap;

        let this = self.imp();

        if readings.gpu_static_info.is_empty() {
            this.column_view.remove_column(&this.gpu_column);
        }

        this.max_cpu_usage.set(num_cpus::get() as f32 * 100.0);
        this.max_memory_usage
            .set(readings.mem_info.mem_total as f32);

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

        let sorter = this.tree_list_sorter.take();
        if let Some(sorter) = sorter.as_ref() {
            sorter.changed(gtk::SorterChange::Different)
        }
        this.tree_list_sorter.set(sorter);

        true
    }
}
