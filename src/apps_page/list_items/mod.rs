/* apps_page/list_items/list_items/mod.rs
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

use gtk::{
    glib,
    glib::prelude::*,
    glib::{ParamSpec, Properties, Value},
    prelude::*,
    subclass::prelude::*,
};

pub use app_entry::AppEntry;
pub use process_entry::ProcessEntry;
pub use section_header_entry::SectionHeaderEntry;

use crate::apps_page::view_model::ContentType;

mod app_entry;
mod process_entry;
mod section_header_entry;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ListItem)]
    pub struct ListItem {
        #[property(get = Self::name, set = Self::set_name, type = glib::GString)]
        name: Cell<glib::GString>,
        #[property(get = Self::icon, set = Self::set_icon, type = glib::GString)]
        icon: Cell<glib::GString>,
        #[property(set = Self::set_content_type, type = u8)]
        pub content_type: Cell<ContentType>,
    }

    impl Default for ListItem {
        fn default() -> Self {
            Self {
                name: Cell::new(glib::GString::default()),
                icon: Cell::new(glib::GString::default()),
                content_type: Cell::new(ContentType::SectionHeader),
            }
        }
    }

    impl ListItem {
        pub fn name(&self) -> glib::GString {
            let name = self.name.take();
            let result = name.clone();
            self.name.set(name);

            result
        }

        pub fn set_name(&self, name: &str) {
            let current_name = self.name.take();
            if current_name == name {
                self.name.set(current_name);
                return;
            }

            self.name.set(glib::GString::from(name));
        }

        pub fn icon(&self) -> glib::GString {
            let icon = self.icon.take();
            let result = icon.clone();
            self.icon.set(icon);

            result
        }

        pub fn set_icon(&self, icon: &str) {
            let current_icon = self.icon.take();
            if current_icon == icon {
                self.icon.set(current_icon);
                return;
            }

            self.icon.set(glib::GString::from(icon));
        }

        fn set_content_type(&self, v: u8) {
            let content_type = match v {
                0 => ContentType::SectionHeader,
                1 => ContentType::App,
                2 => ContentType::Process,
                _ => unreachable!(),
            };

            self.content_type.set(content_type);
        }
    }

    impl ListItem {
        fn update_child(&self) {
            use glib::*;

            let name = unsafe { &*self.name.as_ptr() }.as_str();
            if name.is_empty() {
                return;
            }

            let parent = self
                .obj()
                .parent()
                .and_then(|p| p.downcast::<gtk::TreeExpander>().ok());
            if parent.is_none() {
                g_critical!(
                    "MissionCenter::AppsPage",
                    "Failed to get parent TreeExpander"
                );
                return;
            }
            let parent = parent.unwrap();

            let internal_widget: gtk::Widget = match self.content_type.get() {
                ContentType::SectionHeader => SectionHeaderEntry::new(&parent, name).upcast(),
                ContentType::App => {
                    let app_entry = AppEntry::new(&parent);

                    self.obj()
                        .bind_property("name", &app_entry, "name")
                        .flags(BindingFlags::SYNC_CREATE)
                        .build();

                    self.obj()
                        .bind_property("icon", &app_entry, "icon")
                        .flags(BindingFlags::SYNC_CREATE)
                        .build();

                    app_entry.upcast()
                }
                ContentType::Process => {
                    dbg!("Creating process", name);
                    let process_entry = ProcessEntry::new(&parent);

                    self.obj()
                        .bind_property("name", &process_entry, "name")
                        .flags(BindingFlags::SYNC_CREATE)
                        .build();

                    process_entry.upcast()
                }
            };
            self.obj().first_child().map(|c| self.obj().remove(&c));
            self.obj().append(&internal_widget);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ListItem {
        const NAME: &'static str = "ListItem";
        type Type = super::ListItem;
        type ParentType = gtk::Box;

        fn class_init(_klass: &mut Self::Class) {}

        fn instance_init(_obj: &glib::subclass::InitializingObject<Self>) {}
    }

    impl ObjectImpl for ListItem {
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for ListItem {
        fn realize(&self) {
            self.parent_realize();
            self.update_child();
        }
    }

    impl BoxImpl for ListItem {}
}

glib::wrapper! {
    pub struct ListItem(ObjectSubclass<imp::ListItem>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl ListItem {}
