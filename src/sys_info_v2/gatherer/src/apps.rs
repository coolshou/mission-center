/* sys_info_v2/gatherer/src/apps.rs
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

const APP_IGNORELIST: &[&str] = &["guake-prefs"];

lazy_static! {
    static ref PATH: Vec<String> = {
        let mut result = vec![];

        let mut path =
            std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".to_string());
        path.push_str(":/var/lib/flatpak/exports/bin");

        for dir in path.split(':') {
            if dir.is_empty() {
                continue;
            }
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
        pub static APP_CACHE: Cell<Vec<AppDescriptor>> = Cell::new(vec![]);
        pub static APP_PIDS_CACHE: Cell<Vec<u32>> = Cell::new(vec![]);
    }
}

include!("../common/apps.rs");

impl Apps {
    pub fn new() -> Self {
        use super::processes::Process;
        use std::collections::{BTreeSet, HashMap};

        #[derive(Debug, Clone)]
        struct App {
            descriptor: AppDescriptor,
            pids: BTreeSet<u32>,
        }

        fn extract_app_id(cgroup: &str) -> Option<String> {
            let app_scope = cgroup.split('/').last().unwrap_or_default();
            if app_scope.is_empty() {
                return None;
            }

            if app_scope.starts_with("snap") {
                let split: Vec<&str> = app_scope
                    .trim_start_matches("snap.")
                    .split('.')
                    .rev()
                    .skip(2)
                    .collect();
                if split.is_empty() {
                    return None;
                }

                let mut result = String::new();
                let mut split = split.iter().rev();
                result.push_str(unsafe { split.next().unwrap_unchecked() });
                for s in split {
                    result.push('_');
                    result.push_str(s);
                }
                Some(result)
            } else {
                app_scope
                    .split('-')
                    .skip(1)
                    .skip_while(|s| *s == "gnome" || *s == "plasma" || *s == "flatpak")
                    .next()
                    .and_then(|s| {
                        if s.is_empty() {
                            None
                        } else {
                            Some(s.replace("\\x2d", "-"))
                        }
                    })
            }
        }

        fn update_or_insert_app<'a>(
            app: &'a AppDescriptor,
            process: &Process,
            app_list: &mut HashMap<&'a str, App>,
        ) {
            if let Some(app) = app_list.get_mut(app.id.as_str()) {
                app.pids.insert(process.descriptor.pid);
                app.descriptor.stats.merge(&process.descriptor.stats);
            } else {
                let mut new_app = App {
                    descriptor: app.clone(),
                    pids: BTreeSet::new(),
                };
                new_app.pids.insert(process.descriptor.pid);
                new_app.descriptor.stats = process.descriptor.stats;
                app_list.insert(app.id.as_str(), new_app);
            }
        }

        let mut this = Self::default();

        let app_cache = state::APP_CACHE.with(|state| unsafe { &mut *state.as_ptr() });
        let app_pids_cache = state::APP_PIDS_CACHE.with(|state| unsafe { &mut *state.as_ptr() });

        if app_cache.is_empty() {
            app_pids_cache.clear();

            let mut installed_apps = HashMap::new();
            for dir in &*XDG_DATA_DIRS {
                let dir = std::path::Path::new(dir);
                let dir = dir.join("applications");
                if dir.exists() {
                    Self::load_apps_from_dir(dir, &mut installed_apps);
                }
            }

            let mut running_apps = HashMap::new();

            let processes: Vec<Process> = super::Processes::process_cache()
                .iter()
                .map(|(_, p)| p.clone())
                .collect();

            for process in &processes {
                match process
                    .cgroup
                    .as_ref()
                    .and_then(|cgroup| extract_app_id(cgroup.as_str()))
                {
                    None => {
                        for app in installed_apps.values() {
                            if process
                                .descriptor
                                .exe
                                .as_str()
                                .starts_with(app.command.as_str())
                            {
                                update_or_insert_app(app, process, &mut running_apps);
                                break;
                            }

                            if process
                                .descriptor
                                .cmd
                                .iter()
                                .any(|cmd| cmd.starts_with(app.command.as_str()))
                            {
                                update_or_insert_app(app, process, &mut running_apps);
                                break;
                            }
                        }
                    }
                    Some(app_id) => {
                        if let Some(app) = installed_apps.get(&app_id) {
                            update_or_insert_app(app, process, &mut running_apps);
                        }
                    }
                };
            }

            for app in running_apps.values() {
                app_cache.push(app.descriptor.clone());
                app_pids_cache.extend(app.pids.iter().copied());
                app_pids_cache.push(0);
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

    fn load_apps_from_dir<P: AsRef<std::path::Path>>(
        path: P,
        apps: &mut std::collections::HashMap<String, AppDescriptor>,
    ) {
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
                Self::load_apps_from_dir(path, apps);
            } else if path.is_file() {
                if path.to_string_lossy().ends_with(".desktop") {
                    if let Some(app) = Self::from_desktop_file(path) {
                        apps.insert(app.id.to_string(), app);
                    }
                }
            }
        }
    }

    fn from_desktop_file<P: AsRef<std::path::Path>>(path: P) -> Option<AppDescriptor> {
        use super::ToArrayStringLossy;
        use ini::*;

        let path = path.as_ref();

        let app_id = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_array_string_lossy())
            .unwrap_or_default();

        if APP_IGNORELIST.contains(&app_id.as_str()) {
            return None;
        }

        let ini = match Ini::load_from_file_opt(
            path,
            ParseOption {
                enabled_quote: false,
                enabled_escape: true,
            },
        ) {
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

        let command = match Self::parse_command(command) {
            None => return None,
            Some(c) => c,
        };

        Some(AppDescriptor {
            name: name.to_array_string_lossy(),
            icon: section.get("Icon").map(|s| s.to_array_string_lossy()),
            id: app_id,
            command,
            ..Default::default()
        })
    }

    fn parse_command(command_line: &str) -> Option<ArrayString> {
        use super::ToArrayStringLossy;

        let iter = command_line.split_whitespace();

        let mut cmd_line_split = iter.clone();
        match cmd_line_split.next() {
            Some(cmd) => {
                if cmd.ends_with("flatpak") {
                    for arg in cmd_line_split {
                        if arg.starts_with("--command=") {
                            let binary_name = match arg[10..].split('/').last() {
                                Some(b) => b,
                                None => continue,
                            };
                            return Some(binary_name.to_array_string_lossy());
                        }
                    }
                    return None;
                }
            }
            None => return None,
        };

        'outer: for arg in iter {
            let binary_name = arg.split('/').last().unwrap_or("");
            if binary_name.is_empty() {
                continue;
            }

            for ignored in COMMAND_IGNORELIST {
                if arg.starts_with(*ignored) || binary_name.starts_with(*ignored) {
                    continue 'outer;
                }
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
}

impl AppPIDs {
    pub fn new() -> Self {
        let mut this = Self::default();

        let app_pids_cache = state::APP_PIDS_CACHE.with(|state| unsafe { &mut *state.as_ptr() });

        let drop_count = app_pids_cache
            .chunks(this.pids.capacity())
            .next()
            .unwrap_or(&[])
            .len();

        let it = app_pids_cache.drain(0..drop_count);
        this.pids.extend(it);
        this.is_complete = app_pids_cache.is_empty();

        this
    }
}
