/* apps_page/list_items/app_entry.rs
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

use gtk::{glib, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/app_entry.ui")]
    pub struct AppEntry {
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub name: TemplateChild<gtk::Label>,
    }

    impl Default for AppEntry {
        fn default() -> Self {
            Self {
                icon: TemplateChild::default(),
                name: TemplateChild::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppEntry {
        const NAME: &'static str = "AppEntry";
        type Type = super::AppEntry;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppEntry {}

    impl WidgetImpl for AppEntry {}

    impl BoxImpl for AppEntry {}
}

glib::wrapper! {
    pub struct AppEntry(ObjectSubclass<imp::AppEntry>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl AppEntry {
    pub fn new(
        tree_expander: &gtk::TreeExpander,
        name: &str,
        model: &crate::apps_page::view_models::AppModel,
    ) -> Self {
        let this: Self = glib::Object::builder().build();

        this.imp().name.set_text(name);
        this.imp().icon.set_icon_name(Some(model.icon().as_str()));

        tree_expander.set_hide_expander(true);
        tree_expander.set_indent_for_icon(false);

        this
    }
}
