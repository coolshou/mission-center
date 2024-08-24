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

use adw::{prelude::*, subclass::prelude::*, SwitchRow};
use gtk::{gio, glib};

const MAX_INTERVAL_TICKS: u64 = 200;
const MIN_INTERVAL_TICKS: u64 = 10;

const MAX_POINTS: i32 = 600;
const MIN_POINTS: i32 = 10;

mod imp {
    use adw::SpinRow;
    use gtk::Scale;

    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/preferences/page.ui")]
    pub struct PreferencesPage {
        #[template_child]
        pub update_interval: TemplateChild<SpinRow>,
        #[template_child]
        pub data_points: TemplateChild<Scale>,

        #[template_child]
        pub merged_process_stats: TemplateChild<SwitchRow>,
        #[template_child]
        pub remember_sorting: TemplateChild<SwitchRow>,
        #[template_child]
        pub core_count_affects_percentages: TemplateChild<SwitchRow>,
        #[template_child]
        pub smooth_graphs: TemplateChild<SwitchRow>,

        pub settings: Cell<Option<gio::Settings>>,
    }

    impl Default for PreferencesPage {
        fn default() -> Self {
            Self {
                update_interval: Default::default(),
                data_points: Default::default(),

                merged_process_stats: Default::default(),
                remember_sorting: Default::default(),
                core_count_affects_percentages: Default::default(),
                smooth_graphs: Default::default(),

                settings: Cell::new(None),
            }
        }
    }

    impl PreferencesPage {
        pub fn configure_update_speed(&self) {
            use crate::application::INTERVAL_STEP;
            use glib::g_critical;

            let settings = self.settings.take();
            if settings.is_none() {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to configure update speed settings, could not load application settings"
                );
                return;
            }
            let settings = settings.unwrap();

            let new_interval = (self.update_interval.value() / INTERVAL_STEP).round() as u64;
            let new_points = self.data_points.value() as i32;

            if new_interval <= MAX_INTERVAL_TICKS && new_interval >= MIN_INTERVAL_TICKS {
                if settings
                    .set_uint64("app-update-interval-u64", new_interval)
                    .is_err()
                {
                    g_critical!(
                        "MissionCenter::Preferences",
                        "Failed to set update interval setting",
                    );
                }
            } else {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Update interval out of bounds",
                );
            }

            if new_points <= MAX_POINTS && new_points >= MIN_POINTS {
                if settings
                    .set_int("perfomance-page-data-points", new_points)
                    .is_err()
                {
                    g_critical!(
                        "MissionCenter::Preferences",
                        "Failed to set update points setting",
                    );
                }
            } else {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Points interval out of bounds",
                );
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
            SwitchRow::ensure_type();

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

            self.data_points
                .downcast_ref::<Scale>()
                .unwrap()
                .connect_value_changed({
                    let this = self.obj().downgrade();
                    move |_| {
                        if let Some(this) = this.upgrade() {
                            this.imp().configure_update_speed();
                        }
                    }
                });

            self.update_interval
                .downcast_ref::<SpinRow>()
                .unwrap()
                .connect_changed({
                    let this = self.obj().downgrade();
                    move |_| {
                        if let Some(this) = this.upgrade() {
                            this.imp().configure_update_speed();
                        }
                    }
                });

