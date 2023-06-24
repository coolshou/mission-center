/* apps_page/view_models/section_header_model.rs
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
    gio, glib,
    glib::{prelude::*, subclass::prelude::*},
};

mod imp {
    use super::*;

    pub struct SectionHeaderModel {
        pub section_type: Cell<SectionType>,
        pub children: Cell<gio::ListStore>,
    }

    impl Default for SectionHeaderModel {
        fn default() -> Self {
            Self {
                section_type: Cell::new(SectionType::Apps),
                children: Cell::new(gio::ListStore::new(super::super::ViewModel::static_type())),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SectionHeaderModel {
        const NAME: &'static str = "SectionHeaderModel";
        type Type = super::SectionHeaderModel;
    }

    impl ObjectImpl for SectionHeaderModel {}
}

glib::wrapper! {
    pub struct SectionHeaderModel(ObjectSubclass<imp::SectionHeaderModel>);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SectionType {
    Apps,
    Processes,
}

impl super::ViewModelContent for SectionHeaderModel {}

impl SectionHeaderModel {
    pub fn new(section_type: SectionType) -> Self {
        let this: Self = glib::Object::builder().build();
        this.imp().section_type.set(section_type);
        this
    }

    pub fn section_type(&self) -> SectionType {
        self.imp().section_type.get()
    }

    pub fn children(&self) -> &gio::ListStore {
        unsafe { &*self.imp().children.as_ptr() }
    }

    pub fn set_children(&self, children: gio::ListStore) {
        self.imp().children.set(children);
    }
}
