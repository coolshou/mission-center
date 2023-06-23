/* proxy/src/apps.rs
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

use lazy_static::lazy_static;

lazy_static! {
    static ref PATH: Vec<String> = {
        let mut result = vec![];

        let path =
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

include!("../../common/util.rs");
include!("../../common/app.rs");

pub fn installed_apps() -> Vec<App> {
    let mut apps = vec![];

    for dir in &*XDG_DATA_DIRS {
        let dir = std::path::Path::new(dir);
        let dir = dir.join("applications");
        if dir.exists() {
            load_apps_from_dir(dir, &mut apps);
        }
    }

    apps
}

fn load_apps_from_dir<P: AsRef<std::path::Path>>(path: P, apps: &mut Vec<App>) {
    let path = path.as_ref();
    let dir = std::fs::read_dir(path);
    if dir.is_err() {
        eprintln!(
            "CRTFailed to load apps from {}: {}",
            path.display(),
            dir.err().unwrap()
        );
        return;
    }
    let dir = dir.unwrap();

    for entry in dir {
        let entry = entry.unwrap();
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

fn from_desktop_file<P: AsRef<std::path::Path>>(path: P) -> Option<App> {
    use ini::*;

    let path = path.as_ref();
    let ini = Ini::load_from_file(path);
    if ini.is_err() {
        eprintln!(
            "CRTFailed to load desktop file from {}: {}",
            path.display(),
            ini.err().unwrap()
        );
        return None;
    }
    let ini = ini.unwrap();

    let section = ini.section(Some("Desktop Entry"));
    if section.is_none() {
        eprintln!(
                 "CRTFailed to load desktop file from {}: Invalid or corrupt file, missing \"[Desktop Entry]\"",
                 path.display()
             );
        return None;
    }
    let section = section.unwrap();

    let hidden = section
        .get("NoDisplay")
        .unwrap_or_else(|| section.get("Hidden").unwrap_or("false"));
    if hidden.trim() != "false" {
        return None;
    }

    let name = section.get("Name");
    if name.is_none() {
        eprintln!(
            "CRTFailed to load desktop file from {}: Invalid or corrupt file, \"Name\" key is missing",
            path.display()
        );
        return None;
    }
    let name = name.unwrap();

    let command = section.get("Exec");
    if command.is_none() {
        eprintln!(
            "CRTFailed to load desktop file from {}: Invalid or corrupt file, \"Exec\" key is missing",
            path.display()
        );
        return None;
    }
    let command = command.unwrap();

    let cmd = parse_command(command);
    if cmd.is_none() {
        eprintln!(
                "DBGFailed to load desktop file from {}: Failed to parse \"Exec\" key.\nExec line is: '{}'",
                path.display(),
                command
            );
        return None;
    }
    let (cmd, is_flatpak) = cmd.unwrap();

    let app_id = if is_flatpak {
        parse_app_id(command)
    } else {
        None
    };

    let icon = section.get("Icon");
    let icon = match icon {
        Some(icon) => Some(icon.to_string()),
        None => None,
    };

    Some(App {
        name: name.to_string(),
        command: cmd,
        icon,

        app_id,
        is_flatpak,

        pids: vec![],
    })
}

fn parse_command(command: &str) -> Option<(String, bool)> {
    let mut iter = command.split_whitespace();

    if let Some(cmd) = iter.next() {
        if !cmd.ends_with("flatpak") {
            let cmd_path = std::path::Path::new(cmd);
            if !cmd_path.exists() {
                for p in &*PATH {
                    let p = std::path::Path::new(p);
                    let cmd_path = p.join(cmd_path);
                    if cmd_path.exists() {
                        return Some((cmd_path.to_string_lossy().to_string(), false));
                    }
                }
            } else {
                return Some((cmd.to_string(), false));
            }
        }

        for arg in iter {
            if arg.starts_with("--command=") {
                return Some((arg[10..].to_string(), true));
            }
        }
    }

    None
}

fn parse_app_id(command: &str) -> Option<String> {
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

        return Some(arg.to_string());
    }

    None
}
