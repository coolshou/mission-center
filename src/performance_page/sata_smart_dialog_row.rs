/* apps_page/view_model.rs
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

use std::cell::Cell;

use gtk::{
    gio, glib,
    glib::{prelude::*, subclass::prelude::*, ParamSpec, Properties, Value},
};
use crate::i18n::{i18n, i18n_f};

mod imp {
    use std::cell::OnceCell;
    use adw::gio::ListStore;
    use gtk::prelude::{ButtonExt, WidgetExt};
    use gtk::subclass::prelude::WidgetImpl;
    use gtk::TemplateChild;
    use super::*;
    use gtk::subclass::widget::WidgetClassExt;
    use crate::app;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::SmartDialogRow)]
    pub struct SmartDialogRow {
        #[property(get, set)]
        pub smart_id: Cell<u8>,
        #[property(get, set)]
        pub attribute: OnceCell<String>,
        #[property(get, set)]
        pub value: OnceCell<String>,
        #[property(get, set)]
        pub normalized: Cell<i32>,
        #[property(get, set)]
        pub threshold: Cell<i32>,
        #[property(get, set)]
        pub worst: Cell<i32>,
        #[property(get, set)]
        pub typee: OnceCell<String>,
        #[property(get, set)]
        pub updates: OnceCell<String>,
        #[property(get, set)]
        pub assessment: OnceCell<String>,
    }

    impl SmartDialogRow {
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SmartDialogRow {
        const NAME: &'static str = "SmartDialogRow";
        type ParentType = glib::Object;
        type Type = super::SmartDialogRow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SmartDialogRow {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for SmartDialogRow {}
}

pub struct SmartDialogRowBuilder {
    id: u8,
    attribute: String,
    value: i32,
    units: i32,
    threshold: i32,
    pretty: i64,
    worst: i32,
    flags: u16,
}

// todo remove the builder, this isnt java
impl SmartDialogRowBuilder {
    pub fn new() -> Self {
        Self {
            id: 0,
            attribute: Default::default(),
            value: 0,
            units: 0,
            threshold: 0,
            pretty: 0,
            worst: 0,
            flags: 0,
        }
    }

    pub fn id(mut self, id: u8) -> Self {
        self.id = id;
        self
    }

    pub fn attribute(mut self, attribute: String) -> Self {
        self.attribute = attribute;
        self
    }

    pub fn value(mut self, value: i32, units: i32) -> Self {
        self.value = value;
        self.units = units;
        self
    }

    pub fn pretty(mut self, pretty: i64) -> Self {
        self.pretty = pretty;
        self
    }

    pub fn threshold(mut self, threshold: i32) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn worst(mut self, worst: i32) -> Self {
        self.worst = worst;
        self
    }

    pub fn flags(mut self, flags: u16) -> Self {
        self.flags = flags;
        self
    }

    pub fn build(self) -> SmartDialogRow {
        SmartDialogRow::new(self.id, self.attribute, self.value, self.pretty, self.units, self.threshold, self.worst, &match self.flags & 0b1 { 1 => i18n("Pre-Fail"), _ => i18n("Old-Age")}, &match self.flags & 0b10 >> 1 { 0 => i18n("Online"), _ => i18n("Offline")}, "IDK LMAO")
    }
}

glib::wrapper! {
    pub struct SmartDialogRow(ObjectSubclass<imp::SmartDialogRow>);
        // @extends gtk::Box, gtk::Widget,
        // @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl SmartDialogRow {
    pub fn new(id: u8, attribute: String, value: i32, pretty: i64, units: i32, threshold: i32, worst: i32, typee: &str, updates: &str, assessment: &str) -> Self {
        glib::Object::builder()
            .property("smart_id", id)
            .property("attribute", attribute)
            .property("value", &match units {
                0 => i18n("N/A"),
                2 => crate::to_human_readable_time(pretty as u64 / 1000),
                3 => i18n_f("{} sectors", &[&format!("{}", pretty)]),
                4 => i18n_f("{} °C", &[&format!("{}", (pretty - 273150) / 1000)]),
                _ => format!("{}", pretty),
            })
            .property("normalized", value)
            .property("threshold", threshold)
            .property("worst", worst)
            .property("typee", typee)
            .property("updates", updates)
            .property("assessment", assessment)
            .build()
    }
}
