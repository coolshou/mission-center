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

    #[derive(Properties)]
    #[properties(wrapper_type = super::SmartDialogRow)]
    pub struct SmartDialogRow {
        pub row_entry: OnceCell<gtk::ListBoxRow>,

        pub id: OnceCell<gtk::Label>,
        pub attribute: OnceCell<gtk::Label>,
        pub value: OnceCell<gtk::Label>,
        pub pretty: OnceCell<gtk::Label>,
        pub normalized: OnceCell<gtk::Label>,
        pub threshold: OnceCell<gtk::Label>,
        pub worst: OnceCell<gtk::Label>,
    }

    impl SmartDialogRow {
    }

    impl Default for SmartDialogRow {
        fn default() -> Self {
            Self {
                row_entry: Default::default(),
                id: Default::default(),
                attribute: Default::default(),
                value: Default::default(),
                pretty: Default::default(),
                normalized: Default::default(),
                threshold: Default::default(),
                worst: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SmartDialogRow {
        const NAME: &'static str = "SmartDialogRow";
        type Type = super::SmartDialogRow;
    }

    impl ObjectImpl for SmartDialogRow {
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let sidebar_content_builder = gtk::Builder::from_resource(
                "/io/missioncenter/MissionCenter/ui/performance_page/disk_smart_data_entry.ui",
            );

            let _ = self.row_entry.set(
                sidebar_content_builder
                    .object::<gtk::ListBoxRow>("root")
                    .expect("Could not find `root` object in details pane"),
            );
            let _ = self.id.set(
                sidebar_content_builder
                    .object::<gtk::Label>("id")
                    .expect("Could not find `id` object in details pane"),
            );
            let _ = self.attribute.set(
                sidebar_content_builder
                    .object::<gtk::Label>("attribute")
                    .expect("Could not find `attribute` object in details pane"),
            );
            let _ = self.value.set(
                sidebar_content_builder
                    .object::<gtk::Label>("value")
                    .expect("Could not find `value` object in details pane"),
            );
            let _ = self.pretty.set(
                sidebar_content_builder
                    .object::<gtk::Label>("pretty")
                    .expect("Could not find `pretty` object in details pane"),
            );
            let _ = self.normalized.set(
                sidebar_content_builder
                    .object::<gtk::Label>("normalized")
                    .expect("Could not find `normalized` object in details pane"),
            );
            let _ = self.threshold.set(
                sidebar_content_builder
                    .object::<gtk::Label>("threshold")
                    .expect("Could not find `threshold` object in details pane"),
            );
            let _ = self.worst.set(
                sidebar_content_builder
                    .object::<gtk::Label>("worst")
                    .expect("Could not find `worst` object in details pane"),
            );
        }
    }

    impl WidgetImpl for SmartDialogRow {}
}

pub struct SmartDialogRowBuilder {
    id: u8,
    attribute: glib::GString,
    value: i32,
    units: i32,
    normalized: i32,
    threshold: i32,
    pretty: i64,
    worst: i32,
}

impl SmartDialogRowBuilder {
    pub fn new() -> Self {
        Self {
            id: 0,
            attribute: Default::default(),
            value: 0,
            units: 0,
            normalized: 0,
            threshold: 0,
            pretty: 0,
            worst: 0,
        }
    }

    pub fn id(mut self, id: u8) -> Self {
        self.id = id;
        self
    }

    pub fn attribute(mut self, attribute: &str) -> Self {
        self.attribute = attribute.into();
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

    pub fn build(self) -> SmartDialogRow {
        let this = SmartDialogRow::new();
        {
            let this = this.imp();

            this.id.get().expect("damn").set_label(format!("{}", self.id).as_str());
            this.worst.get().expect("Damn").set_label(format!("{}", self.worst).as_str());

            this.value.get().expect("Damn").set_label(format!("{}", self.value).as_str());

            this.pretty.get().expect("Damn").set_label(&match self.units {
                0 => i18n("N/A"),
                2 => crate::to_human_readable_time(self.pretty as u64 / 1000),
                3 => i18n_f("{} sectors", &[&format!("{}", self.pretty)]),
                4 => i18n_f("{} °C", &[&format!("{}", (self.pretty - 273150) / 1000)]),
                _ => format!("{}", self.pretty),
            });

            // this.pretty.get().expect("Damn").set_label(format!("{}", self.pretty).as_str());
            this.threshold.get().expect("Damn").set_label(format!("{}", self.threshold).as_str());
            this.normalized.get().expect("Damn").set_label(format!("{}", self.normalized).as_str());

            this.attribute.get().expect("Damn").set_label(self.attribute.as_str());
        }

        this
    }
}

glib::wrapper! {
    pub struct SmartDialogRow(ObjectSubclass<imp::SmartDialogRow>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl SmartDialogRow {
    pub fn new() -> Self {
        use gtk::prelude::*;

        let this: Self = glib::Object::builder().build();

        this
    }
}
