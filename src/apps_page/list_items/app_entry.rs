/* apps_page/list_items/app_entry.rs
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

use gtk::{
    glib,
    glib::{ParamSpec, Properties, Value},
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::AppEntry)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/app_entry.ui")]
    pub struct AppEntry {
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub name: TemplateChild<gtk::Label>,

        #[allow(dead_code)]
        #[property(name = "icon", get = Self::icon, set = Self::set_icon, type = glib::GString)]
        icon_property: [u8; 0],
        #[allow(dead_code)]
        #[property(name = "name", get = Self::name, set = Self::set_name, type = glib::GString)]
        name_property: [u8; 0],
    }

    impl Default for AppEntry {
        fn default() -> Self {
            Self {
                icon: TemplateChild::default(),
                name: TemplateChild::default(),

                icon_property: [0; 0],
                name_property: [0; 0],
            }
        }
    }

    impl AppEntry {
        pub fn name(&self) -> glib::GString {
            self.name.label()
        }

        pub fn set_name(&self, name: &str) {
            self.name.set_text(name)
        }

        pub fn icon(&self) -> glib::GString {
            self.icon.icon_name().unwrap_or("".into())
        }

        pub fn set_icon(&self, icon: &str) {
            self.icon.set_icon_name(Some(icon))
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppEntry {
        const NAME: &'static str = "AppEntry";
        type Type = super::AppEntry;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppEntry {
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

    impl WidgetImpl for AppEntry {}

    impl BoxImpl for AppEntry {}
}

glib::wrapper! {
    pub struct AppEntry(ObjectSubclass<imp::AppEntry>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl AppEntry {
    pub fn new(tree_expander: &gtk::TreeExpander) -> Self {
        let this: Self = glib::Object::builder().build();

        tree_expander.set_hide_expander(true);
        tree_expander.set_indent_for_icon(false);

        this
    }
}
