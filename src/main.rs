/* main.rs
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

use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::gio;
use gtk::prelude::*;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};

use self::application::MissionCenterApplication;
use self::window::MissionCenterWindow;

mod application;
mod window;

mod config {
    include!(concat!(env!("BUILD_ROOT"), "/src/config.rs"));
}

fn main() {
    // Set up gettext translations
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    // Find the GSETTINGS_SCHEMA_DIR environment variable
    let gschema_dir = if let Ok(gschema_dir) = std::env::var("GSETTINGS_SCHEMA_DIR") {
        gschema_dir
    } else {
        PKGDATADIR.to_owned()
    };

    // Load resources
    let resources = gio::Resource::load(gschema_dir + "/missioncenter.gresource")
        .expect("Could not load resources");
    gio::resources_register(&resources);

    // Create a new GtkApplication. The application manages our main loop,
    // application windows, integration with the window manager/compositor, and
    // desktop features such as file opening and single-instance applications.
    let app = MissionCenterApplication::new(
        "me.kicsyromy.MissionCenter",
        &gio::ApplicationFlags::empty(),
    );

    // Run the application. This function will block until the application
    // exits. Upon return, we have our exit code to return to the shell. (This
    // is the code you see when you do `echo $?` after running a command in a
    // terminal.
    std::process::exit(app.run().into());
}
