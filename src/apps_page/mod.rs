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
mod model_entry;

const APPS_SECTION_HEADER_ID: isize = isize::MIN;
const PROCESSES_SECTION_HEADER_ID: isize = isize::MIN + 1;

const APP_BLACKLIST: &[&'static str] = &["fish", "Fish"];

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

                apps_model: Cell::new(gio::ListStore::new(model_entry::ModelEntry::static_type())),
                processes_root_model: Cell::new(gio::ListStore::new(
                    model_entry::ModelEntry::static_type(),
                )),

                apps: Cell::new(HashMap::new()),
                process_tree: Cell::new(Process::default()),
            }
        }
    }

    impl AppsPage {
        pub fn update_app_model(&self) {
            use model_entry::ModelEntry;
            use std::collections::BTreeSet;

            let model = self.apps_model.take();
            let apps = self.apps.take();

            let mut to_remove = BTreeSet::new();
            for i in 0..model.n_items() {
                let current = model.item(i).unwrap().downcast::<ModelEntry>();
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
                if APP_BLACKLIST.contains(&name.as_str()) {
                    continue;
                }

                let pos = if model.n_items() > 0 {
                    model.find_with_equal_func(|current| {
                        let current = current.downcast_ref::<ModelEntry>();
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
                    let model_entry = ModelEntry::new(&app.name);
                    model_entry.set_hide_expander(true);
                    model_entry.set_indent(false);
                    model_entry.set_icon(
                        app.icon
                            .as_ref()
                            .unwrap_or(&"application-x-executable".to_string())
                            .as_str(),
                    );
                    model_entry.set_icon_size(24);
                    model.append(&model_entry);
                }
            }

            self.apps.set(apps);
            self.apps_model.set(model);
        }

        pub fn update_processes_models(&self) {
            use model_entry::ModelEntry;

            fn update_model(model: Option<&gio::ListStore>, process: &crate::sys_info_v2::Process) {
                if model.is_none() {
                    return;
                }
                let model = model.unwrap();

                let mut to_remove = Vec::new();
                for i in 0..model.n_items() {
                    let current = model.item(i).unwrap().downcast::<ModelEntry>();
                    if current.is_err() {
                        continue;
                    }
                    let current = current.unwrap();

                    if !process
                        .children
                        .contains_key(&(current.id().unwrap_or(0) as _))
                    {
                        to_remove.push(i);
                    }
                }

                for (i, to_remove_i) in to_remove.iter().enumerate() {
                    model.remove((*to_remove_i as usize - i) as _);
                }

                for (pid, child) in &process.children {
                    let pos = if model.n_items() > 0 {
                        model.find_with_equal_func(|current| {
                            let current = current.downcast_ref::<ModelEntry>();
                            if current.is_none() {
                                return false;
                            }
                            let current = current.unwrap();

                            current.id().unwrap_or(0) == *pid as isize
                        })
                    } else {
                        None
                    };

                    let mut model_entry = ModelEntry::new(&child.name);
                    let child_model = if pos.is_none() {
                        model_entry.set_id(*pid as _);
                        model_entry.set_children(gio::ListStore::new(ModelEntry::static_type()));
                        model_entry.set_icon("application-x-executable-symbolic");
                        model.append(&model_entry);

                        model_entry.children()
                    } else {
                        let model: gio::ListModel = model.clone().into();
                        model_entry = model
                            .item(pos.unwrap())
                            .unwrap()
                            .downcast::<ModelEntry>()
                            .unwrap();

                        model_entry.children()
                    };

                    update_model(child_model, child);
                }
            }

            let process_tree = self.process_tree.take();
            let processes_root_model = self.processes_root_model.take();

            update_model(Some(&processes_root_model), &process_tree);

            self.processes_root_model.set(processes_root_model);
            self.process_tree.set(process_tree);
        }

        pub fn set_up_view_model(&self) {
            use model_entry::ModelEntry;

            let apps_section_header = ModelEntry::new(&i18n("Apps"));
            apps_section_header.set_id(APPS_SECTION_HEADER_ID);
            apps_section_header.set_is_section_header(true);

            let processes_section_header = ModelEntry::new(&i18n("Processes"));
            processes_section_header.set_id(PROCESSES_SECTION_HEADER_ID);
            processes_section_header.set_is_section_header(true);
            processes_section_header
                .set_children(unsafe { &*self.processes_root_model.as_ptr() }.clone());

            let root_model = gio::ListStore::new(ModelEntry::static_type());
            root_model.append(&apps_section_header);
            root_model.append(&processes_section_header);

            let this = self.obj().downgrade();
            let treemodel = gtk::TreeListModel::new(root_model, false, false, move |model_entry| {
                let this = this.upgrade();
                if this.is_none() {
                    return None;
                }
                let this = this.unwrap();
                let this = this.imp();

                let model_entry = model_entry.downcast_ref::<ModelEntry>();
                if model_entry.is_none() {
                    return None;
                }
                let model_entry = model_entry.unwrap();

                if model_entry.id() == Some(APPS_SECTION_HEADER_ID) {
                    this.update_app_model();

                    let model = this.apps_model.take();
                    this.apps_model.set(model.clone());

                    Some(model.into())
                } else if model_entry.id() == Some(PROCESSES_SECTION_HEADER_ID) {
                    model_entry.children().map(|model| model.clone().into())
                } else {
                    model_entry.children().map(|model| model.clone().into())
                }
            });
            let selection = gtk::SingleSelection::new(Some(treemodel));
            self.column_view.set_model(Some(&selection));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppsPage {
        const NAME: &'static str = "AppsPage";
        type Type = super::AppsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            list_item::ListItem::ensure_type();
            column_header::ColumnHeader::ensure_type();
            model_entry::ModelEntry::ensure_type();

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
            let column_view_title = list_item_widget.first_child().unwrap();
            let column_view_box = column_view_title
                .first_child()
                .unwrap()
                .downcast::<gtk::Box>()
                .unwrap();

            column_view_box.first_child().unwrap().set_visible(false);
            column_view_box.prepend(&column_header::ColumnHeader::new(
                "",
                &i18n("Name"),
                gtk::Align::Start,
            ));

            let column_view_title = column_view_title.next_sibling().unwrap();
            let column_view_box = column_view_title
                .first_child()
                .unwrap()
                .downcast::<gtk::Box>()
                .unwrap();
            column_view_box.first_child().unwrap().set_visible(false);
            column_view_box.prepend(&column_header::ColumnHeader::new(
                "34%",
                &i18n("CPU"),
                gtk::Align::End,
            ));
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
