/* preferences/toggle_row.rs
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

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    glib,
    glib::{ParamSpec, Properties, Value},
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::SwitchRow)]
    pub struct SwitchRow {
        #[allow(dead_code)]
        #[property(get = Self::active, set = Self::set_active, type = bool)]
        active: [u8; 0],

        switch: std::cell::Cell<gtk::Switch>,
    }

    impl Default for SwitchRow {
        fn default() -> Self {
            let switch = gtk::Switch::new();
            switch.set_valign(gtk::Align::Center);
            switch.set_can_focus(false);

            Self {
                active: Default::default(),
                switch: std::cell::Cell::new(switch),
            }
        }
    }

    impl SwitchRow {
        pub fn active(&self) -> bool {
            unsafe { &*self.switch.as_ptr() }.is_active()
        }

        pub fn set_active(&self, active: bool) {
            unsafe { &*self.switch.as_ptr() }.set_active(active);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SwitchRow {
        const NAME: &'static str = "SwitchRow";
        type Type = super::SwitchRow;
        type ParentType = adw::ActionRow;
    }

    impl ObjectImpl for SwitchRow {
        fn constructed(&self) {
            self.parent_constructed();

            let switch = unsafe { &*self.switch.as_ptr() }.clone();
            let this = self.obj();
            let this = this.as_ref();

            this.set_activatable(true);
            this.add_suffix(&switch);
            this.set_activatable_widget(Some(&switch));

            this.bind_property("action-name", &switch, "action-name")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            this.bind_property("action-target", &switch, "action-target")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            switch.connect_active_notify({
                let this = this.downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        this.notify_active();
                    }
                }
            });
        }

        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for SwitchRow {}

    impl PreferencesRowImpl for SwitchRow {}

    impl ListBoxRowImpl for SwitchRow {}

    impl ActionRowImpl for SwitchRow {}
}

glib::wrapper! {
    pub struct SwitchRow(ObjectSubclass<imp::SwitchRow>)
        @extends adw::ActionRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}
