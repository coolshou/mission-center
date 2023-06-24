/* apps_page/list_items/process_entry.rs
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
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/process_entry.ui")]
    pub struct ProcessEntry {
        #[template_child]
        pub name: TemplateChild<gtk::Label>,

        pub tree_expander: Cell<gtk::TreeExpander>,
    }

    impl Default for ProcessEntry {
        fn default() -> Self {
            Self {
                name: TemplateChild::default(),
                tree_expander: Cell::new(gtk::TreeExpander::new()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProcessEntry {
        const NAME: &'static str = "ProcessEntry";
        type Type = super::ProcessEntry;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProcessEntry {}

    impl WidgetImpl for ProcessEntry {}

    impl BoxImpl for ProcessEntry {}
}

glib::wrapper! {
    pub struct ProcessEntry(ObjectSubclass<imp::ProcessEntry>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl ProcessEntry {
    pub fn new(
        tree_expander: &gtk::TreeExpander,
        name: &str,
        model: &crate::apps_page::view_models::ProcessModel,
    ) -> Self {
        use gtk::prelude::*;

        let this: Self = glib::Object::builder().build();
        this.imp().name.set_text(name);
        this.imp().tree_expander.set(tree_expander.clone());

        tree_expander.set_hide_expander(model.children().n_items() == 0);
        model.children().connect_items_changed(
            glib::clone!(@weak tree_expander => move |model, _, _, _| {
                tree_expander.set_hide_expander(model.n_items() == 0);
            }),
        );
        this
    }
}
