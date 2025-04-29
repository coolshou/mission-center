/* main.rs
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
use std::{
    env,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};
use std::cmp::PartialEq;
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::{gio, prelude::*};
use gtk::gio::Settings;
use application::MissionCenterApplication;
use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};
use i18n::{i18n, i18n_f};
use window::MissionCenterWindow;

mod application;
mod apps_page;
mod i18n;
mod magpie_client;
mod performance_page;
mod preferences;
mod services_page;
mod widgets;
mod window;

#[macro_export]
macro_rules! glib_clone {
    ($var:expr) => {{
        unsafe { &*$var.as_ptr() }.clone()
    }};
}

mod config {
    include!(concat!(env!("BUILD_ROOT"), "/src/config.rs"));
}

fn user_home() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();

    HOME.get_or_init(|| {
        env::var_os("HOME")
            .or(env::var_os("USERPROFILE"))
            .map(|v| PathBuf::from(v))
            .unwrap_or(if cfg!(windows) {
                "C:\\Windows\\Temp".into()
            } else {
                "/tmp".into()
            })
    })
    .as_path()
}

fn flatpak_data_dir() -> &'static Path {
    static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

    DATA_DIR
        .get_or_init(|| {
            let path = user_home().join(".var/app/io.missioncenter.MissionCenter/data");
            std::fs::create_dir_all(&path).expect("Failed to create flatpak data directory");
            path
        })
        .as_path()
}

pub fn is_flatpak() -> bool {
    static IS_FLATPAK: OnceLock<bool> = OnceLock::new();
    *IS_FLATPAK.get_or_init(|| Path::new("/.flatpak-info").exists())
}

// tysm gdu
pub fn to_human_readable_time(seconds: u64) -> String {
    const USEC_PER_YEAR: u64 = 60 * 60 * 6 * 1461; // ((60 * 60 * 24) as f32 * 365.25);
    const USEC_PER_MONTH: u64 = 60 * 30 * 1461; // ((60 * 60 * 24) as f32 * 365.25 / 12.0);
    const USEC_PER_DAY: u64 = 60 * 60 * 24;
    const USEC_PER_HOUR: u64 = 60 * 60;
    const USEC_PER_MINUTE: u64 = 60;

    pub fn years_to_string(time: u64) -> String {
        let timestr = time.to_string();
        if time == 1 {
            i18n_f("{} year", &[&timestr])
        } else {
            i18n_f("{} years", &[&timestr])
        }
    }

    pub fn months_to_string(time: u64) -> String {
        let timestr = time.to_string();
        if time == 1 {
            i18n_f("{} month", &[&timestr])
        } else {
            i18n_f("{} months", &[&timestr])
        }
    }

    pub fn days_to_string(time: u64) -> String {
        let timestr = time.to_string();
        if time == 1 {
            i18n_f("{} day", &[&timestr])
        } else {
            i18n_f("{} days", &[&timestr])
        }
    }

    pub fn hours_to_string(time: u64) -> String {
        let timestr = time.to_string();
        if time == 1 {
            i18n_f("{} hour", &[&timestr])
        } else {
            i18n_f("{} hours", &[&timestr])
        }
    }

    pub fn minutes_to_string(time: u64) -> String {
        let timestr = time.to_string();
        if time == 1 {
            i18n_f("{} minute", &[&timestr])
        } else {
            i18n_f("{} minutes", &[&timestr])
        }
    }

    pub fn seconds_to_string(time: u64) -> String {
        let timestr = time.to_string();
        if time == 1 {
            i18n_f("{} second", &[&timestr])
        } else {
            i18n_f("{} seconds", &[&timestr])
        }
    }

    let mut t = seconds;
    let years = t / USEC_PER_YEAR;
    t -= years * USEC_PER_YEAR;

    let months = t / USEC_PER_MONTH;
    t -= months * USEC_PER_MONTH;

    let days = t / USEC_PER_DAY;
    t -= days * USEC_PER_DAY;

    let hours = t / USEC_PER_HOUR;
    t -= hours * USEC_PER_HOUR;

    let minutes = t / USEC_PER_MINUTE;
    t -= minutes * USEC_PER_MINUTE;

    let seconds = t;

    let string3 = years_to_string(years);
    let years_str = string3.as_str();
    let string2 = months_to_string(months);
    let months_str = string2.as_str();
    let string1 = days_to_string(days);
    let days_str = string1.as_str();
    let string = hours_to_string(hours);
    let hours_str = string.as_str();
    let string4 = minutes_to_string(minutes);
    let minutes_str = string4.as_str();
    let string5 = seconds_to_string(seconds);
    let seconds_str = string5.as_str();

    if years > 0 {
        /* Translators: Used for duration greater than one year. First %s is number of years, second %s is months, third %s is days */
        i18n_f("{}, {} and {}", &[years_str, months_str, days_str])
    } else if months > 0 {
        /* Translators: Used for durations less than one year but greater than one month. First %s is number of months, second %s is days */
        i18n_f("{} and {}", &[months_str, days_str])
    } else if days > 0 {
        /* Translators: Used for durations less than one month but greater than one day. First %s is number of days, second %s is hours */
        i18n_f("{} and {}", &[days_str, hours_str])
    } else if hours > 0 {
        /* Translators: Used for durations less than one day but greater than one hour. First %s is number of hours, second %s is minutes */
        i18n_f("{} and {}", &[hours_str, minutes_str])
    } else if minutes > 0 {
        String::from(minutes_str)
    } else if seconds == 0 {
        seconds_str.to_string()
    } else {
        i18n("Less than a minute")
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DataType {
    MemoryBytes,
    DriveBytes,
    DriveBytesPerSecond,
    NetworkBytes,
    NetworkBytesPerSecond,
    Hertz,
    Watts,
}

fn data_type_setting_name(data_type: &DataType) -> &'static str {
    match data_type {
        DataType::MemoryBytes => { "memory" }
        DataType::DriveBytes => { "drive" }
        DataType::DriveBytesPerSecond => { "drive" }
        DataType::NetworkBytes => { "network" }
        DataType::NetworkBytesPerSecond => { "network" }
        DataType::Hertz => {
            panic!("Hertz data type not supported yet");
        }
        DataType::Watts => {
            panic!("Watts data type not supported yet");
        }
    }
}

