/* apps_page/view_models/view_models/mod.rs
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

use std::cell::{Cell, RefCell};

use gtk::{
    glib,
    glib::{prelude::*, subclass::prelude::*, ParamSpec, Properties, Value},
};

pub use app_model::AppModel;
pub use process_model::ProcessModel;
pub use section_header_model::{SectionHeaderModel, SectionType};

mod app_model;
mod process_model;
mod section_header_model;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ViewModel)]
    pub struct ViewModel {
        #[property(get = Self::name, set = Self::set_name, type = glib::GString)]
        name: Cell<glib::GString>,

        #[property(get)]
        pub content: RefCell<Option<glib::Object>>,
        #[property(get, type = i32)]
        pub content_type: Cell<ContentType>,
    }

    impl Default for ViewModel {
        fn default() -> Self {
            Self {
                content: RefCell::new(None),
                name: Cell::new(glib::GString::default()),

                content_type: Cell::new(ContentType::None),
            }
        }
    }

    impl ViewModel {
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ViewModel {
        const NAME: &'static str = "ViewModel";
        type Type = super::ViewModel;
    }

    impl ObjectImpl for ViewModel {
        fn constructed(&self) {
            self.parent_constructed();
        }

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
}

pub trait ViewModelContent: IsA<glib::Object> + Clone {}

pub enum ContentVariant {
    SectionHeader(SectionHeaderModel),
    App(AppModel),
    Process(ProcessModel),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, glib::Enum)]
#[enum_type(name = "ContentType")]
pub enum ContentType {
    None,
    SectionHeader,
    App,
    Process,
}

glib::wrapper! {
    pub struct ViewModel(ObjectSubclass<imp::ViewModel>);
}

impl ViewModel {
    pub fn new(name: &str, content: ContentVariant) -> Self {
        let this: Self = glib::Object::builder().property("name", name).build();
        this.set_content(content);

        this
    }

    pub fn real_content_type(&self) -> ContentType {
        self.imp().content_type.get()
    }

    pub fn set_content(&self, content: ContentVariant) {
        let this = self.imp();

        this.content.replace(match &content {
            ContentVariant::SectionHeader(c) => {
                this.content_type.set(ContentType::SectionHeader);
                Some(c.clone().upcast::<glib::Object>())
            }
            ContentVariant::App(c) => {
                this.content_type.set(ContentType::App);
                Some(c.clone().upcast())
            }
            ContentVariant::Process(c) => {
                this.content_type.set(ContentType::Process);
                Some(c.clone().upcast())
            }
        });
    }

    pub fn content_model<M: ViewModelContent>(&self) -> Option<M> {
        self.imp()
            .content
            .borrow()
            .as_ref()
            .and_then(|c| c.downcast_ref::<M>())
            .cloned()
    }
}
