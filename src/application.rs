/* application.rs
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

use adw::glib::g_critical;
use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::config::VERSION;

mod imp {
    use super::*;

    pub struct MissionCenterApplication {
        pub settings: std::cell::Cell<Option<gio::Settings>>,
    }

    impl Default for MissionCenterApplication {
        fn default() -> Self {
            Self {
                settings: std::cell::Cell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MissionCenterApplication {
        const NAME: &'static str = "MissioncenterApplication";
        type Type = super::MissionCenterApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for MissionCenterApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            self.settings
                .set(Some(gio::Settings::new("io.missioncenter.MissionCenter")));

            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
        }
    }

    impl ApplicationImpl for MissionCenterApplication {
        fn activate(&self) {
            use gtk::glib::*;

            let application = self.obj();
            // Get the current window or create one if necessary
            let window = if let Some(window) = application.active_window() {
                window
            } else {
                let settings = self.settings.take();
                self.settings.set(settings.clone());

                let window = crate::MissionCenterWindow::new(&*application, settings.as_ref());
                window.connect_default_height_notify(clone!(@weak self as this => move |window| {
                    dbg!(window.default_height());

                    let settings = this.settings.take();
                    if settings.is_none() {
                        return;
                    }
                    let settings = settings.unwrap();
                    settings.set_int("window-height", window.default_height()).unwrap_or_else(|err|{
                        g_critical!("MissionCenter", "Failed to save window height: {}", err);
                    });

                    this.settings.set(Some(settings));
                }));
                window.connect_default_width_notify(clone!(@weak self as this => move|window| {
                    dbg!(window.default_width());

                    let settings = this.settings.take();
                    if settings.is_none() {
                        return;
                    }
                    let settings = settings.unwrap();
                    settings.set_int("window-width", window.default_width()).unwrap_or_else(|err|{
                        g_critical!("MissionCenter", "Failed to save window width: {}", err);
                    });

                    this.settings.set(Some(settings));
                }));

                if settings.is_none() {
                    g_critical!("MissionCenter", "Failed to load application settings");
                }
                let settings = settings.unwrap();
                window
                    .set_default_size(settings.int("window-width"), settings.int("window-height"));

                window.upcast()
            };

            window.present();
        }
    }

    impl GtkApplicationImpl for MissionCenterApplication {}

    impl AdwApplicationImpl for MissionCenterApplication {}
}

glib::wrapper! {
    pub struct MissionCenterApplication(ObjectSubclass<imp::MissionCenterApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl MissionCenterApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(
                MissionCenterApplication::static_type(),
                &mut [
                    ("application-id", application_id.into()),
                    ("flags", flags.into()),
                ],
            )
            .downcast()
            .unwrap()
        };

        this
    }

    pub fn refresh_readings(&self, readings: &mut crate::sys_info_v2::Readings) -> bool {
        use crate::MissionCenterWindow;
        use gtk::glib::*;

        let window = self.active_window();
        if window.is_none() {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to refresh data"
            );
            return false;
        }

        let window = window.unwrap();
        let window = window.downcast_ref::<MissionCenterWindow>();
        if window.is_none() {
            g_critical!(
                "MissionCenter::Application",
                "Active window is not a MissionCenterWindow",
            );
            return false;
        }

        window.unwrap().update_readings(readings)
    }

    pub fn default_instance() -> Option<Self> {
        let app = gio::Application::default();
        if app.is_none() {
            g_critical!(
                "MissionCenter",
                "Unable to get the default MissionCenterApplication instance"
            );
        }
        let app = app.unwrap();
        app.downcast_ref::<crate::MissionCenterApplication>()
            .cloned()
    }

    pub fn settings(&self) -> Option<gio::Settings> {
        let settings = unsafe { &*self.imp().settings.as_ptr() };
        settings.clone()
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let preferences_action = gio::ActionEntry::builder("preferences")
            .activate(move |app: &Self, _, _| {
                app.show_preferences();
            })
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        self.add_action_entries([quit_action, preferences_action, about_action]);
    }

    fn show_preferences(&self) {
        let window = self.active_window().unwrap();
        let settings = self.imp().settings.take();

        let preferences = crate::preferences::PreferencesWindow::new(&window, settings.as_ref());
        preferences.present();

        self.imp().settings.set(settings);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutWindow::builder()
            .transient_for(&window)
            .application_name("Mission Center")
            .application_icon("io.missioncenter.MissionCenter")
            .developer_name("Romeo Calota")
            .developers(["Romeo Calota"])
            .version(VERSION)
            .issue_url("https://gitlab.com/mission-center-devs/mission-center/-/issues")
            .copyright("© 2023 Mission Center Developers")
            .license_type(gtk::License::Gpl30)
            .website("https://missioncenter.io")
            .build();

        about.add_credit_section(
            Some("Standing on the shoulders of giants"),
            &[
                "GTK https://www.gtk.org/",
                "GNOME https://www.gnome.org/",
                "Libadwaita https://gitlab.gnome.org/GNOME/libadwaita",
                "Pathfinder 3 https://github.com/servo/pathfinder",
                "sysinfo https://docs.rs/sysinfo/latest/sysinfo",
                "NVTOP https://github.com/Syllo/nvtop",
                "musl libc https://musl.libc.org/",
                "Dmidecode https://www.nongnu.org/dmidecode/",
                "Workbench https://github.com/sonnyp/Workbench",
                "And many more... Thank you all!",
            ],
        );

        about.present();
    }
}
