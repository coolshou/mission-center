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

use crate::apps_page::view_models::{AppModel, ContentType, ProcessModel};

mod app_entry;
mod process_entry;
mod section_header_entry;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ListItem)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/list_item.ui")]
    pub struct ListItem {
        #[property(get = Self::name, set = Self::set_name, type = glib::GString)]
        name: Cell<glib::GString>,
        #[property(set = Self::set_content)]
        pub content: Cell<u64>,
        #[property(set = Self::set_content_type, type = i32)]
        pub content_type: Cell<ContentType>,
    }

    impl Default for ListItem {
        fn default() -> Self {
            Self {
                name: Cell::new(glib::GString::default()),
                content: Cell::new(0),
                content_type: Cell::new(ContentType::None),
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

        fn set_content(&self, content: u64) {
            self.content.set(content);

            if content == 0 {
                return;
            }

            self.update_child(content);
        }

        fn set_content_type(&self, v: i32) {
            let content_type = match v {
                0 => ContentType::None,
                1 => ContentType::SectionHeader,
                2 => ContentType::App,
                3 => ContentType::Process,
                _ => unreachable!(),
            };

            self.content_type.set(content_type);

            if self.content.get() == 0 {
                return;
            }

            self.update_child(self.content.get());
        }
    }

    impl ListItem {
        fn update_child(&self, content: u64) {
            use glib::{translate::*, *};

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

            let name = unsafe { &*self.name.as_ptr() }.as_str();
            let internal_widget: gtk::Widget = match self.content_type.get() {
                ContentType::None => return,
                ContentType::SectionHeader => SectionHeaderEntry::new(&parent, name).upcast(),
                ContentType::App => {
                    let model: AppModel = unsafe { from_glib_none(content as *mut _) };
                    AppEntry::new(&parent, name, &model).upcast()
                }
                ContentType::Process => {
                    let model: ProcessModel = unsafe { from_glib_none(content as *mut _) };
                    ProcessEntry::new(&parent, name, &model).upcast()
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

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
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

    impl WidgetImpl for ListItem {}

    impl BoxImpl for ListItem {}
}

glib::wrapper! {
    pub struct ListItem(ObjectSubclass<imp::ListItem>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl ListItem {}
