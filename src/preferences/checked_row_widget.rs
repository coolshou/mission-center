/* preferences/checked_row_widget.rs
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

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gio, glib,
    glib::{ParamSpec, Properties, Value},
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::CheckedRowWidget)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/preferences/checked_row.ui")]
    pub struct CheckedRowWidget {
        #[template_child]
        title_label: TemplateChild<gtk::Label>,
        #[template_child]
        subtitle_label: TemplateChild<gtk::Label>,
        #[template_child]
        checkmark: TemplateChild<gtk::Image>,

        #[allow(dead_code)]
        #[property(get = Self::title, set = Self::set_title, type = glib::GString)]
        title: [u8; 0],
        #[allow(dead_code)]
        #[property(get = Self::subtitle, set = Self::set_subtitle, type = glib::GString)]
        subtitle: [u8; 0],
        #[allow(dead_code)]
        #[property(get = Self::checkmark_visible, set = Self::set_checkmark_visible, type = bool)]
        checkmark_visible: [u8; 0],
    }

    impl Default for CheckedRowWidget {
        fn default() -> Self {
            Self {
                title_label: Default::default(),
                subtitle_label: Default::default(),
                checkmark: Default::default(),
                title: Default::default(),
                subtitle: Default::default(),
                checkmark_visible: Default::default(),
            }
        }
    }

    impl CheckedRowWidget {
        pub fn title(&self) -> glib::GString {
            self.title_label.label()
        }

        pub fn set_title(&self, title: &str) {
            self.title_label.set_label(title);
        }

        pub fn subtitle(&self) -> glib::GString {
            self.subtitle_label.label()
        }

        pub fn set_subtitle(&self, title: &str) {
            self.subtitle_label.set_label(title);
        }

        pub fn checkmark_visible(&self) -> bool {
            self.checkmark.is_visible()
        }

        pub fn set_checkmark_visible(&self, visible: bool) {
            self.checkmark.set_visible(visible);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CheckedRowWidget {
        const NAME: &'static str = "CheckedRow";
        type Type = super::CheckedRowWidget;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CheckedRowWidget {
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for CheckedRowWidget {}

    impl BoxImpl for CheckedRowWidget {}
}

glib::wrapper! {
    pub struct CheckedRowWidget(ObjectSubclass<imp::CheckedRowWidget>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl CheckedRowWidget {
    pub fn new() -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(CheckedRowWidget::static_type(), &mut [])
                .downcast()
                .unwrap()
        };
        this
    }
}
