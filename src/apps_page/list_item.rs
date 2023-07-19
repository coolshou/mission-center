/* apps_page/list_items/list_item.rs
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

use crate::apps_page::view_model::ContentType;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ListItem)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/list_item.ui")]
    pub struct ListItem {
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub name: TemplateChild<gtk::Label>,

        css_provider: Cell<gtk::CssProvider>,

        #[allow(dead_code)]
        #[property(name = "name", get = Self::name, set = Self::set_name, type = glib::GString)]
        name_property: [u8; 0],
        #[allow(dead_code)]
        #[property(name = "icon", get = Self::icon, set = Self::set_icon, type = glib::GString)]
        icon_property: [u8; 0],
        #[property(set = Self::set_content_type, type = u8)]
        pub content_type: Cell<ContentType>,
        #[property(get, set = Self::set_show_expander)]
        pub show_expander: Cell<bool>,

        #[property(set = Self::set_cpu_usage_percent)]
        pub cpu_usage_percent: Cell<f32>,
        #[property(set = Self::set_memory_usage_percent)]
        pub memory_usage_percent: Cell<f32>,
    }

    impl Default for ListItem {
        fn default() -> Self {
            Self {
                name: TemplateChild::default(),
                icon: TemplateChild::default(),

                css_provider: Cell::new(gtk::CssProvider::new()),

                name_property: [0; 0],
                icon_property: [0; 0],
                content_type: Cell::new(ContentType::SectionHeader),
                show_expander: Cell::new(true),

                cpu_usage_percent: Cell::new(0.0),
                memory_usage_percent: Cell::new(0.0),
            }
        }
    }

    impl ListItem {
        pub fn name(&self) -> glib::GString {
            self.name.text()
        }

        pub fn set_name(&self, name: &str) {
            self.name.set_text(name);
        }

        pub fn icon(&self) -> glib::GString {
            self.icon.icon_name().unwrap_or("".into())
        }

        pub fn set_icon(&self, icon: &str) {
            let display = gtk::gdk::Display::default().unwrap();
            let icon_theme = gtk::IconTheme::for_display(&display);

            if icon_theme.has_icon(icon) {
                self.icon.set_from_icon_name(Some(icon));
            } else {
                self.icon
                    .set_from_icon_name(Some("application-x-executable"));
            }
        }

        fn set_content_type(&self, v: u8) {
            let content_type = match v {
                0 => {
                    self.icon.set_visible(false);
                    self.name.add_css_class("heading");

                    let this = self.obj();
                    this.set_margin_top(6);
                    this.set_margin_bottom(6);

                    ContentType::SectionHeader
                }
                1 => {
                    self.icon.set_visible(true);
                    self.icon.set_margin_end(10);
                    self.icon.set_pixel_size(24);
                    self.name.remove_css_class("heading");

                    let this = self.obj();
                    this.set_margin_top(0);
                    this.set_margin_bottom(0);

                    ContentType::App
                }
                2 => {
                    self.icon.set_visible(true);
                    self.icon.set_margin_end(10);
                    self.icon.set_pixel_size(16);
                    self.name.remove_css_class("heading");

                    let this = self.obj();
                    this.set_margin_top(0);
                    this.set_margin_bottom(0);

                    ContentType::Process
                }
                _ => unreachable!(),
            };

            self.content_type.set(content_type);
        }

        fn set_show_expander(&self, show: bool) {
            use glib::g_critical;

            let parent = self
                .obj()
                .parent()
                .and_then(|p| p.downcast::<gtk::TreeExpander>().ok());
            if parent.is_none() {
                g_critical!(
                    "MissionCenter::AppsPage",
                    "Failed to get parent TreeExpander"
                );
            } else {
                let parent = parent.unwrap();
                parent.set_hide_expander(!show);
            }

            self.show_expander.set(show);
        }

        fn set_cpu_usage_percent(&self, usage_percent: f32) {
            self.cpu_usage_percent.set(usage_percent);

            self.update_css(usage_percent.max(self.memory_usage_percent.get()));
        }

        fn set_memory_usage_percent(&self, usage_percent: f32) {
            self.memory_usage_percent.set(usage_percent);

            self.update_css(usage_percent.max(self.cpu_usage_percent.get()));
        }

        fn update_css(&self, usage_percent: f32) {
            use crate::apps_page::{
                CSS_CELL_USAGE_HIGH, CSS_CELL_USAGE_LOW, CSS_CELL_USAGE_MEDIUM,
            };

            let css_provider = unsafe { &*self.css_provider.as_ptr() };
            if usage_percent >= 90.0 {
                css_provider.load_from_data(CSS_CELL_USAGE_HIGH);
            } else if usage_percent >= 80.0 {
                css_provider.load_from_data(CSS_CELL_USAGE_MEDIUM);
            } else if usage_percent >= 70.0 {
                css_provider.load_from_data(CSS_CELL_USAGE_LOW);
            } else {
                css_provider.load_from_data("");
            }
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
            self.parent_realize();

            if let Some(tree_expander) = self.obj().parent() {
                if let Some(column_view_cell) = tree_expander.parent() {
                    let style_provider = unsafe { &*self.css_provider.as_ptr() };
                    // FIXME: Deprecated in GTK 4.10, removed in GTK 5.0, unclear what the replacement is
                    #[allow(deprecated)]
                    {
                        column_view_cell
                            .style_context()
                            .add_provider(style_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);
                    }
                }
            }
        }
    }

    impl BoxImpl for ListItem {}
}

glib::wrapper! {
    pub struct ListItem(ObjectSubclass<imp::ListItem>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}
