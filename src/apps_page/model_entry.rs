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

use gtk::glib::g_critical;
use gtk::{
    gio, glib,
    glib::{prelude::*, subclass::prelude::*, ParamSpec, Properties, Value},
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ModelEntry)]
    pub struct ModelEntry {
        #[property(get = Self::name, set = Self::set_name, type = glib::GString)]
        name: Cell<glib::GString>,
        #[property(get = Self::icon, set = Self::set_icon, type = glib::GString)]
        icon: Cell<glib::GString>,
        #[property(get, set)]
        icon_size: Cell<i32>,
        #[property(get = Self::cpu_usage, set = Self::set_cpu_usage, type = glib::GString)]
        cpu_usage: Cell<glib::GString>,

        #[property(get, set)]
        hide_expander: Cell<bool>,
        #[property(get, set)]
        indent: Cell<bool>,
        #[property(get, set)]
        is_section_header: Cell<bool>,

        pub id: Cell<Option<isize>>,

        pub children: Cell<Option<gio::ListStore>>,
    }

    impl Default for ModelEntry {
        fn default() -> Self {
            Self {
                name: Cell::new(glib::GString::default()),
                icon: Cell::new(glib::GString::default()),
                icon_size: Cell::new(16),
                cpu_usage: Cell::new(glib::GString::default()),

                hide_expander: Cell::new(false),
                indent: Cell::new(true),
                is_section_header: Cell::new(false),

                id: Cell::new(None),

                children: Cell::new(None),
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
    pub fn new(name: &str) -> Self {
        let this: Self = glib::Object::builder().property("name", name).build();
        this
    }

    pub fn id(&self) -> Option<isize> {
        self.imp().id.get()
    }

    pub fn set_id(&self, id: isize) {
        self.imp().id.set(Some(id));
    }

    pub fn children(&self) -> Option<&gio::ListStore> {
        unsafe { &*self.imp().children.as_ptr() }.as_ref()
    }

    pub fn set_children(&self, children: gio::ListStore) {
        use glib::*;
        use gtk::prelude::*;

        if unsafe { &*self.imp().children.as_ptr() }.is_some() {
            g_critical!(
                "MissionCenter::AppsPage",
                "Attempted to set children on a ModelEntry that already has children"
            );
            return;
        }

        children.connect_items_changed(glib::clone!(@weak self as this => move |_, _, _, _| {
            if this.is_section_header() {
                return;
            }

            let children = this.imp().children.take();
            if children.is_some() {
                let children = children.unwrap();
                this.set_hide_expander(children.n_items() == 0);
                this.imp().children.set(Some(children));
            }
        }));

        self.set_hide_expander(children.n_items() == 0);
        self.imp().children.set(Some(children));
    }
}
