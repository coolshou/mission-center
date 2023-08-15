/* sys_info_v2/gatherer/common/apps.rs
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

use arrayvec::ArrayVec;
use lazy_static::lazy_static;

use super::ArrayString;

lazy_static! {
    static ref PATH: Vec<String> = {
        let mut result = vec![];

        let mut path = std::env::var("PATH").unwrap_or_else(|_| {
            "/usr/local/bin:/usr/bin:/bin".to_string()
        });
        // Add some extra paths to search for binaries. In particular web-browsers seem to launch
        // themselves from a different path than the one in $PATH
        path.push_str(":/opt/brave.com/brave:/opt/google/chrome:/opt/microsoft/msedge:/opt/vivaldi:/usr/lib/firefox:/usr/lib64/firefox:/usr/lib/chromium-browser:/usr/lib64/chromium-browser:/usr/lib64/chromium:/usr/lib/opera:/usr/lib/x86_64-linux-gnu/opera:/usr/lib/aarch64-linux-gnu/opera:/usr/lib64/opera");

        for dir in path.split(':') {
            let p = std::path::Path::new(dir);
            if p.exists() {
                result.push(dir.to_string());
            }
        }

        result
    };

    static ref XDG_DATA_DIRS: Vec<String> = {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| format!("/usr/share:{}/.local/share", home));

        let mut dirs = xdg_data_dirs
            .split(':')
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        dirs.push(format!("{}/.local/share", home));

        dirs
    };
}

mod state {
    use std::{cell::Cell, thread_local};

    thread_local! {
        pub static APP_CACHE: Cell<Vec<super::AppDescriptor>> = Cell::new(vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct AppDescriptor {
    pub name: ArrayString,
    pub commands: ArrayVec<ArrayString, 8>,
    pub icon: Option<ArrayString>,

    pub app_id: Option<ArrayString>,
    pub is_flatpak: bool,
}

impl Default for AppDescriptor {
    fn default() -> Self {
        Self {
            name: Default::default(),
            commands: Default::default(),
            icon: None,
            app_id: None,
            is_flatpak: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstalledApps {
    pub apps: ArrayVec<AppDescriptor, 25>,
    pub is_complete: bool,
}

impl Default for InstalledApps {
    fn default() -> Self {
        Self {
            apps: ArrayVec::new(),
            is_complete: false,
        }
    }
}

impl InstalledApps {
    pub fn new() -> Self {
        let mut this = Self::default();

        let _ = super::running_apps::Apps::new();

        let app_cache = state::APP_CACHE.with(|state| unsafe { &mut *state.as_ptr() });
        if app_cache.is_empty() {
            for dir in &*XDG_DATA_DIRS {
                let dir = std::path::Path::new(dir);
                let dir = dir.join("applications");
                if dir.exists() {
                    load_apps_from_dir(dir, app_cache);
                }
            }
        }

        let drop_count = app_cache
            .chunks(this.apps.capacity())
            .next()
            .unwrap_or(&[])
            .len();

        let it = app_cache.drain(0..drop_count);
        this.apps.extend(it);
        this.is_complete = app_cache.is_empty();

        this
    }
}

fn load_apps_from_dir<P: AsRef<std::path::Path>>(path: P, apps: &mut Vec<AppDescriptor>) {
    let path = path.as_ref();
    let dir = match std::fs::read_dir(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to load apps from {}: {}", path.display(), e);
            return;
        }
    };

    for entry in dir {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path.is_dir() {
            load_apps_from_dir(path, apps);
        } else if path.is_file() {
            if path.to_string_lossy().ends_with(".desktop") {
                if let Some(app) = from_desktop_file(path) {
                    apps.push(app);
                }
            }
        }
    }
}

fn from_desktop_file<P: AsRef<std::path::Path>>(path: P) -> Option<AppDescriptor> {
    use super::ToArrayStringLossy;
    use ini::*;

    let path = path.as_ref();
    let ini = match Ini::load_from_file(path) {
        Ok(ini) => ini,
        Err(e) => {
            eprintln!("Failed to load desktop file from {}: {}", path.display(), e);
            return None;
        }
    };

    let section = match ini.section(Some("Desktop Entry")) {
        None => {
            eprintln!(
                "Failed to load desktop file from {}: Invalid or corrupt file, missing \"[Desktop Entry]\"",
                path.display()
            );
            return None;
        }
        Some(s) => s,
    };

    let hidden = section
        .get("NoDisplay")
        .unwrap_or_else(|| section.get("Hidden").unwrap_or("false"));
    if hidden.trim() != "false" {
        return None;
    }

    let name = match section.get("Name") {
        None => {
            eprintln!(
                "Failed to load desktop file from {}: Invalid or corrupt file, \"Name\" key is missing",
                path.display()
            );
            return None;
        }
        Some(n) => n,
    };

    let command = match section.get("Exec") {
        None => {
            eprintln!(
                "Failed to load desktop file from {}: Invalid or corrupt file, \"Exec\" key is missing",
                path.display()
            );
            return None;
        }
        Some(c) => c,
    };

    let (commands, is_flatpak) = match parse_command(command) {
        None => {
            eprintln!(
                "Failed to load desktop file from {}: Failed to parse \"Exec\" key.\nExec line is: '{}'",
                path.display(),
                command
            );
            return None;
        }
        Some(c) => c,
    };

    let app_id = if is_flatpak {
        parse_app_id(command)
    } else {
        None
    };

    let icon = section.get("Icon");

    Some(AppDescriptor {
        name: name.to_array_string_lossy(),
        commands,
        icon: icon.map(|s| s.to_array_string_lossy()),

        app_id: app_id.map(|s| s.to_array_string_lossy()),
        is_flatpak,
    })
}

fn parse_command(command: &str) -> Option<(ArrayVec<ArrayString, 8>, bool)> {
    use super::ToArrayStringLossy;

    let mut iter = command.split_whitespace();

    let mut commands = ArrayVec::new();
    let mut result = None;

    if let Some(cmd) = iter.next() {
        if !cmd.ends_with("flatpak") {
            if let Some(file_name) = std::path::Path::new(cmd).file_name() {
                for p in &*PATH {
                    let p = std::path::Path::new(p);
                    let cmd_path = p.join(file_name);
                    if cmd_path.exists() {
                        match commands.try_push(cmd_path.to_string_lossy().to_array_string_lossy())
                        {
                            Err(_) => break,
                            _ => {}
                        }
                    }

                    let file_name = file_name.to_string_lossy();

                    // The main Firefox process in RedHat and ArchLinux based distros is `firefox-bin`
                    if file_name.starts_with("firefox") {
                        let cmd_path = p.join("firefox-bin");
                        if cmd_path.exists() {
                            match commands
                                .try_push(cmd_path.to_string_lossy().to_array_string_lossy())
                            {
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    }

                    // Chromium-based browsers are wierd, their desktop file has the name of the
                    // binary with a `-stable`, `-beta`, `-nightly`, `-canary`, etc. suffix, but the
                    // binary itself doesn't have that suffix, so let's check for that too
                    let last_index = match file_name.rfind('-') {
                        None => continue,
                        Some(v) => v,
                    };

                    let file_name = &file_name[..last_index];

                    // Also Google Chrome is even stranger, the process is just called `chrome`
                    if file_name.starts_with("google-") {
                        let file_name = file_name.trim_start_matches("google-");
                        let cmd_path = p.join(file_name);
                        if cmd_path.exists() {
                            match commands
                                .try_push(cmd_path.to_string_lossy().to_array_string_lossy())
                            {
                                Err(_) => break,
                                _ => {}
                            }
                        }
                        // Vivaldi just randomly adds a `-bin` at the end
                    } else if file_name.starts_with("vivaldi") {
                        let cmd_path = p.join("vivaldi-bin");
                        if cmd_path.exists() {
                            match commands
                                .try_push(cmd_path.to_string_lossy().to_array_string_lossy())
                            {
                                Err(_) => break,
                                _ => {}
                            }
                        }
                        // Microsoft decided to call their binary `msedge` instead of `microsoft-edge`
                    } else if file_name.starts_with("microsoft-edge") {
                        let cmd_path = p.join("msedge");
                        if cmd_path.exists() {
                            match commands
                                .try_push(cmd_path.to_string_lossy().to_array_string_lossy())
                            {
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    } else {
                        let cmd_path = p.join(file_name);
                        if cmd_path.exists() {
                            match commands
                                .try_push(cmd_path.to_string_lossy().to_array_string_lossy())
                            {
                                Err(_) => break,
                                _ => {}
                            }
                        }
                    }
                }
            } else {
                match commands.try_push(cmd.to_array_string_lossy()) {
                    _ => {}
                }
            }

            result = Some((commands, false));
        } else {
            for arg in iter {
                if arg.starts_with("--command=") {
                    match commands.try_push(arg[10..].to_array_string_lossy()) {
                        Err(_) => break,
                        _ => {}
                    }
                    result = Some((commands, true));
                    break;
                }
            }
        }
    }

    result
}

fn parse_app_id(command: &str) -> Option<&str> {
    // We already know it's a flatpak app, so we can skip the first arg
    let iter = command.split_whitespace().skip(1);

    for arg in iter {
        if arg == "run" {
            continue;
        }

        if arg.starts_with("-") {
            continue;
        }

        if arg.starts_with("@") {
            continue;
        }

        if arg.starts_with("%") {
            continue;
        }

        return Some(arg);
    }

    None
}