            self.merged_process_stats.connect_active_notify({
                let this = self.obj().downgrade();
                move |switch_row| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let settings = this.settings.take();
                    if let Some(settings) = settings {
                        if let Err(e) = settings
                            .set_boolean("apps-page-merged-process-stats", switch_row.is_active())
                        {
                            g_critical!(
                                "MissionCenter::Preferences",
                                "Failed to set merged process stats setting: {}",
                                e
                            );
                        }
                        this.settings.set(Some(settings));
                    }
                }
            });

            self.remember_sorting.connect_active_notify({
                let this = self.obj().downgrade();
                move |switch_row| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let settings = this.settings.take();
                    if let Some(settings) = settings {
                        if let Err(e) = settings
                            .set_boolean("apps-page-remember-sorting", switch_row.is_active())
                        {
                            g_critical!(
                                "MissionCenter::Preferences",
                                "Failed to set merged process stats setting: {}",
                                e
                            );
                        }
                        this.settings.set(Some(settings));
                    }
                }
            });

            self.core_count_affects_percentages.connect_active_notify({
                let this = self.obj().downgrade();
                move |switch_row| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let settings = this.settings.take();
                    if let Some(settings) = settings {
                        if let Err(e) = settings.set_boolean(
                            "apps-page-core-count-affects-percentages",
                            switch_row.is_active(),
                        ) {
                            g_critical!(
                                "MissionCenter::Preferences",
                                "Failed to set core counts affects percentages setting: {}",
                                e
                            );
                        }
                        this.settings.set(Some(settings));
                    }
                }
            });

            self.smooth_graphs.connect_active_notify({
                let this = self.obj().downgrade();
                move |switch_row| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let settings = this.settings.take();
                    if let Some(settings) = settings {
                        if let Err(e) = settings
                            .set_boolean("performance-smooth-graphs", switch_row.is_active())
                        {
                            g_critical!(
                                "MissionCenter::Preferences",
                                "Failed to set smooth graphs setting: {}",
                                e
                            );
                        }
                        this.settings.set(Some(settings));
                    }
                }
            });
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
        this.set_initial_remember_sorting_option();
        this.set_initial_core_count_affects_percentages();
        this.set_initial_smooth_graphs();

        this
    }

    fn set_initial_update_speed(&self) {
        use crate::application::INTERVAL_STEP;
        use gtk::glib::*;

        let settings = match self.imp().settings.take() {
            None => {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to set up update speed settings, could not load application settings"
                );
                return;
            }
            Some(settings) => settings,
        };
        let data_points = settings.int("perfomance-page-data-points");
        let update_interval_s = (settings.uint64("app-update-interval-u64") as f64) * INTERVAL_STEP;
        let this = self.imp();

        this.data_points.set_value(data_points as f64);
        this.update_interval.set_value(update_interval_s);

        this.settings.set(Some(settings));
    }

    fn set_initial_merge_process_stats(&self) {
        use gtk::glib::*;

        let settings = match self.imp().settings.take() {
            None => {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to configure merge process stats setting, could not load application settings"
                );
                return;
            }
            Some(settings) => settings,
        };

        let this = self.imp();
        this.merged_process_stats
            .set_active(settings.boolean("apps-page-merged-process-stats"));

        this.settings.set(Some(settings));
    }

    fn set_initial_remember_sorting_option(&self) {
        use gtk::glib::*;

        let settings = match self.imp().settings.take() {
            None => {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to configure remember sorting setting, could not load application settings"
                );
                return;
            }
            Some(settings) => settings,
        };

        let this = self.imp();

        let remember_sorting = settings.boolean("apps-page-remember-sorting");
        if !remember_sorting {
            if let Err(e) = settings.set_enum("apps-page-sorting-column", 255) {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to reset apps-page-sorting-column setting: {}",
                    e
                );
            }
            if let Err(e) = settings.set_enum("apps-page-sorting-order", 255) {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to reset apps-page-sorting-order setting: {}",
                    e
                );
            }
        }

        this.remember_sorting.set_active(remember_sorting);

        this.settings.set(Some(settings));
    }

    fn set_initial_core_count_affects_percentages(&self) {
        use gtk::glib::*;

        let settings = match self.imp().settings.take() {
            None => {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to configure core count affects percentages setting, could not load application settings"
                );
                return;
            }
            Some(settings) => settings,
        };

        let this = self.imp();
        this.core_count_affects_percentages
            .set_active(settings.boolean("apps-page-core-count-affects-percentages"));

        this.settings.set(Some(settings));
    }

    fn set_initial_smooth_graphs(&self) {
        use gtk::glib::*;

        let settings = match self.imp().settings.take() {
            None => {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Failed to configure smooth graphs setting, could not load application settings"
                );
                return;
            }
            Some(settings) => settings,
        };

        let this = self.imp();
        this.smooth_graphs
            .set_active(settings.boolean("performance-smooth-graphs"));

        this.settings.set(Some(settings));
    }
}
