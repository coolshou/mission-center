/* neo_services_page/columns/label_cell.rs
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

use std::cell::Cell;

use adw::subclass::prelude::*;
use gtk::{glib, prelude::*};

use crate::neo_services_page::row_model::ServicesRowModel;

mod imp {
    use super::*;
    use adw::prelude::BinExt;

    pub struct ServicesLabelCell {
        pub label: gtk::Label,

        sig_handler: Cell<Option<glib::SignalHandlerId>>,
        model: Cell<glib::WeakRef<ServicesRowModel>>,
    }

    impl Default for ServicesLabelCell {
        fn default() -> Self {
            Self {
                label: gtk::Label::new(None),

                sig_handler: Cell::new(None),
                model: Cell::new(glib::WeakRef::default()),
            }
        }
    }

    impl ServicesLabelCell {
        pub fn bind(
            &self,
            model: &ServicesRowModel,
            property: &'static str,
            handler: impl Fn(&super::ServicesLabelCell, glib::Value) + 'static,
        ) {
            let this = self.obj().downgrade();

            self.model.set(model.downgrade());

            let sig_handler = model.connect_notify_local(Some(property), {
                let this = this.clone();
                move |model, _| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    handler(&this, model.property_value(property));
                }
            });
            self.sig_handler.set(Some(sig_handler));
        }

        pub fn unbind(&self) {
            let Some(model) = self.model.take().upgrade() else {
                return;
            };

            if let Some(sig_id) = self.sig_handler.take() {
                model.disconnect(sig_id);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServicesLabelCell {
        const NAME: &'static str = "ServicesLabelCell";
        type Type = super::ServicesLabelCell;
        type ParentType = adw::Bin;

        fn class_init(_klass: &mut Self::Class) {}

        fn instance_init(_obj: &glib::subclass::InitializingObject<Self>) {}
    }

    impl ObjectImpl for ServicesLabelCell {
        fn constructed(&self) {
            self.parent_constructed();

            self.label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            self.label.set_hexpand(true);
            self.label.set_halign(gtk::Align::End);

            self.obj().set_child(Some(&self.label));
        }
    }

    impl WidgetImpl for ServicesLabelCell {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl BinImpl for ServicesLabelCell {}
}

glib::wrapper! {
    pub struct ServicesLabelCell(ObjectSubclass<imp::ServicesLabelCell>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl ServicesLabelCell {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_label(&self, label: &str) {
        self.imp().label.set_label(label);
    }

    pub fn bind(
        &self,
        model: &ServicesRowModel,
        property: &'static str,
        handler: impl Fn(&Self, glib::Value) + 'static,
    ) {
        self.imp().bind(model, property, handler);
    }

    pub fn unbind(&self) {
        self.imp().unbind();
    }
}
