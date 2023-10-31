use std::{collections::HashMap, sync::Arc};

use lazy_static::lazy_static;

use crate::platform::apps::*;

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
    "/usr/bin/distrobox",
    "/usr/bin/waydroid",
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
    "distrobox",
    "waydroid",
];

const APP_IGNORELIST: &[&str] = &["guake-prefs"];
const STALE_DELTA: std::time::Duration = std::time::Duration::from_millis(1000);

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

pub type AppUsageStats = crate::platform::AppUsageStats;
type LinuxProcess = crate::platform::Process;

#[derive(Debug, Clone)]
pub struct LinuxApp {
    pub name: Arc<str>,
    pub icon: Option<Arc<str>>,
    pub id: Arc<str>,
    pub command: Arc<str>,
    pub pids: Vec<u32>,
    pub usage_stats: AppUsageStats,
}

impl<'a> AppExt<'a> for LinuxApp {
    type Iter = std::slice::Iter<'a, u32>;

    fn name(&self) -> &str {
        self.name.as_ref()
    }

    fn icon(&self) -> Option<&str> {
        self.icon.as_ref().map(|s| s.as_ref())
    }

    fn id(&self) -> &str {
        self.id.as_ref()
    }

    fn command(&self) -> &str {
        self.command.as_ref()
    }

    fn pids(&'a self) -> Self::Iter {
        self.pids.iter()
    }

    fn usage_stats(&self) -> &AppUsageStats {
        &self.usage_stats
    }
}

pub struct LinuxApps {
    app_cache: Vec<LinuxApp>,
    refresh_timestamp: std::time::Instant,
}

impl LinuxApps {
    pub fn new() -> Self {
        Self {
            app_cache: vec![],
            refresh_timestamp: std::time::Instant::now()
                - (STALE_DELTA + std::time::Duration::from_millis(1)),
        }
    }

    fn extract_app_id(cgroup: &str) -> Option<Arc<str>> {
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
            Some(result.into())
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
                        Some(s.replace("\\x2d", "-").into())
                    }
                })
        }
    }

    fn update_running_apps(
        app: &mut LinuxApp,
        process: &LinuxProcess,
        running_apps: &mut std::collections::HashSet<Arc<str>>,
    ) {
        use crate::platform::ProcessExt;

        if running_apps.contains(&app.id) {
            app.pids.push(process.pid());
            app.usage_stats.merge(process.usage_stats());
        } else {
            app.pids.push(process.pid());
            app.usage_stats.merge(process.usage_stats());
            running_apps.insert(Arc::clone(&app.id));
        }
    }

    fn load_apps_from_dir<P: AsRef<std::path::Path>>(
        path: P,
        apps: &mut HashMap<Arc<str>, LinuxApp>,
    ) {
        use crate::critical;

        let path = path.as_ref();
        let dir = match std::fs::read_dir(path) {
            Ok(d) => d,
            Err(e) => {
                critical!(
                    "Gatherer::Apps",
                    "Failed to load apps from {}: {}",
                    path.display(),
                    e
                );
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
                        apps.insert(Arc::clone(&app.id), app);
                    }
                }
            }
        }
    }

    fn from_desktop_file<P: AsRef<std::path::Path>>(path: P) -> Option<LinuxApp> {
        use crate::critical;
        use ini::*;

        let path = path.as_ref();

        let app_id: Arc<str> = path
            .file_stem()
            .map(|s| s.to_string_lossy().into())
            .unwrap_or(Arc::from(""));

        if APP_IGNORELIST.contains(&app_id.as_ref()) {
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
                critical!(
                    "Gatherer::Apps",
                    "Failed to load desktop file from {}: {}",
                    path.display(),
                    e
                );
                return None;
            }
        };

        let section = match ini.section(Some("Desktop Entry")) {
            None => {
                critical!(
                    "Gatherer::Apps",
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
                critical!(
                    "Gatherer::Apps",
                    "Failed to load desktop file from {}: Invalid or corrupt file, \"Name\" key is missing",
                    path.display()
                );
                return None;
            }
            Some(n) => n,
        };

        let command = match section.get("Exec") {
            None => {
                critical!(
                    "Gatherer::Apps",
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

        Some(LinuxApp {
            name: name.into(),
            icon: section.get("Icon").map(|s| s.into()),
            id: app_id,
            command,
            pids: vec![],
            usage_stats: Default::default(),
        })
    }

    fn parse_command(command_line: &str) -> Option<Arc<str>> {
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
                            return Some(binary_name.into());
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
                    return Some(cmd_path.to_string_lossy().into());
                }
            }
        }

        None
    }
}

impl<'a> AppsExt<'a> for LinuxApps {
    type A = LinuxApp;
    type P = LinuxProcess;

    fn refresh_cache(&mut self, processes: &HashMap<u32, LinuxProcess>) {
        use crate::platform::ProcessExt;
        use std::collections::HashSet;

        self.app_cache.clear();

        let mut installed_apps = HashMap::new();
        for dir in &*XDG_DATA_DIRS {
            let dir = std::path::Path::new(dir);
            let dir = dir.join("applications");
            if dir.exists() {
                Self::load_apps_from_dir(dir, &mut installed_apps);
            }
        }

        let mut running_apps: HashSet<Arc<str>> = HashSet::new();

        let processes: Vec<_> = processes.iter().map(|(_, p)| p.clone()).collect();

        for process in &processes {
            match process
                .cgroup
                .as_ref()
                .and_then(|cgroup| Self::extract_app_id(cgroup.as_ref()))
            {
                None => {
                    for app in installed_apps.values_mut() {
                        if process.exe().starts_with(app.command()) {
                            Self::update_running_apps(app, process, &mut running_apps);
                            break;
                        }

                        if process.cmd().any(|cmd| cmd.starts_with(app.command())) {
                            Self::update_running_apps(app, process, &mut running_apps);
                            break;
                        }
                    }
                }
                Some(app_id) => {
                    if let Some(app) = installed_apps.get_mut(&app_id) {
                        Self::update_running_apps(app, process, &mut running_apps);
                    }
                }
            };
        }

        for app_id in running_apps.iter() {
            if let Some(app) = installed_apps.remove(app_id) {
                self.app_cache.push(app)
            }
        }

        self.refresh_timestamp = std::time::Instant::now();
    }

    fn is_cache_stale(&self) -> bool {
        std::time::Instant::now().duration_since(self.refresh_timestamp) > STALE_DELTA
    }

    fn app_list(&self) -> &[Self::A] {
        &self.app_cache
    }
}

#[cfg(test)]
mod test {
    use crate::platform::{AppsExt, Processes, ProcessesExt};

    use super::*;

    #[test]
    fn test_refresh_cache() {
        let mut p = Processes::new();
        p.refresh_cache();

        let mut apps = LinuxApps::new();
        assert!(apps.app_cache.is_empty());

        apps.refresh_cache(p.process_list());
        assert!(!apps.app_cache.is_empty());

        let sample = apps.app_cache.iter().take(10);
        dbg!(sample);
    }
}
