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

            fn update_model(model: &gio::ListStore, process: &crate::sys_info_v2::Process) {
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
                    model.remove((*to_remove_i as usize - i) as _);
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

                    #[allow(unused_assignments)]
                    let mut process_model = None;
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
                        process_model = Some(model.unwrap());

                        process_model.as_ref().unwrap().children()
                    } else {
                        let model: gio::ListModel = model.clone().into();
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

                        process_model = Some(model);
                        process_model.as_ref().unwrap().children()
                    };

                    update_model(child_model, child);
                }
            }

            let process_tree = self.process_tree.take();
            let processes_root_model = self.processes_root_model.take();

            update_model(&processes_root_model, &process_tree);

            self.processes_root_model.set(processes_root_model);
            self.process_tree.set(process_tree);
        }

        pub fn set_up_view_model(&self) {
            use view_models::{ContentVariant, SectionHeaderModel, SectionType, ViewModel};

            let apps_section_header = ViewModel::new(
                &i18n("Apps"),
                ContentVariant::SectionHeader(SectionHeaderModel::new(SectionType::Apps)),
            );

            let processes_section_header = ViewModel::new(
                &i18n("Processes"),
                ContentVariant::SectionHeader(SectionHeaderModel::new(SectionType::Processes)),
            );
            let section_header_model =
                processes_section_header.content_model::<SectionHeaderModel>();
            if section_header_model.is_some() {
                section_header_model
                    .unwrap()
                    .set_children(unsafe { &*self.processes_root_model.as_ptr() }.clone());
            }

            let root_model = gio::ListStore::new(ViewModel::static_type());
            root_model.append(&apps_section_header);
            root_model.append(&processes_section_header);

            let this = self.obj().downgrade();
            let treemodel = gtk::TreeListModel::new(root_model, false, false, move |model_entry| {
                use view_models::{ContentType, ProcessModel};

                let this = this.upgrade();
                if this.is_none() {
                    return None;
                }
                let this = this.unwrap();
                let this = this.imp();

                let view_model = model_entry.downcast_ref::<ViewModel>();
                if view_model.is_none() {
                    return None;
                }
                let view_model = view_model.unwrap();

                match view_model.real_content_type() {
                    ContentType::None => None,
                    ContentType::SectionHeader => {
                        let section_header_model = view_model.content_model::<SectionHeaderModel>();
                        if section_header_model.is_none() {
                            return None;
                        }
                        let section_header_model = section_header_model.unwrap();

                        if section_header_model.section_type() == SectionType::Apps {
                            let model = this.apps_model.take();
                            this.apps_model.set(model.clone());

                            Some(model.into())
                        } else {
                            Some(section_header_model.children().clone().into())
                        }
                    }
                    ContentType::App => None,
                    ContentType::Process => {
                        let process_model = view_model.content_model::<ProcessModel>();
                        if process_model.is_none() {
                            return None;
                        }
                        let process_model = process_model.unwrap();
                        Some(process_model.children().clone().into())
                    }
                }
            });
            let selection = gtk::SingleSelection::new(Some(treemodel));
            self.column_view.set_model(Some(&selection));
        }

        pub fn configure_column_header(
            &self,
            column_header: &gtk::Widget,
            name: &str,
            heading: &str,
            align: gtk::Align,
        ) -> Option<gtk::Widget> {
            let column_view_box = column_header
                .first_child()
                .unwrap()
                .downcast::<gtk::Box>()
                .unwrap();
            column_view_box.first_child().unwrap().set_visible(false);
            column_view_box.prepend(&column_header::ColumnHeader::new(heading, name, align));

            column_header.next_sibling()
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

    impl ObjectImpl for AppsPage {}

    impl WidgetImpl for AppsPage {
        fn realize(&self) {
            self.parent_realize();

            let list_item_widget = self.column_view.first_child().unwrap();

            let mut column_view_title = list_item_widget.first_child().unwrap();
            column_view_title = self
                .configure_column_header(&column_view_title, &i18n("Name"), "", gtk::Align::Start)
                .unwrap();
            column_view_title = self
                .configure_column_header(&column_view_title, &i18n("CPU"), "0%", gtk::Align::End)
                .unwrap();
            column_view_title = self
                .configure_column_header(&column_view_title, &i18n("Memory"), "0%", gtk::Align::End)
                .unwrap();
            column_view_title = self
                .configure_column_header(&column_view_title, &i18n("Disk"), "0%", gtk::Align::End)
                .unwrap();
            self.configure_column_header(&column_view_title, &i18n("GPU"), "0%", gtk::Align::End);
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

        this.update_app_model();

        let mut process_tree = crate::sys_info_v2::Process::default();
        std::mem::swap(&mut process_tree, &mut readings.process_tree);
        this.process_tree.set(process_tree);

        this.update_processes_models();

        this.set_up_view_model();

        true
    }

    pub fn update_readings(&self, readings: &mut crate::sys_info_v2::Readings) -> bool {
        let this = self.imp();

        let mut apps = this.apps.take();
        std::mem::swap(&mut apps, &mut readings.running_apps);
        this.apps.set(apps);

        this.update_app_model();

        let mut process_tree = this.process_tree.take();
        std::mem::swap(&mut process_tree, &mut readings.process_tree);
        this.process_tree.set(process_tree);

        this.update_processes_models();

        true
    }
}
