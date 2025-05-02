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

use crate::{config::VERSION, i18n::i18n, magpie_client::Readings};

pub const INTERVAL_STEP: f64 = 0.05;
pub const BASE_INTERVAL: f64 = 1f64;

#[macro_export]
macro_rules! app {
    () => {{
        use ::gtk::glib::object::Cast;
        ::gtk::gio::Application::default()
            .and_then(|app| app.downcast::<$crate::MissionCenterApplication>().ok())
            .expect("Failed to get MissionCenterApplication instance")
    }};
}

#[macro_export]
macro_rules! settings {
    () => {
        $crate::app!().settings()
    };
}

mod imp {
    use super::*;

    pub struct MissionCenterApplication {
        pub settings: Cell<Option<gio::Settings>>,
        pub sys_info: RefCell<Option<crate::magpie_client::MagpieClient>>,
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

            obj.set_default();

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
            let window = if let Some(window) = application.window() {
                window
            } else {
                let settings = unsafe { self.settings.take().unwrap_unchecked() };
                self.settings.set(Some(settings.clone()));

                let sys_info = crate::magpie_client::MagpieClient::new();

                let window = crate::MissionCenterWindow::new(&*application, &settings, &sys_info);

                window.connect_default_height_notify({
                    move |window| {
                        let settings = settings!();
                        settings
                            .set_int("window-height", window.default_height())
                            .unwrap_or_else(|err| {
                                g_critical!(
                                    "MissionCenter",
                                    "Failed to save window height: {}",
                                    err
                                );
                            });
                    }
                });
                window.connect_default_width_notify({
                    move |window| {
                        let settings = settings!();
                        settings
                            .set_int("window-width", window.default_width())
                            .unwrap_or_else(|err| {
                                g_critical!(
                                    "MissionCenter",
                                    "Failed to save window width: {}",
                                    err
                                );
                            });
                    }
                });

                window
                    .set_default_size(settings.int("window-width"), settings.int("window-height"));

                window.connect_maximized_notify({
                    move |window| {
                        let settings = settings!();
                        settings
                            .set_boolean("is-maximized", window.is_maximized())
                            .unwrap_or_else(|err| {
                                g_critical!(
                                    "MissionCenter",
                                    "Failed to save window maximization: {}",
                                    err
                                );
                            });
                    }
                });

                window.set_maximized(settings.boolean("is-maximized"));

                sys_info.set_core_count_affects_percentages(
                    settings.boolean("apps-page-core-count-affects-percentages"),
                );

                settings.connect_changed(
                    Some("apps-page-core-count-affects-percentages"),
                    move |settings, _| {
                        let app = app!();
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
        use gtk::glib::*;

        let Some(window) = self.window() else {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to refresh data"
            );
            return;
        };

        window.set_initial_readings(readings)
    }

    pub fn setup_animations(&self) {
        use gtk::glib::*;

        let Some(window) = self.window() else {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to refresh data"
            );
            return;
        };

        window.setup_animations()
    }

    pub fn refresh_readings(&self, readings: &mut Readings) -> bool {
        use gtk::glib::*;

        let Some(window) = self.window() else {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to refresh data"
            );
            return false;
        };

        window.update_readings(readings)
    }

    pub fn refresh_animations(&self) -> bool {
        use gtk::glib::*;

        let Some(window) = self.window() else {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to refresh data"
            );
            return false;
        };

