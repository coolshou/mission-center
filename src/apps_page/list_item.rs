/* apps_page/list_item.rs
 *
 * Copyright 2025 Mission Center Developers
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

use std::cell::{Cell, RefCell};

use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::apps_page::row_model::{ContentType, RowModel};

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/list_item.ui")]
    pub struct ListItem {
        #[template_child]
        icon: TemplateChild<gtk::Image>,
        #[template_child]
        name: TemplateChild<gtk::Label>,

        sig_icon: Cell<Option<glib::SignalHandlerId>>,
        sig_name: Cell<Option<glib::SignalHandlerId>>,
        sig_content_type: Cell<Option<glib::SignalHandlerId>>,

        model: Cell<Option<RowModel>>,
        expander: RefCell<Option<gtk::TreeExpander>>,
    }

    impl Default for ListItem {
        fn default() -> Self {
            Self {
                icon: TemplateChild::default(),
                name: TemplateChild::default(),

                sig_icon: Cell::new(None),
                sig_name: Cell::new(None),
                sig_content_type: Cell::new(None),

                model: Cell::new(None),
                expander: RefCell::new(None),
            }
        }
    }

    impl ListItem {
        pub fn bind(&self, model: &RowModel, expander: &gtk::TreeExpander) {
            let this = self.obj().downgrade();

            let sig_icon = model.connect_icon_notify({
                let this = this.clone();
                move |model| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let this = this.imp();
                    this.set_icon(model.icon());
                }
            });
            self.sig_icon.set(Some(sig_icon));
            self.set_icon(model.icon());

            let sig_name = model.connect_name_notify({
                let this = this.clone();
                move |model| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let this = this.imp();
                    this.name.set_label(&model.name());
                }
            });
            self.sig_name.set(Some(sig_name));
            self.name.set_label(&model.name());

            let sig_content_type = model.connect_content_type_notify({
                move |model| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let this = this.imp();
                    this.set_content_type(model.content_type());
                }
            });
            self.sig_content_type.set(Some(sig_content_type));
            self.set_content_type(model.content_type());

            self.model.set(Some(model.clone()));
            *self.expander.borrow_mut() = Some(expander.clone());
        }

        pub fn unbind(&self) {
            self.expander.replace(None);
            let Some(model) = self.model.take() else {
                return;
            };

            if let Some(sig_id) = self.sig_icon.take() {
                model.disconnect(sig_id);
            }

            if let Some(sig_id) = self.sig_name.take() {
                model.disconnect(sig_id);
            }

            if let Some(sig_id) = self.sig_content_type.take() {
                model.disconnect(sig_id);
            }
        }

        fn set_icon(&self, icon: glib::GString) {
            let icon_path = std::path::Path::new(icon.as_str());
            if icon_path.exists() {
                self.icon.set_from_file(Some(&icon_path));
                return;
            }

            let display = gtk::gdk::Display::default().unwrap();
            let icon_theme = gtk::IconTheme::for_display(&display);

            if icon_theme.has_icon(&icon) {
                self.icon.set_icon_name(Some(&icon));
            } else {
                self.icon.set_icon_name(Some("application-x-executable"));
            }
        }

        fn set_content_type(&self, content_type: ContentType) {
            match content_type {
                ContentType::SectionHeader => {
                    self.icon.set_visible(false);
                    self.name.add_css_class("heading");

                    let this = self.obj();
                    this.set_margin_start(6);
                    this.set_margin_top(6);
                    this.set_margin_bottom(6);

                    if let Some(expander) = self.expander.borrow().as_ref() {
                        expander.set_indent_for_icon(false);
                    };
                }
                ContentType::App => {
                    self.icon.set_visible(true);
                    self.icon.set_margin_end(10);
                    self.icon.set_pixel_size(24);
                    self.name.remove_css_class("heading");

                    let this = self.obj();
                    this.set_margin_start(0);
                    this.set_margin_top(0);
                    this.set_margin_bottom(0);

                    let this = this.downgrade();
                    glib::idle_add_local_once(move || {
                        let Some(this) = this.upgrade() else {
                            return;
                        };
                        let _ = this.activate_action("listitem.collapse", None);
                    });

                    if let Some(expander) = self.expander.borrow().as_ref() {
                        expander.set_indent_for_icon(true);
                    };
                }
                ContentType::Process => {
                    self.icon.set_visible(true);
                    self.icon.set_margin_end(10);
                    self.icon.set_pixel_size(16);
                    self.name.remove_css_class("heading");

                    let this = self.obj();
                    this.set_margin_start(0);
                    this.set_margin_top(0);
                    this.set_margin_bottom(0);

                    if let Some(expander) = self.expander.borrow().as_ref() {
                        expander.set_indent_for_icon(true);
                    };
                }
            };
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
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for ListItem {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl BoxImpl for ListItem {}
}

glib::wrapper! {
    pub struct ListItem(ObjectSubclass<imp::ListItem>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl ListItem {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn bind(&self, model: &RowModel, expander: &gtk::TreeExpander) {
        self.imp().bind(model, expander);
    }

    pub fn unbind(&self) {
        self.imp().unbind();
    }
}
