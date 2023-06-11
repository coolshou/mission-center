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

use gtk::{gio, glib, prelude::*, subclass::prelude::*};

mod column_header;
mod model_entry;

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/page.ui")]
    pub struct AppsPage {
        #[template_child]
        column_view: TemplateChild<gtk::ColumnView>,

        #[template_child]
        name_column: TemplateChild<gtk::ColumnViewColumn>,
    }

    impl Default for AppsPage {
        fn default() -> Self {
            Self {
                column_view: TemplateChild::default(),
                name_column: TemplateChild::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppsPage {
        const NAME: &'static str = "AppsPage";
        type Type = super::AppsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            model_entry::ModelEntry::ensure_type();
            column_header::ColumnHeader::ensure_type();

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

            let model = gio::ListStore::new(ModelEntry::static_type());
            model.append(&ModelEntry::new("Item 1", 2));
            model.append(&ModelEntry::new("Item 2", 3));
            model.append(&ModelEntry::new("Item 3", 1));

            let treemodel = gtk::TreeListModel::new(model, false, false, |_| {
                let model = gio::ListStore::new(ModelEntry::static_type());
                model.append(&ModelEntry::new("Subitem 1", 4));
                model.append(&ModelEntry::new("Subitem 2", 5));
                model.append(&ModelEntry::new("Subitem 3", 0));

                Some(model.into())
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

impl AppsPage {}
