/* apps_page/list_item.rs
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

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ListItem)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/list_item.ui")]
    pub struct ListItem {
        #[template_child]
        icon_image: TemplateChild<gtk::Image>,
        #[template_child]
        text_label: TemplateChild<gtk::Label>,

        #[property(get = Self::icon, set = Self::set_icon, type = glib::GString)]
        #[allow(dead_code)]
        icon: [u8; 0],
        #[property(get = Self::icon_size, set = Self::set_icon_size, type = i32)]
        #[allow(dead_code)]
        icon_size: [u8; 0],
        #[property(get = Self::label, set = Self::set_label, type = glib::GString)]
        #[allow(dead_code)]
        label: [u8; 0],
        #[property(get, set)]
        is_section_header: Cell<bool>,
    }

    impl Default for ListItem {
        fn default() -> Self {
            Self {
                icon_image: TemplateChild::default(),
                text_label: TemplateChild::default(),
                icon: [0; 0],
                icon_size: [0; 0],
                label: [0; 0],
                is_section_header: Cell::new(false),
            }
        }
    }

    impl ListItem {
        fn label(&self) -> glib::GString {
            self.text_label.text()
        }

        fn set_label(&self, label: &str) {
            self.text_label.set_text(label);
        }

        fn icon(&self) -> glib::GString {
            self.icon_image.icon_name().unwrap_or_else(|| "".into())
        }

        fn set_icon(&self, icon: &str) {
            self.icon_image.set_icon_name(Some(icon));
        }

        fn icon_size(&self) -> i32 {
            self.icon_image.pixel_size()
        }

        fn set_icon_size(&self, icon_size: i32) {
            self.icon_image.set_pixel_size(icon_size);
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

    impl WidgetImpl for ListItem {
        fn realize(&self) {
            use glib::*;

            self.parent_realize();

            if !self.is_section_header.get() {
                return;
            }

            self.icon_image.set_visible(false);
            self.text_label.add_css_class("heading");

            self.obj().set_margin_top(5);
            self.obj().set_margin_bottom(5);

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
            parent.set_hide_expander(true);
            parent.set_indent_for_depth(false);
            parent.set_indent_for_icon(false);
            let _ = parent.activate_action("listitem.expand", None);
        }
    }

    impl BoxImpl for ListItem {}
}

glib::wrapper! {
    pub struct ListItem(ObjectSubclass<imp::ListItem>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl ListItem {}