        window.update_animations()
    }

    pub fn settings(&self) -> gio::Settings {
        unsafe { (&*self.imp().settings.as_ptr()).as_ref().unwrap_unchecked() }.clone()
    }

    pub fn sys_info(&self) -> Result<Ref<crate::magpie_client::MagpieClient>, BorrowError> {
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
        let keyboard_shortcuts_action = gio::ActionEntry::builder("keyboard-shortcuts")
            .activate(move |app: &Self, _, _| app.show_keyboard_shortcuts())
            .build();

        self.add_action_entries([
            quit_action,
            preferences_action,
            about_action,
            keyboard_shortcuts_action,
        ]);

        self.set_accels_for_action("app.preferences", &["<Control>comma"]);
        self.set_accels_for_action("app.keyboard-shortcuts", &["<Control>question"]);
    }

    fn show_preferences(&self) {
        let Some(window) = self.window() else {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to show preferences"
            );
            return;
        };

        let preferences = crate::preferences::PreferencesDialog::new();
        preferences.present(Some(&window));
    }

    fn show_keyboard_shortcuts(&self) {
        let Some(app_window) = self.window() else {
            return;
        };

        let builder =
            gtk::Builder::from_resource("/io/missioncenter/MissionCenter/ui/keyboard_shortcuts.ui");
        let dialog = builder
            .object::<adw::PreferencesDialog>("keyboard_shortcuts")
            .expect("Failed to get shortcuts window");

        let select_device = builder
            .object::<gtk::ShortcutsShortcut>("select_device")
            .expect("Failed to get select device shortcut");

        // This is a hack to set the label of the shortcut to from 'Ctrl + F10' to 'F1 .. F10'
        if let Some(ctrl_label) = select_device
            .first_child()
            .and_then(|c| c.first_child())
            .and_then(|c| c.next_sibling())
            .and_then(|c| c.first_child())
            .and_then(|c| c.downcast::<gtk::Label>().ok())
        {
            ctrl_label.set_label(&"F1");
            ctrl_label.set_width_request(-1);
            ctrl_label
                .next_sibling()
                .and_then(|c| c.downcast::<gtk::Label>().ok())
                .and_then(|l| Some(l.set_label("..")));
        }

        dialog.present(Some(&app_window));
    }

    fn show_about(&self) {
        let Some(window) = self.window() else {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to show about dialog"
            );
            return;
        };

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
            .release_notes(r#"
<p>Major Changes:</p>
<ul>
    <li>Complete refactor of the Gatherer, now called Magpie</li>
    <li>Magpie available as a standalone set of libraries and executable; can be used by other
        monitoring solutions
    </li>
    <li>Show SMART data for SATA and NVMe devices</li>
    <li>Allow ejecting removable storage devices (optical disks, USB thumb drives, etc.)
    </li>
    <li>Per-process network usage (see
        https://gitlab.com/mission-center-devs/mission-center/-/wikis/Home/Nethogs)
    </li>
    <li>Redesigned Apps Page with more information available for each app and process</li>
</ul>
<p>Noteworthy changes</p>
<ul>
    <li>A new option to make graphs, in the Performance Page, glide smoothly</li>
    <li>Visual and functional improvements in the Memory view</li>
    <li>Enabled all NVTOP supported GPUs including RaspberryPi (see
        https://github.com/Syllo/nvtop?tab=readme-ov-file#gpu-support)
    </li>
    <li>Right sidebar in the Performance page is now more consistent between devices</li>
    <li>Support for keyboard shortcuts</li>
    <li>Show maximum bitrate for all network interfaces</li>
    <li>Allow customizing units for memory, disk and network usage</li>
    <li>Removed a bunch of unsafe code from the main app</li>
    <li>Lower overall memory usage</li>
    <li>Lower overall CPU usage</li>
    <li>Improved overall responsiveness of the app and minimized time deviations between refresh cycles

    </li>
    <li>Enable full LTO for AppImage, Flatpak and Snap builds</li>
</ul>
<p>Bug fixes:</p>
<ul>
    <li>Missing memory composition graph after update to libadwaita 1.7</li>
    <li>Service details window to narrow after update to libadwaita 1.7</li>
    <li>Integer overflow when copying large files to/from slower storage devices</li>
    <li>Gatherer stops responding after a period of time</li>
    <li>Don't call `lsblk` to figure out if a drive has the root partition</li>
    <li>Fixed memory leak that would occur on some OS/HW combinations</li>
    <li>Don't target a specific CPU architecture variant when building Magpie</li>
    <li>Fix GPU encoding/decoding graphs misrepresenting the type of workload</li>
</ul>
<p>Translation updates</p>
<ul>
    <li>Catalan by Jaime Muñoz Martín</li>
    <li>Chinese (Simplified Han script) by Zhang Peng</li>
    <li>Chinese (Traditional Han script) by jhihyulin, D0735, Peter Dave Hello</li>
    <li>Czech by pavelbo</li>
    <li>Dutch by philip.goto</li>
    <li>Estonian by Priit Jõerüüt, Indrek Haav</li>
    <li>Finnish by Ricky-Tigg</li>
    <li>French by europa91m DenisMARCQ, Louis-Simon</li>
    <li>Galician by Espasant3</li>
    <li>German by Aircraft192, Lauritz, Tieste, ItsGamerik</li>
    <li>Hebrew by yarons</li>
    <li>Italian by beppeilgommista, ppasserini, svlmrc</li>
    <li>Japanese by rainy_sunset</li>
    <li>Korean by darkcircle.0426</li>
    <li>Norwegian Bokmål by ovl-1, Telaneo</li>
    <li>Polish by keloH</li>
    <li>Portuguese by hugok79</li>
    <li>Spanish by Espasant3</li>
    <li>Tamil by anishprabu.t</li>
</ul>
<p>For a more detailed set of release notes see:
    https://gitlab.com/mission-center-devs/mission-center/-/wikis/Release-Notes/v1.0.0
</p>"#)
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
