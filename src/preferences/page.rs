/* preferences/page.rs
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

use adw::{prelude::*, subclass::prelude::*};
use gtk::{gio, glib};

use super::checked_row_widget::CheckedRowWidget;

macro_rules! as_widget_ptr {
    ($e:expr) => {{
        use gtk::glib::translate::*;

        let result: *mut gtk::ffi::GtkWidget = $e.upcast_ref::<gtk::Widget>().to_glib_none().0;

        result
    }};
}

mod imp {
    use adw::glib::g_critical;

    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/preferences/page.ui")]
    pub struct PreferencesPage {
        #[template_child]
        pub update_very_slow: TemplateChild<CheckedRowWidget>,
        #[template_child]
        pub update_slow: TemplateChild<CheckedRowWidget>,
        #[template_child]
        pub update_normal: TemplateChild<CheckedRowWidget>,
        #[template_child]
        pub update_fast: TemplateChild<CheckedRowWidget>,

        pub settings: Cell<Option<gio::Settings>>,

        pub current_speed_selection: Cell<CheckedRowWidget>,
    }

    impl Default for PreferencesPage {
        fn default() -> Self {
            Self {
                update_very_slow: Default::default(),
                update_slow: Default::default(),
                update_normal: Default::default(),
                update_fast: Default::default(),

                settings: Cell::new(None),

                current_speed_selection: Cell::new(CheckedRowWidget::new()),
            }
        }
    }

    impl PreferencesPage {
        pub fn configure_update_speed(&self, checked_row: &CheckedRowWidget) {
            let uvs = as_widget_ptr!(self.update_very_slow) as usize;
            let us = as_widget_ptr!(self.update_slow) as usize;
            let un = as_widget_ptr!(self.update_normal) as usize;
            let uf = as_widget_ptr!(self.update_fast) as usize;

            let old_selection = self.current_speed_selection.replace(checked_row.clone());
            old_selection.set_checkmark_visible(false);

            let settings = self.settings.take();
            if settings.is_none() {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to set up update speed settings, could not load application settings"
                );
                return;
            }
            let settings = settings.unwrap();

            let new_selection = as_widget_ptr!(checked_row) as usize;
            let set_result = if new_selection == uvs {
                checked_row.set_checkmark_visible(true);
                settings.set_int("update-speed", 1)
            } else if new_selection == us {
                checked_row.set_checkmark_visible(true);
                settings.set_int("update-speed", 2)
            } else if new_selection == un {
                checked_row.set_checkmark_visible(true);
                settings.set_int("update-speed", 3)
            } else if new_selection == uf {
                checked_row.set_checkmark_visible(true);
                settings.set_int("update-speed", 4)
            } else {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Unknown update speed selection",
                );
                settings.set_int("update-speed", 3)
            };
            if set_result.is_err() {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to set update speed setting",
                );

                self.update_very_slow.set_checkmark_visible(false);
                self.update_slow.set_checkmark_visible(false);
                self.update_normal.set_checkmark_visible(false);
                self.update_fast.set_checkmark_visible(false);
            }

            self.settings.set(Some(settings));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesPage {
        const NAME: &'static str = "PreferencesPage";
        type Type = super::PreferencesPage;
        type ParentType = adw::PreferencesPage;

        fn class_init(klass: &mut Self::Class) {
            CheckedRowWidget::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesPage {
        fn constructed(&self) {
            use gtk::glib::*;

            self.parent_constructed();

            let update_speed_row = self.update_very_slow.parent().and_then(|p| p.parent());
            if update_speed_row.is_none() {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to set up update speed settings"
                );
            } else {
                let update_speed_row = update_speed_row.unwrap();
                let update_speed_row = update_speed_row.downcast_ref::<gtk::ListBox>().unwrap();
                update_speed_row.connect_row_activated(
                    glib::clone!(@weak self as this => move |_, row| {
                        let row = row.first_child().unwrap();
                        let checked_row = row.downcast_ref::<CheckedRowWidget>().unwrap();
                        this.configure_update_speed(checked_row);
                    }),
                );
            }
        }
    }

    impl WidgetImpl for PreferencesPage {}

    impl PreferencesPageImpl for PreferencesPage {}
}

glib::wrapper! {
    pub struct PreferencesPage(ObjectSubclass<imp::PreferencesPage>)
        @extends adw::PreferencesPage, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PreferencesPage {
    pub fn new(settings: Option<&gio::Settings>) -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(PreferencesPage::static_type(), &mut [])
                .downcast()
                .unwrap()
        };
        this.imp().settings.set(settings.cloned());
        this.set_initial_update_speed();

        this
    }

    fn set_initial_update_speed(&self) {
        use gtk::glib::*;

        let settings = self.imp().settings.take();
        if settings.is_none() {
            g_critical!(
                "MissionCenter::Preferences",
                "Failed to set up update speed settings, could not load application settings"
            );
            return;
        }
        let settings = settings.unwrap();
        let update_speed = settings.int("update-speed");
        let selected_widget = match update_speed {
            1 => &self.imp().update_very_slow,
            2 => &self.imp().update_slow,
            3 => &self.imp().update_normal,
            4 => &self.imp().update_fast,
            _ => {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Unknown update speed setting, defaulting to normal"
                );
                &self.imp().update_normal
            }
        };
        selected_widget.set_checkmark_visible(true);
        self.imp()
            .current_speed_selection
            .set(selected_widget.get());

        self.imp().settings.set(Some(settings));
    }
}
