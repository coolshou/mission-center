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

use std::collections::HashMap;

use adw::prelude::*;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use magpie_types::apps::icon::Icon;

use crate::apps_page::row_model::{ContentType, RowModel, RowModelBuilder};
use crate::list_cell::ListCell;
use crate::magpie_client::App;

mod column_header;
mod list_item;
mod pid_column;
mod row_model;
mod stat_column;

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
        pub disk_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_usage_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_memory_column: TemplateChild<gtk::ColumnViewColumn>,
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
                disk_column: TemplateChild::default(),
                gpu_usage_column: TemplateChild::default(),
                gpu_memory_column: TemplateChild::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppsPage {
        const NAME: &'static str = "AppsPage";
        type Type = super::AppsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
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

            let name_factory = gtk::SignalListItemFactory::new();
            self.name_column.set_factory(Some(&name_factory));

            name_factory.connect_setup(|_, list_item| {
                let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                    return;
                };

                let my_list_item = list_item::ListItem::new();

                let list_cell = ListCell::new("apps-page.show-context-menu");
                list_cell.set_child(Some(&my_list_item));

                let expander = gtk::TreeExpander::new();
                expander.set_child(Some(&list_cell));

                list_item.set_child(Some(&expander));

                unsafe {
                    list_item.set_data("expander", expander);
                    list_item.set_data("list_cell", list_cell);
                    list_item.set_data("list_item", my_list_item);
                }
            });
            name_factory.connect_bind(|_, list_item| {
                let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                    return;
                };

                let Some(row) = list_item
                    .item()
                    .and_then(|item| item.downcast::<gtk::TreeListRow>().ok())
                else {
                    return;
                };

                let expander = unsafe {
                    list_item
                        .data::<gtk::TreeExpander>("expander")
                        .unwrap_unchecked()
                        .as_ref()
                };
                expander.set_list_row(Some(&row));

                let my_list_item = unsafe {
                    list_item
                        .data::<list_item::ListItem>("list_item")
                        .unwrap_unchecked()
                        .as_ref()
                };

                let Some(model) = expander
                    .item()
                    .and_then(|item| item.downcast::<RowModel>().ok())
                else {
                    return;
                };

                my_list_item.bind(&model, expander);
            });
            name_factory.connect_unbind(|_, list_item| {
                let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                    return;
                };

                let expander = unsafe {
                    list_item
                        .data::<gtk::TreeExpander>("expander")
                        .unwrap_unchecked()
                        .as_ref()
                };
                expander.set_list_row(None);

                let my_list_item = unsafe {
                    list_item
                        .data::<list_item::ListItem>("list_item")
                        .unwrap_unchecked()
                        .as_ref()
                };
                my_list_item.unbind();
            });
            name_factory.connect_teardown(|_, list_item| {
                let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                    return;
                };

                unsafe {
                    let _ = list_item.steal_data::<gtk::TreeExpander>("expander");
                    let _ = list_item.steal_data::<ListCell>("list_cell");
                    let _ = list_item.steal_data::<list_item::ListItem>("list_item");
                }
            });
        }
    }

    impl WidgetImpl for AppsPage {
        fn realize(&self) {
            self.parent_realize();
            if let Some(header) = self.column_view.first_child() {
                header.add_css_class("app-list-header");
            }
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
        let model = gio::ListStore::new::<RowModel>();

        let apps_section = RowModelBuilder::new()
            .name("Apps")
            .content_type(ContentType::SectionHeader)
            .build();

        let processes_section = RowModelBuilder::new()
            .name("Processes")
            .content_type(ContentType::SectionHeader)
            .build();

        model.append(&apps_section);
        model.append(&processes_section);

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
                .icon(icon.as_str())
                .build();
            apps_section.children().append(&row_model)
        }

        let tree_model = gtk::TreeListModel::new(model, false, true, move |model_entry| {
            let Some(row_model) = model_entry.downcast_ref::<RowModel>() else {
                return None;
            };
            Some(row_model.children().clone().into())
        });

        self.imp()
            .column_view
            .set_model(Some(&gtk::SingleSelection::new(Some(tree_model))));

        true
    }

    pub fn update_readings(&self, readings: &mut crate::magpie_client::Readings) -> bool {
        true
    }

    pub fn get_running_apps(&self) -> HashMap<String, App> {
        HashMap::new()
    }
}
