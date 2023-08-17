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

use crate::{i18n::*, preferences::checked_row_widget::CheckedRowWidget};

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/preferences/page.ui")]
    pub struct PreferencesPage {
        #[template_child]
        pub update_speed_setting: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub update_very_slow: TemplateChild<CheckedRowWidget>,
        #[template_child]
        pub update_slow: TemplateChild<CheckedRowWidget>,
        #[template_child]
        pub update_normal: TemplateChild<CheckedRowWidget>,
        #[template_child]
        pub update_fast: TemplateChild<CheckedRowWidget>,

        #[template_child]
        pub merged_process_stats: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub mps_no: TemplateChild<CheckedRowWidget>,
        #[template_child]
        pub mps_yes: TemplateChild<CheckedRowWidget>,

        pub settings: Cell<Option<gio::Settings>>,

        pub current_speed_selection: Cell<CheckedRowWidget>,
        pub current_mps_selection: Cell<CheckedRowWidget>,
    }

    impl Default for PreferencesPage {
        fn default() -> Self {
            Self {
                update_speed_setting: Default::default(),
                update_very_slow: Default::default(),
                update_slow: Default::default(),
                update_normal: Default::default(),
                update_fast: Default::default(),

                merged_process_stats: Default::default(),
                mps_no: Default::default(),
                mps_yes: Default::default(),

                settings: Cell::new(None),

                current_speed_selection: Cell::new(CheckedRowWidget::new()),
                current_mps_selection: Cell::new(CheckedRowWidget::new()),
            }
        }
    }

    impl PreferencesPage {
        pub fn configure_update_speed(&self, checked_row: &CheckedRowWidget) {
            use glib::g_critical;

            let uvs = self.update_very_slow.as_ptr() as usize;
            let us = self.update_slow.as_ptr() as usize;
            let un = self.update_normal.as_ptr() as usize;
            let uf = self.update_fast.as_ptr() as usize;

            let old_selection = self.current_speed_selection.replace(checked_row.clone());
            old_selection.set_checkmark_visible(false);

            let settings = self.settings.take();
            if settings.is_none() {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to configure update speed settings, could not load application settings"
                );
                return;
            }
            let settings = settings.unwrap();

            let new_selection = checked_row.as_ptr() as usize;
            let set_result = if new_selection == uvs {
                self.update_speed_setting.set_subtitle(&i18n("Very Slow"));
                checked_row.set_checkmark_visible(true);
                settings.set_int("update-speed", 4)
            } else if new_selection == us {
                self.update_speed_setting.set_subtitle(&i18n("Slow"));
                checked_row.set_checkmark_visible(true);
                settings.set_int("update-speed", 3)
            } else if new_selection == un {
                self.update_speed_setting.set_subtitle(&i18n("Normal"));
                checked_row.set_checkmark_visible(true);
                settings.set_int("update-speed", 2)
            } else if new_selection == uf {
                self.update_speed_setting.set_subtitle(&i18n("Fast"));
                checked_row.set_checkmark_visible(true);
                settings.set_int("update-speed", 1)
            } else {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Unknown update speed selection",
                );

                self.update_speed_setting.set_subtitle(&i18n("Normal"));
                settings.set_int("update-speed", 2)
            };
            if set_result.is_err() {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to set update speed setting",
                );

                self.update_speed_setting.set_subtitle("");

                self.update_very_slow.set_checkmark_visible(false);
                self.update_slow.set_checkmark_visible(false);
                self.update_normal.set_checkmark_visible(false);
                self.update_fast.set_checkmark_visible(false);
            }

            self.settings.set(Some(settings));
        }

        pub fn configure_merged_process_stats(&self, checked_row: &CheckedRowWidget) {
            use glib::g_critical;

            let no = self.mps_no.as_ptr() as usize;
            let yes = self.mps_yes.as_ptr() as usize;

            let old_selection = self.current_mps_selection.replace(checked_row.clone());
            old_selection.set_checkmark_visible(false);

            let settings = self.settings.take();
            if settings.is_none() {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to configure merged process stats settings, could not load application settings"
                );
                return;
            }
            let settings = settings.unwrap();

            let new_selection = checked_row.as_ptr() as usize;
            let set_result = if new_selection == no {
                self.merged_process_stats.set_subtitle(&i18n("No"));
                checked_row.set_checkmark_visible(true);
                settings.set_boolean("apps-page-merged-process-stats", false)
            } else if new_selection == yes {
                self.merged_process_stats.set_subtitle(&i18n("Yes"));
                checked_row.set_checkmark_visible(true);
                settings.set_boolean("apps-page-merged-process-stats", true)
            } else {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Unknown merge process stats selection",
                );

                self.merged_process_stats.set_subtitle(&i18n("No"));
                settings.set_boolean("apps-page-merged-process-stats", false)
            };
            if set_result.is_err() {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to set merge process stats setting",
                );

                self.merged_process_stats.set_subtitle("");

                self.mps_no.set_checkmark_visible(false);
                self.mps_yes.set_checkmark_visible(false);
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

            let merge_process_stats_row = self.mps_no.parent().and_then(|p| p.parent());
            if merge_process_stats_row.is_none() {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to set up merge process stats settings"
                );
            } else {
                let merge_process_stats_row = merge_process_stats_row.unwrap();
                let merge_process_stats_row = merge_process_stats_row
                    .downcast_ref::<gtk::ListBox>()
                    .unwrap();
                merge_process_stats_row.connect_row_activated(
                    glib::clone!(@weak self as this => move |_, row| {
                        let row = row.first_child().unwrap();
                        let checked_row = row.downcast_ref::<CheckedRowWidget>().unwrap();
                        this.configure_merged_process_stats(checked_row);
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
        this.set_initial_merge_process_stats();

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
        let this = self.imp();
        let selected_widget = match update_speed {
            1 => {
                this.update_speed_setting.set_subtitle(&i18n("Fast"));
                &this.update_fast
            }
            2 => {
                this.update_speed_setting.set_subtitle(&i18n("Normal"));
                &this.update_normal
            }
            3 => {
                this.update_speed_setting.set_subtitle(&i18n("Slow"));
                &this.update_slow
            }
            4 => {
                this.update_speed_setting.set_subtitle(&i18n("Very Slow"));
                &this.update_very_slow
            }
            _ => {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Unknown update speed setting, defaulting to normal"
                );
                this.update_speed_setting.set_subtitle(&i18n("Normal"));
                &this.update_normal
            }
        };
        selected_widget.set_checkmark_visible(true);
        this.current_speed_selection.set(selected_widget.get());

        this.settings.set(Some(settings));
    }

    fn set_initial_merge_process_stats(&self) {
        use gtk::glib::*;

        let settings = self.imp().settings.take();
        if settings.is_none() {
            g_critical!(
                "MissionCenter::Preferences",
                "Failed to configure merge process stats settings, could not load application settings"
            );
            return;
        }
        let settings = settings.unwrap();
        let merge_process_stats = settings.boolean("apps-page-merged-process-stats");
        let this = self.imp();
        let selected_widget = match merge_process_stats {
            false => {
                this.merged_process_stats.set_subtitle(&i18n("No"));
                &this.mps_no
            }
            true => {
                this.merged_process_stats.set_subtitle(&i18n("Yes"));
                &this.mps_yes
            }
        };
        selected_widget.set_checkmark_visible(true);
        this.current_mps_selection.set(selected_widget.get());

        this.settings.set(Some(settings));
    }
}
