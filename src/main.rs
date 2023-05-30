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

use std::sync::{Arc, RwLock};

use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::gio;
use gtk::prelude::*;
use lazy_static::lazy_static;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};

use self::application::MissionCenterApplication;
use self::window::MissionCenterWindow;

mod application;
mod performance_page;
mod sys_info;
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
    pub static ref SYS_INFO: Arc<RwLock<sys_info::SysInfo>> =
        Arc::new(RwLock::new(sys_info::SysInfo::new()));
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

    // Initialize GL
    let lib = minidl::Library::load("libGL.so.1\0").expect("Unable to load libGL.so.1");
    gl::load_with(move |symbol| {
        let symbol_name = format!("{}\0", symbol);
        unsafe { lib.sym(&symbol_name).unwrap() }
    });

    // Take an initial measurement
    {
        let mut sys_info = SYS_INFO
            .write()
            .expect("System information refresh failed: Unable to acquire lock");
        sys_info.refresh_components_list();
        sys_info.refresh_all();
    }

    // Set up the system information refresh thread
    let sysinfo_refresh_thread_running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let sysinfo_refresh_thread_running_clone = Arc::clone(&sysinfo_refresh_thread_running);
    let sysinfo_refresh_thread = std::thread::spawn(move || {
        while sysinfo_refresh_thread_running_clone.load(std::sync::atomic::Ordering::Acquire) {
            std::thread::sleep(std::time::Duration::from_secs(1));

            {
                let mut sys_info = SYS_INFO
                    .write()
                    .expect("System information refresh failed: Unable to acquire lock");

                sys_info.refresh_components_list();
                sys_info.refresh_all();
            }
        }

        let mut sys_info = SYS_INFO
            .write()
            .expect("System information refresh failed: Unable to acquire lock");

        if let Some(gpu_info) = sys_info.gpu_info() {
            gpu_info.stop_gpud();
        }
    });

    // Create a new GtkApplication. The application manages our main loop,
    // application windows, integration with the window manager/compositor, and
    // desktop features such as file opening and single-instance applications.
    let app = MissionCenterApplication::new(
        "io.missioncenter.MissionCenter",
        &gio::ApplicationFlags::empty(),
    );

    // Run the application. This function will block until the application
    // exits. Upon return, we have our exit code to return to the shell. (This
    // is the code you see when you do `echo $?` after running a command in a
    // terminal.
    let exit_code = app.run();

    // Ask the system information refresh thread to stop
    sysinfo_refresh_thread_running.store(false, std::sync::atomic::Ordering::Release);
    sysinfo_refresh_thread
        .join()
        .expect("Unable to stop the system information refresh thread");

    std::process::exit(exit_code.into());
}
