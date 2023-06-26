/* apps_page/column_header.rs
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
    glib::{prelude::*, ParamSpec, Properties, Value},
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ColumnHeader)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/column_header.ui")]
    pub struct ColumnHeader {
        #[template_child]
        heading_label: TemplateChild<gtk::Label>,
        #[template_child]
        title_label: TemplateChild<gtk::Label>,

        #[property(get = Self::heading, set = Self::set_heading, type = glib::GString)]
        #[allow(dead_code)]
        heading: [u8; 0],
        #[property(get = Self::title, set = Self::set_title, type = glib::GString)]
        #[allow(dead_code)]
        title: [u8; 0],
        #[property(get = Self::alignment, set = Self::set_alignment, type = i32)]
        #[allow(dead_code)]
        alignment: [u8; 0],
    }

    impl Default for ColumnHeader {
        fn default() -> Self {
            Self {
                heading_label: TemplateChild::default(),
                title_label: TemplateChild::default(),

                heading: [0; 0],
                title: [0; 0],
                alignment: [0; 0],
            }
        }
    }

    impl ColumnHeader {
        pub fn heading(&self) -> glib::GString {
            self.heading_label.label()
        }

        pub fn set_heading(&self, heading: &str) {
            if heading.is_empty() {
                self.heading_label.set_visible(false);
            } else {
                self.heading_label.set_visible(true);
            }
            self.heading_label.set_label(heading);
        }

        pub fn title(&self) -> glib::GString {
            self.title_label.label()
        }

        pub fn set_title(&self, title: &str) {
            self.title_label.set_label(title)
        }

        pub fn alignment(&self) -> i32 {
            use gtk::glib::translate::IntoGlib;

            self.obj().halign().into_glib()
        }

        pub fn set_alignment(&self, alignment: i32) {
            use gtk::glib::translate::FromGlib;

            let alignment = unsafe { gtk::Align::from_glib(alignment) };

            self.obj().set_halign(alignment);
            self.heading_label.set_halign(alignment);
            self.title_label.set_halign(alignment);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ColumnHeader {
        const NAME: &'static str = "ColumnHeader";
        type Type = super::ColumnHeader;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ColumnHeader {
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

    impl WidgetImpl for ColumnHeader {}

    impl BoxImpl for ColumnHeader {}
}

glib::wrapper! {
    pub struct ColumnHeader(ObjectSubclass<imp::ColumnHeader>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl ColumnHeader {
    pub fn new(heading: &str, title: &str, alignment: gtk::Align) -> Self {
        use gtk::glib::translate::IntoGlib;

        let this: Self = unsafe {
            glib::Object::new_internal(
                ColumnHeader::static_type(),
                &mut [
                    ("heading", heading.into()),
                    ("title", title.into()),
                    ("alignment", alignment.into_glib().into()),
                ],
            )
            .downcast()
            .unwrap()
        };
        this
    }
}
