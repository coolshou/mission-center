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

use crate::{apps_page::view_model::ContentType, i18n::*};

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
        #[property(get, set = Self::set_expanded)]
        pub expanded: Cell<bool>,

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
                expanded: Cell::new(false),

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

        fn set_expanded(&self, expanded: bool) {
            self.expanded.set(expanded);

            if !expanded {
                let _ = self.obj().activate_action("listitem.collapse", None);
            }
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
            use crate::apps_page::view_model::ViewModel;
            use glib::g_critical;

            self.parent_realize();

            if let Some(tree_expander) = self.obj().parent() {
                let view_model = match tree_expander
                    .downcast_ref::<gtk::TreeExpander>()
                    .and_then(|te| te.item())
                    .and_then(|model| model.downcast::<ViewModel>().ok())
                {
                    None => {
                        g_critical!(
                            "MissionCenter::AppsPage",
                            "Failed to get ViewModel, cannot show context menu for App/Process"
                        );
                        return;
                    }
                    Some(model) => model,
                };

                // FIXME (Romeo Calota):
                // Compound hackery with the property in the ViewModel. If we're an app we start
                // out collapsed. We set the expanded property to true in the model, so that any
                // action from the user (expand/collapse) will be ignored. We do this with a timeout
                // so that the view has time to refresh at least once with the binding set to true.
                let model_content_type: ContentType =
                    unsafe { core::mem::transmute(view_model.content_type()) };
                if model_content_type == ContentType::App {
                    glib::timeout_add_seconds_local_once(1, {
                        let view_model = view_model.downgrade();
                        move || {
                            if let Some(view_model) = view_model.upgrade() {
                                view_model.set_expanded(true);
                            }
                        }
                    });
                }

                if let Some(column_view_cell) = tree_expander.parent() {
                    let style_provider = unsafe { &*self.css_provider.as_ptr() };
                    // FIXME: Deprecated in GTK 4.10, removed in GTK 5.0, unclear what the replacement is
                    #[allow(deprecated)]
                    {
                        column_view_cell
                            .style_context()
                            .add_provider(style_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);
                    }

                    if let Some(list_item_widget) = column_view_cell.parent() {
                        let right_click_controller = gtk::GestureClick::new();
                        right_click_controller.set_button(3); // Secondary click (AKA right click)

                        list_item_widget.add_controller(right_click_controller.clone());

                        right_click_controller.connect_released(move |_, _, x, y| {
                            use gtk::glib::*;

                            let (stop_label, force_stop_label) = match view_model.content_type() {
                                0 => {
                                    // ContentType::SectionHeader
                                    return;
                                }
                                1 => {
                                    // ContentType::App
                                    (i18n("Stop Application"), i18n("Force Stop Application"))
                                }
                                2 => {
                                    // ContentType::Process
                                    (i18n("Stop Process"), i18n("Force Stop Process"))
                                }
                                _ => unreachable!(),
                            };

                            let apps_page = match list_item_widget.
                                parent()
                                .and_then(|p| p.parent())
                                .and_then(|p| p.parent())
                                .and_then(|p| p.parent())
                                .and_then(|p| p.downcast::<crate::apps_page::AppsPage>().ok()) {
                                Some(ap) => ap,
                                None => {
                                    g_critical!(
                                        "MissionCenter::AppsPage",
                                        "Failed to get AppsPage, cannot show context menu for App/Process"
                                    );
                                    return;
                                }
                            };

                            let mouse_pos = match list_item_widget.compute_point(
                                &apps_page,
                                &gtk::graphene::Point::new(x as _, y as _),
                            ) {
                                None => {
                                    g_critical!(
                                        "MissionCenter::AppsPage",
                                        "Failed to compute_point, cannot context menu will not be anchored to mouse position"
                                    );
                                    (x as f32, y as f32)
                                }
                                Some(p) => {
                                    (p.x(), p.y())
                                }
                            };

                            let context_menu = apps_page.context_menu();

                            let _ = list_item_widget.activate_action(
                                "listitem.select",
                                Some(&Variant::from((true, true))),
                            );

                            let menu = gtk::gio::Menu::new();

                            let mi_stop = gtk::gio::MenuItem::new(Some(&stop_label), None);
                            mi_stop.set_action_and_target_value(Some("apps-page.stop"), Some(&Variant::from(view_model.pid())));
                            let mi_force_stop = gtk::gio::MenuItem::new(Some(&force_stop_label), None);
                            mi_force_stop.set_action_and_target_value(Some("apps-page.force-stop"), Some(&Variant::from(view_model.pid())));

                            menu.append_item(&mi_stop);
                            menu.append_item(&mi_force_stop);

                            context_menu.set_menu_model(Some(&menu));
                            context_menu.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                                mouse_pos.0.round() as i32,
                                mouse_pos.1.round() as i32,
                                1,
                                1,
                            )));
                            context_menu.popup();
                        });
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
