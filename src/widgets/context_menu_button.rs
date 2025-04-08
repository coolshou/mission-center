/* widgets/context_menu_button.rs
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

use std::cell::RefCell;
use std::rc::Rc;

use gtk::{
    glib::{self, gobject_ffi, prelude::*, ParamSpec, Properties, Value, Variant},
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ContextMenuButton)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/widgets/context_menu_button.ui")]
    pub struct ContextMenuButton {
        #[template_child]
        pub button: TemplateChild<gtk::Button>,

        #[property(set = Self::set_item_id, type = glib::GString)]
        item_id: RefCell<Rc<str>>,
        #[property(set = Self::set_action_name, type = glib::GString)]
        action_name: RefCell<Rc<str>>,
    }

    impl Default for ContextMenuButton {
        fn default() -> Self {
            let empty_string = Rc::<str>::from("");
            Self {
                button: TemplateChild::default(),
                item_id: RefCell::new(empty_string.clone()),
                action_name: RefCell::new(empty_string),
            }
        }
    }

    impl ContextMenuButton {
        pub fn set_item_id(&self, id: &str) {
            *self.item_id.borrow_mut() = Rc::<str>::from(id);

            if self.action_name.borrow().is_empty() {
                return;
            }

            self.update_action();
        }

        fn set_action_name(&self, action_name: &str) {
            *self.action_name.borrow_mut() = Rc::<str>::from(action_name);

            if self.item_id.borrow().is_empty() {
                return;
            }

            self.update_action();
        }
    }

    impl ContextMenuButton {
        fn update_action(&self) {
            // Create a weak reference to the object and pass it to the action
            let weak_self = unsafe {
                let weak_ref = Box::leak(Box::<gobject_ffi::GWeakRef>::new(core::mem::zeroed()));
                gobject_ffi::g_weak_ref_init(weak_ref, self.obj().as_ptr() as *mut _);

                weak_ref as *mut _ as u64
            };

            self.button
                .set_action_name(Some(self.action_name.borrow().as_ref()));
            self.button.set_action_target_value(Some(&Variant::from((
                self.item_id.borrow().as_ref().to_owned(),
                weak_self,
                -1_f64,
                -1_f64,
            ))));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContextMenuButton {
        const NAME: &'static str = "ContextMenuButton";
        type Type = super::ContextMenuButton;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContextMenuButton {
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

    impl WidgetImpl for ContextMenuButton {}

    impl BoxImpl for ContextMenuButton {}
}

glib::wrapper! {
    pub struct ContextMenuButton(ObjectSubclass<imp::ContextMenuButton>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}
