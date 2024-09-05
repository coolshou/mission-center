/* application.rs
 *
 * Copyright 2024 Romeo Calota
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

use std::cell::{BorrowError, Cell, Ref, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gio,
    glib::{self, g_critical, property::PropertySet},
};

use crate::{config::VERSION, i18n::i18n, sys_info_v2::Readings};

mod imp {
    use super::*;

    pub struct MissionCenterApplication {
        pub settings: Cell<Option<gio::Settings>>,
        pub sys_info: RefCell<Option<crate::sys_info_v2::SysInfoV2>>,
        pub window: RefCell<Option<crate::MissionCenterWindow>>,
    }

    impl Default for MissionCenterApplication {
        fn default() -> Self {
            Self {
                settings: Cell::new(None),
                sys_info: RefCell::new(None),
                window: RefCell::new(None),
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

                let sys_info = crate::sys_info_v2::SysInfoV2::new();

                let window =
                    crate::MissionCenterWindow::new(&*application, settings.as_ref(), &sys_info);

                window.connect_default_height_notify({
                    let this = self.obj().downgrade();
                    move |window| {
                        let this = match this.upgrade() {
                            Some(this) => this,
                            None => return,
                        };
                        let this = this.imp();

                        let settings = this.settings.take();
                        if settings.is_none() {
                            return;
                        }
                        let settings = settings.unwrap();
                        settings
                            .set_int("window-height", window.default_height())
                            .unwrap_or_else(|err| {
                                g_critical!(
                                    "MissionCenter",
                                    "Failed to save window height: {}",
                                    err
                                );
                            });

                        this.settings.set(Some(settings));
                    }
                });
                window.connect_default_width_notify({
                    let this = self.obj().downgrade();
                    move |window| {
                        let this = match this.upgrade() {
                            Some(this) => this,
                            None => return,
                        };
                        let this = this.imp();

                        let settings = this.settings.take();
                        if settings.is_none() {
                            return;
                        }
                        let settings = settings.unwrap();
                        settings
                            .set_int("window-width", window.default_width())
                            .unwrap_or_else(|err| {
                                g_critical!(
                                    "MissionCenter",
                                    "Failed to save window width: {}",
                                    err
                                );
                            });

                        this.settings.set(Some(settings));
                    }
                });

                if settings.is_none() {
                    g_critical!("MissionCenter", "Failed to load application settings");
                }
                let settings = settings.unwrap();
                window
                    .set_default_size(settings.int("window-width"), settings.int("window-height"));

                sys_info.set_core_count_affects_percentages(
                    settings.boolean("apps-page-core-count-affects-percentages"),
                );

                settings.connect_changed(
                    Some("apps-page-core-count-affects-percentages"),
                    move |settings, _| {
                        let app = match super::MissionCenterApplication::default_instance() {
                            Some(app) => app,
                            None => {
                                g_critical!(
                                    "MissionCenter",
                                    "Failed to get default instance of MissionCenterApplication"
                                );
                                return;
                            }
                        };
                        match app.sys_info() {
                            Ok(sys_info) => {
                                sys_info.set_core_count_affects_percentages(
                                    settings.boolean("apps-page-core-count-affects-percentages"),
                                );
                            }
                            Err(e) => {
                                g_critical!(
                                    "MissionCenter",
                                    "Failed to get sys_info from MissionCenterApplication: {}",
                                    e
                                );
                            }
                        };
                    },
                );

                self.sys_info.set(Some(sys_info));

                let provider = gtk::CssProvider::new();
                provider.load_from_bytes(&Bytes::from_static(include_bytes!(
                    "../resources/ui/style.css"
                )));

                gtk::style_context_add_provider_for_display(
                    &gtk::gdk::Display::default().expect("Could not connect to a display."),
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );

                window.upcast()
            };

            window.present();

            self.window
                .set(window.downcast_ref::<crate::MissionCenterWindow>().cloned());
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
        use glib::g_message;

        let this: Self = glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build();

        g_message!(
            "MissionCenter::Application",
            "Starting Mission Center v{}",
            env!("CARGO_PKG_VERSION")
        );

        this
    }

    pub fn set_initial_readings(&self, readings: Readings) {
        use crate::MissionCenterWindow;
        use gtk::glib::*;

        let window = self.active_window();
        if window.is_none() {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to refresh data"
            );
            return;
        }

        let window = window.unwrap();
        let window = window.downcast_ref::<MissionCenterWindow>();
        if window.is_none() {
            g_critical!(
                "MissionCenter::Application",
                "Active window is not a MissionCenterWindow",
            );
            return;
        }

        window.unwrap().set_initial_readings(readings)
    }

    pub fn refresh_readings(&self, readings: &mut Readings) -> bool {
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
        match gio::Application::default() {
            Some(app) => app.downcast_ref::<Self>().cloned(),
            None => {
                g_critical!(
                    "MissionCenter",
                    "Unable to get the default MissionCenterApplication instance"
                );
                return None;
            }
        }
    }

    pub fn settings(&self) -> Option<gio::Settings> {
        let settings = unsafe { &*self.imp().settings.as_ptr() };
        settings.clone()
    }

    pub fn sys_info(&self) -> Result<Ref<crate::sys_info_v2::SysInfoV2>, BorrowError> {
        match self.imp().sys_info.try_borrow() {
            Ok(sys_info_ref) => Ok(Ref::map(sys_info_ref, |sys_info_opt| match sys_info_opt {
                Some(sys_info) => sys_info,
                None => {
                    panic!("MissionCenter::Application::sys_info() called before sys_info was initialized");
                }
            })),
            Err(e) => Err(e),
        }
    }

    pub fn window(&self) -> Option<crate::MissionCenterWindow> {
        unsafe { &*self.imp().window.as_ptr() }.clone()
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

        let preferences = crate::preferences::PreferencesDialog::new(settings.as_ref());
        preferences.present(Some(&window));

        self.imp().settings.set(settings);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name("Mission Center")
            .application_icon("io.missioncenter.MissionCenter")
            .developer_name("Mission Center Developers")
            .developers(["Romeo Calota", "QwertyChouskie", "jojo2357", "Jan Luca"])
            .translator_credits(i18n("translator-credits"))
            .version(VERSION)
            .issue_url("https://gitlab.com/mission-center-devs/mission-center/-/issues")
            .copyright("© 2024 Mission Center Developers")
            .license_type(gtk::License::Gpl30)
            .website("https://missioncenter.io")
            .release_notes(
                r#"<ul>
<li>New Services page: view and control system services using SystemD or OpenRC</li>
<li>New smooth graphs option</li>
<li>CPU frequency governor and driver are now displayed where supported</li>
<li>Indices are no longer shown next to devices if there is only one of its kind</li>
<li>A bunch of small tweaks that makes the app more consistent</li>
<li>Translation updates and fixes</li>
</ul>"#,
            )
            .build();

        about.add_credit_section(
            Some("Standing on the shoulders of giants"),
            &[
                "GTK https://www.gtk.org/",
                "GNOME https://www.gnome.org/",
                "Libadwaita https://gitlab.gnome.org/GNOME/libadwaita",
                "Blueprint Compiler https://jwestman.pages.gitlab.gnome.org/blueprint-compiler/",
                "NVTOP https://github.com/Syllo/nvtop",
                "Workbench https://github.com/sonnyp/Workbench",
                "And many more... Thank you all!",
            ],
        );

        about.present(Some(&window));
    }
}

pub const INTERVAL_STEP: f64 = 0.05;
pub const BASE_INTERVAL: f64 = 1f64;
pub const BASE_POINTS: u32 = 60;
