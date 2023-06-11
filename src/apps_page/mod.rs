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

mod column_header;
mod list_item;
mod model_entry;

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/page.ui")]
    pub struct AppsPage {
        #[template_child]
        column_view: TemplateChild<gtk::ColumnView>,

        pub apps: Cell<Vec<crate::sys_info_v2::App>>,
        pub process_tree: Cell<crate::sys_info_v2::Process>,
    }

    impl Default for AppsPage {
        fn default() -> Self {
            Self {
                column_view: TemplateChild::default(),
                apps: Cell::new(Vec::new()),
                process_tree: Cell::new(crate::sys_info_v2::Process::default()),
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
            use model_entry::ModelEntry;

            self.parent_realize();

            let apps_section_header = ModelEntry::new("Apps");
            apps_section_header.set_is_section_header(true);

            let processes_section_header = ModelEntry::new("Processes");
            processes_section_header.set_is_section_header(true);

            let model = gio::ListStore::new(ModelEntry::static_type());
            model.append(&apps_section_header);
            model.append(&processes_section_header);

            let this = self.obj().downgrade();
            let treemodel = gtk::TreeListModel::new(model, false, false, move |model_entry| {
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

                if !model_entry.is_section_header() {
                    return None;
                }

                if model_entry.name() == "Apps" {
                    let model = gio::ListStore::new(ModelEntry::static_type());

                    let apps = this.apps.take();
                    for app in &apps {
                        let model_entry = ModelEntry::new(&app.name);
                        model.append(&model_entry);
                    }
                    this.apps.set(apps);

                    Some(model.into())
                } else if model_entry.name() == "Processes" {
                    let model = gio::ListStore::new(ModelEntry::static_type());

                    let process_tree = this.process_tree.take();
                    for process in &process_tree.children {
                        let model_entry = ModelEntry::new(&process.name);
                        model.append(&model_entry);
                    }
                    this.process_tree.set(process_tree);

                    Some(model.into())
                } else {
                    None
                }
            });
            let selection = gtk::SingleSelection::new(Some(treemodel));
            self.column_view.set_model(Some(&selection));

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
                "Name",
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
                "CPU",
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
        let this = self.imp();

        let mut apps = vec![];
        std::mem::swap(&mut apps, &mut readings.running_apps);
        this.apps.set(apps);

        let mut process_tree = crate::sys_info_v2::Process::default();
        std::mem::swap(&mut process_tree, &mut readings.process_tree);
        this.process_tree.set(process_tree);

        true
    }
}
