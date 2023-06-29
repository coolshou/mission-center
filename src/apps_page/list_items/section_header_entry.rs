/* apps_page/list_items/section_header_entry.rs
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

use gtk::{glib, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/section_header_entry.ui")]
    pub struct SectionHeaderEntry {
        #[template_child]
        pub name: TemplateChild<gtk::Label>,

        pub tree_expander: Cell<gtk::TreeExpander>,
    }

    impl Default for SectionHeaderEntry {
        fn default() -> Self {
            Self {
                name: TemplateChild::default(),
                tree_expander: Cell::new(gtk::TreeExpander::new()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SectionHeaderEntry {
        const NAME: &'static str = "SectionHeaderEntry";
        type Type = super::SectionHeaderEntry;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SectionHeaderEntry {}

    impl WidgetImpl for SectionHeaderEntry {}

    impl BoxImpl for SectionHeaderEntry {}
}

glib::wrapper! {
    pub struct SectionHeaderEntry(ObjectSubclass<imp::SectionHeaderEntry>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl SectionHeaderEntry {
    pub fn new(tree_expander: &gtk::TreeExpander, name: &str) -> Self {
        let this: Self = glib::Object::builder().build();
        this.imp().name.set_text(name);
        this.imp().tree_expander.set(tree_expander.clone());

        tree_expander.set_indent_for_depth(false);
        tree_expander.set_indent_for_icon(false);

        this
    }
}
