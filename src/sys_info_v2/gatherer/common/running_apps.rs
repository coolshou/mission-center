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

const COMMAND_IGNORELIST: &[&str] = &[
    "/usr/bin/sh",
    "/usr/bin/bash",
    "/usr/bin/zsh",
    "/usr/bin/fish",
    "/usr/bin/tmux",
    "/usr/bin/nu",
    "/usr/bin/screen",
    "/usr/bin/python",
    "/usr/bin/python2",
    "/usr/bin/python3",
    "/bin/sh",
    "/bin/bash",
    "/bin/zsh",
    "/bin/fish",
    "/bin/tmux",
    "/bin/nu",
    "/bin/screen",
    "/bin/python",
    "/bin/python2",
    "/bin/python3",
    "sh",
    "bash",
    "zsh",
    "fish",
    "tmux",
    "nu",
    "screen",
    "python",
    "python2",
    "python3",
];

lazy_static! {
    static ref PATH: Vec<String> = {
        let mut result = vec![];

        let mut path =
            std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".to_string());

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

    use super::*;

    thread_local! {
        pub static APP_CACHE: Cell<Vec<AppDescriptor>> = Cell::new(vec![]);
        pub static APP_PIDS_CACHE: Cell<Vec<u32>> = Cell::new(vec![]);
    }
}

pub type Stats = super::processes::Stats;

#[derive(Debug, Clone)]
pub struct AppDescriptor {
    pub name: ArrayString,
    pub icon: Option<ArrayString>,
    pub command: ArrayString,
    pub stats: Stats,
}

impl Default for AppDescriptor {
    fn default() -> Self {
        Self {
            name: Default::default(),
            icon: None,
            command: Default::default(),
            stats: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Apps {
    pub apps: ArrayVec<AppDescriptor, 25>,
    pub is_complete: bool,
}

// PIDs for the running apps. The order is the same as the order of the apps in the `Apps` struct.
// Each group of PIDs for an app is separated by a `0` value.
#[derive(Debug, Clone)]
pub struct AppPids {
    pub pids: ArrayVec<u32, 100>,
    pub is_complete: bool,
}

impl Default for Apps {
    fn default() -> Self {
        Self {
            apps: ArrayVec::new(),
            is_complete: false,
        }
    }
}

impl Apps {
    pub fn new() -> Self {
        let mut this = Self::default();

        let app_cache = state::APP_CACHE.with(|state| unsafe { &mut *state.as_ptr() });
        if app_cache.is_empty() {
            let mut installed_apps = vec![];
            for dir in &*XDG_DATA_DIRS {
                let dir = std::path::Path::new(dir);
                let dir = dir.join("applications");
                if dir.exists() {
                    load_apps_from_dir(dir, &mut installed_apps);
                }
            }

            let processes = match super::Processes::process_hierarchy() {
                Some(p) => p,
                None => {
                    eprintln!("Gatherer: Failed to get process hierarchy for running apps");
                    return this;
                }
            };
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

    let command = match parse_command(command) {
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

    let icon = section.get("Icon");

    Some(AppDescriptor {
        name: name.to_array_string_lossy(),
        icon: icon.map(|s| s.to_array_string_lossy()),
        command,
        ..Default::default()
    })
}

fn parse_command(command_line: &str) -> Option<ArrayString> {
    use super::ToArrayStringLossy;

    let mut iter = command_line.split_whitespace();

    let mut cmd_line_split = iter.clone();
    match cmd_line_split.next() {
        Some(cmd) => {
            if cmd.ends_with("flatpak") {
                for arg in cmd_line_split {
                    if arg.starts_with("--command=") {
                        return Some(arg[10..].to_array_string_lossy());
                    }
                }
            }
        }
        None => return None,
    };

    for arg in iter {
        if COMMAND_IGNORELIST.contains(&arg) {
            continue;
        }
        let binary_name = arg.split('/').last().unwrap_or("");
        if binary_name.is_empty() || COMMAND_IGNORELIST.contains(&binary_name) {
            continue;
        }

        for p in &*PATH {
            let p = std::path::Path::new(p);
            let cmd_path = p.join(binary_name);
            if cmd_path.exists() && cmd_path.is_file() {
                return Some(cmd_path.to_string_lossy().to_array_string_lossy());
            }
        }
    }

    None
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