pub fn to_human_readable_adv_str(
    value_bytes: f32,
    use_bytes: bool,
    use_binary: bool,
    per_second: bool,
    unit_label: &str,
    min_exponent: usize,
) -> String {
    const UNITS: [&'static str; 9] = ["", "K", "M", "G", "T", "P", "E", "Z", "Y"];

    let divisor = if use_binary {
        1024.
    } else {
        1000.
    };

    let (mut value, label) = if use_bytes {
        (value_bytes, if per_second { "/s" } else { "" })
    } else {
        (value_bytes * 8., if per_second { "ps" } else { "" })
    };

    let mut exponent = 0;

    // Only display bit/byte values in the given range or higher, not individual bits/bytes
    // This sacrifices some precision for the sake of readability
    while exponent < min_exponent {
        value /= divisor;
        exponent += 1;
    }

    while value >= divisor && exponent < UNITS.len() - 1 {
        value /= divisor;
        exponent += 1;
    }

    // Calculate number of decimals to display
    // Only display fractional values for bit/byte numbers in the defined range and bigger
    // This sacrifices some precision for the sake of readability
    let dec_to_display = if exponent > min_exponent {
        if value < 10.0 {
            2
        } else if value < 100.0 {
            1
        } else {
            0
        }
    } else {
        0
    };

    format!("{0:.1$} {2}{3}{4}{5}", value, dec_to_display, UNITS[exponent], if use_binary { "i" } else { "" }, unit_label, label)
}

pub fn to_human_readable_nice(
    value_bytes: f32,
    data_type: &DataType
) -> String {
    to_human_readable_nice_cached(value_bytes, data_type, &settings!())
}

pub fn to_human_readable_nice_cached(
    value_bytes: f32,
    data_type: &DataType,
    settings: &Settings,
) -> String {
    let label = match data_type {
        DataType::Hertz => { "Hz" }
        DataType::Watts => { "W" }
        _ => {
            let bytes = settings.boolean(&format!("performance-page-{}-use-bytes", data_type_setting_name(&data_type)));
            return to_human_readable_adv_str(
                value_bytes,
                bytes,
                settings.boolean(&format!("performance-page-{}-use-binary", data_type_setting_name(&data_type))),
                *data_type == DataType::NetworkBytesPerSecond || *data_type == DataType::DriveBytesPerSecond,
                if bytes { "B" } else { "b" },
                1,
            )
        }
    };
    
    to_human_readable_adv_str(
        value_bytes,
        true,
        false,
        false,
        label,
        0,
    )
}

pub fn show_error_dialog_and_exit(message: &str) -> ! {
    use crate::i18n::*;
    use adw::prelude::*;

    let message = Arc::<str>::from(message);
    gtk::glib::idle_add_once(move || {
        let app_window = app!().window();

        let error_dialog =
            adw::AlertDialog::new(Some("A fatal error has occurred"), Some(message.as_ref()));
        error_dialog.add_responses(&[("close", &i18n("_Quit"))]);
        error_dialog.set_response_appearance("close", adw::ResponseAppearance::Destructive);
        error_dialog.connect_response(None, |dialog, _| {
            dialog.close();
            std::process::exit(-1);
        });
        error_dialog.present(app_window.as_ref());
    });

    loop {}
}

fn main() {
    let home = user_home().to_string_lossy().to_string();
    let mut xdg_data_dirs = env::var_os("XDG_DATA_DIRS")
        .map(|str| str.to_string_lossy().to_string())
        .unwrap_or("/usr/share:/usr/local/share".into());
    xdg_data_dirs.push_str(&format!(":{home}/.local/share"));
    env::set_var("XDG_DATA_DIRS", xdg_data_dirs.replace('~', &home));

    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    let gresource_dir = if let Ok(gresource_dir) = std::env::var("MC_RESOURCE_DIR") {
        gresource_dir
    } else {
        PKGDATADIR.to_owned()
    };

    let resources = gio::Resource::load(&format!("{gresource_dir}/missioncenter.gresource"))
        .expect("Could not load resources");
    gio::resources_register(&resources);

    let app = MissionCenterApplication::new(
        "io.missioncenter.MissionCenter",
        &gio::ApplicationFlags::empty(),
    );
    gtk::Application::set_default(app.upcast_ref::<gtk::Application>());

    let exit_code = app.run();
    std::process::exit(exit_code.value());
}
