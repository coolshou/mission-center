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
use lazy_static::lazy_static;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};

use self::application::MissionCenterApplication;
use self::window::MissionCenterWindow;

mod application;
mod i18n;
mod performance_page;
mod preferences;
mod sys_info_v2;
mod window;

mod config {
    include!(concat!(env!("BUILD_ROOT"), "/src/config.rs"));
}

lazy_static! {
    pub static ref HW_DB_DIR: String = {
        if let Ok(path) = std::env::var("HW_DB_DIR") {
            path
        } else {
            PKGDATADIR.to_owned()
        }
    };
}

pub fn to_human_readable(value: f32, divisor: f32) -> (f32, &'static str) {
    const UNITS: [&'static str; 9] = ["", "K", "M", "G", "T", "P", "E", "Z", "Y"];

    let mut index = 0;
    let mut value = value;

    while value >= divisor && index < UNITS.len() - 1 {
        value /= divisor;
        index += 1;
    }

    (value, UNITS[index])
}

fn main() {
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    let gresource_dir = if let Ok(gresource_dir) = std::env::var("MC_RESOURCE_DIR") {
        gresource_dir
    } else {
        PKGDATADIR.to_owned()
    };

    let resources = gio::Resource::load(gresource_dir + "/missioncenter.gresource")
        .expect("Could not load resources");
    gio::resources_register(&resources);

    let lib = minidl::Library::load("libGL.so.1\0").expect("Unable to load libGL.so.1");
    gl::load_with(move |symbol| {
        let symbol_name = format!("{}\0", symbol);
        unsafe { lib.sym(&symbol_name).unwrap() }
    });

    let app = MissionCenterApplication::new(
        "io.missioncenter.MissionCenter",
        &gio::ApplicationFlags::empty(),
    );
    gtk::Application::set_default(app.upcast_ref::<gtk::Application>());

    let exit_code = app.run();
    std::process::exit(exit_code.into());
}
