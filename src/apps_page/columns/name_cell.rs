/* apps_page/columns/name_cell.rs
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

use gdk::pango::EllipsizeMode;
use gtk::glib::g_critical;
use gtk::glib::FileError;
use gtk::{gdk, glib, prelude::*, subclass::prelude::*};

use crate::apps_page::row_model::{ContentType, RowModel};
use crate::widgets::ListCell;

mod icon_cache {
    use super::*;

    use std::collections::HashMap;

    use gdk::gdk_pixbuf::Pixbuf;

    thread_local! {
        static CACHE: RefCell<HashMap<String, Pixbuf>> = RefCell::new(HashMap::new());
    }

    pub fn get(name: &str) -> Option<Pixbuf> {
        CACHE.with(|cache| {
            let cache = cache.borrow();
            cache.get(name).cloned()
        })
    }

    pub fn set(name: glib::GString, pixbuf: Pixbuf) {
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.insert(name.into(), pixbuf);
        })
    }
}

mod imp {
    use super::*;

    pub struct NameCell {
        icon: gtk::Image,
        name: gtk::Label,

        sig_id: Cell<Option<glib::SignalHandlerId>>,
        sig_icon: Cell<Option<glib::SignalHandlerId>>,
        sig_name: Cell<Option<glib::SignalHandlerId>>,
        sig_content_type: Cell<Option<glib::SignalHandlerId>>,
        sig_children_changed: Cell<Option<glib::SignalHandlerId>>,

        model: Cell<Option<RowModel>>,
        expander: RefCell<Option<gtk::TreeExpander>>,
    }

    impl Default for NameCell {
        fn default() -> Self {
            Self {
                icon: gtk::Image::new(),
                name: gtk::Label::new(None),

                sig_id: Cell::new(None),
                sig_icon: Cell::new(None),
                sig_name: Cell::new(None),
                sig_content_type: Cell::new(None),
                sig_children_changed: Cell::new(None),

                model: Cell::new(None),
                expander: RefCell::new(None),
            }
        }
    }

    impl NameCell {
        pub fn bind(&self, model: &RowModel, list_cell: &ListCell, expander: &gtk::TreeExpander) {
            let this = self.obj().downgrade();

            self.model.set(Some(model.clone()));
            *self.expander.borrow_mut() = Some(expander.clone());

            let sig_id = model.connect_id_notify({
                let list_cell = list_cell.downgrade();
                move |model| {
                    let Some(list_cell) = list_cell.upgrade() else {
                        return;
                    };
                    list_cell.set_item_id(model.id())
                }
            });
            self.sig_id.set(Some(sig_id));
            list_cell.set_item_id(model.id());

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
                let this = this.clone();
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

            let sig_children_changed = model.children().connect_items_changed({
                let expander = expander.downgrade();
                move |children, _, _, _| {
                    let Some(expander) = expander.upgrade() else {
                        return;
                    };
                    expander.set_hide_expander(children.n_items() == 0);
                }
            });
            self.sig_children_changed.set(Some(sig_children_changed));
            expander.set_hide_expander(model.children().n_items() == 0);
        }

        pub fn unbind(&self) {
            self.expander.replace(None);
            let Some(model) = self.model.take() else {
                return;
            };

            if let Some(sig_id) = self.sig_id.take() {
                model.disconnect(sig_id);
            }

            if let Some(sig_id) = self.sig_icon.take() {
                model.disconnect(sig_id);
            }

            if let Some(sig_id) = self.sig_name.take() {
                model.disconnect(sig_id);
            }

            if let Some(sig_id) = self.sig_content_type.take() {
                model.disconnect(sig_id);
            }

            if let Some(sig_id) = self.sig_children_changed.take() {
                model.children().disconnect(sig_id);
            }
        }

        #[allow(deprecated)]
        fn set_icon(&self, icon_name: glib::GString) {
            if let Some(pixbuf) = icon_cache::get(icon_name.as_str()) {
                self.icon.set_from_pixbuf(Some(&pixbuf));
                return;
            }

            let icon_path = std::path::Path::new(icon_name.as_str());
            match gdk::gdk_pixbuf::Pixbuf::from_file(&icon_path) {
                Ok(pixbuf) => {
                    self.icon.set_from_pixbuf(Some(&pixbuf));
                    icon_cache::set(icon_name, pixbuf);
                    return;
                }
                Err(e) => {
                    if !e.matches(FileError::Noent) {
                        g_critical!("MissionCenter::AppsPage", "Failed to load icon: {}", e);
                        return;
                    }
                }
            }

            let display = gtk::gdk::Display::default().unwrap();
            let icon_theme = gtk::IconTheme::for_display(&display);

            if icon_theme.has_icon(&icon_name) {
                self.icon.set_icon_name(Some(&icon_name));
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
    impl ObjectSubclass for NameCell {
        const NAME: &'static str = "NameCell";
        type Type = super::NameCell;
        type ParentType = gtk::Box;

        fn class_init(_klass: &mut Self::Class) {}

        fn instance_init(_obj: &glib::subclass::InitializingObject<Self>) {}
    }

    impl ObjectImpl for NameCell {
        fn constructed(&self) {
            self.parent_constructed();

            self.name.set_ellipsize(EllipsizeMode::Middle);

            let _ = self.obj().append(&self.icon);
            let _ = self.obj().append(&self.name);
        }
    }

    impl WidgetImpl for NameCell {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl BoxImpl for NameCell {}
}

glib::wrapper! {
    pub struct NameCell(ObjectSubclass<imp::NameCell>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl NameCell {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn bind(&self, model: &RowModel, list_cell: &ListCell, expander: &gtk::TreeExpander) {
        self.imp().bind(model, list_cell, expander);
    }

    pub fn unbind(&self) {
        self.imp().unbind();
    }
}
