/* apps_page/model_entry.rs
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
    glib::{prelude::*, subclass::prelude::*, ParamSpec, Properties, Value},
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ModelEntry)]
    pub struct ModelEntry {
        #[property(get = Self::name, set = Self::set_name, type = glib::GString)]
        name: Cell<glib::GString>,
        #[property(get = Self::cpu_usage, set = Self::set_cpu_usage, type = glib::GString)]
        cpu_usage: Cell<glib::GString>,
        #[property(get, set)]
        is_section_header: Cell<bool>,
        #[property(get, set)]
        is_regular_entry: Cell<bool>,
    }

    impl Default for ModelEntry {
        fn default() -> Self {
            Self {
                name: Cell::new(glib::GString::default()),
                cpu_usage: Cell::new(glib::GString::default()),
                is_section_header: Cell::new(false),
                is_regular_entry: Cell::new(true),
            }
        }
    }

    impl ModelEntry {
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

        pub fn cpu_usage(&self) -> glib::GString {
            let cpu_usage = self.cpu_usage.take();
            let result = cpu_usage.clone();
            self.cpu_usage.set(cpu_usage);

            result
        }

        pub fn set_cpu_usage(&self, cpu_usage: &str) {
            let current_cpu_usage = self.cpu_usage.take();
            if current_cpu_usage == cpu_usage {
                self.cpu_usage.set(current_cpu_usage);
                return;
            }

            self.cpu_usage.set(glib::GString::from(cpu_usage));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ModelEntry {
        const NAME: &'static str = "ModelEntry";
        type Type = super::ModelEntry;
    }

    impl ObjectImpl for ModelEntry {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj()
                .as_ref()
                .bind_property("is-section-header", self.obj().as_ref(), "is-regular-entry")
                .flags(
                    glib::BindingFlags::SYNC_CREATE
                        | glib::BindingFlags::BIDIRECTIONAL
                        | glib::BindingFlags::INVERT_BOOLEAN,
                )
                .build();
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

glib::wrapper! {
    pub struct ModelEntry(ObjectSubclass<imp::ModelEntry>);
}

impl ModelEntry {
    pub fn new(name: &str, cpu_usage: u8) -> Self {
        let cpu_usage_str = format!("{}%", cpu_usage);

        let this: Self = unsafe {
            glib::Object::new_internal(
                ModelEntry::static_type(),
                &mut [("name", name.into()), ("cpu-usage", cpu_usage_str.into())],
            )
            .downcast()
            .unwrap()
        };
        this
    }
}
