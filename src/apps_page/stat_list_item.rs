/* apps_page/stat_list_item.rs
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
    subclass::prelude::*,
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::StatListItem)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/stat_list_item.ui")]
    pub struct StatListItem {
        #[template_child]
        label: TemplateChild<gtk::Label>,

        #[property(get = Self::value, set = Self::set_value, type = f32)]
        value: Cell<f32>,
        #[property(get = Self::unit, set = Self::set_unit, type = glib::GString)]
        unit: Cell<glib::GString>,
    }

    impl Default for StatListItem {
        fn default() -> Self {
            Self {
                label: TemplateChild::default(),
                value: Cell::new(0.),
                unit: Cell::new(glib::GString::from("")),
            }
        }
    }

    impl StatListItem {
        fn value(&self) -> f32 {
            self.value.get()
        }

        fn set_value(&self, value: f32) {
            self.value.set(value);

            let this_unit = self.unit();

            let (value, unit) = crate::to_human_readable(value, 1024.);
            self.label.set_text(&format!(
                "{}{}{}{}{}",
                value.round(),
                if this_unit.starts_with("%") { "" } else { " " },
                unit,
                if unit.is_empty() || this_unit.starts_with("bps") {
                    ""
                } else {
                    "i"
                },
                self.unit()
            ));
        }

        fn unit(&self) -> glib::GString {
            unsafe { &*self.unit.as_ptr() }.clone()
        }

        fn set_unit(&self, unit: glib::GString) {
            self.unit.set(unit);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatListItem {
        const NAME: &'static str = "StatListItem";
        type Type = super::StatListItem;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for StatListItem {
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

    impl WidgetImpl for StatListItem {}

    impl BoxImpl for StatListItem {}
}

glib::wrapper! {
    pub struct StatListItem(ObjectSubclass<imp::StatListItem>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl StatListItem {}
