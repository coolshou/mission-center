use super::Process;

const APP_BLACKLIST: &[&'static str] = &["fish", "Fish", "Guake Preferences"];

#[derive(Debug, Default, Clone)]
pub struct App {
    base: super::gatherer::AppDescriptor,
    pub pids: Vec<u32>,
    pub stats: Stats,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Stats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    pub gpu_usage: f32,
}

impl App {
    pub fn new(base: super::gatherer::AppDescriptor) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.base.name
    }

    #[inline]
    pub fn commands(&self) -> &[arrayvec::ArrayString<128>] {
        &self.base.commands
    }

    #[inline]
    pub fn icon(&self) -> Option<&str> {
        self.base.icon.as_deref()
    }

    #[inline]
    pub fn app_id(&self) -> Option<&str> {
        self.base.app_id.as_deref()
    }

    #[inline]
    pub fn is_flatpak(&self) -> bool {
        self.base.is_flatpak
    }
}

impl super::GathererSupervisor {
    pub fn installed_apps(&mut self) -> Vec<App> {
        use super::gatherer::SharedDataContent;
        use gtk::glib::*;

        let mut result = vec![];

        self.execute(
            super::gatherer::Message::GetInstalledApps,
            |gatherer, process_restarted| {
                let shared_memory = match gatherer.shared_memory() {
                    Ok(shm) => shm,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Unable to to access shared memory: {}",
                            e
                        );
                        return false;
                    }
                };

                match shared_memory.content {
                    SharedDataContent::InstalledApps(ref apps) => {
                        if process_restarted {
                            result.clear();
                        }

                        for app in &apps.apps {
                            result.push(App::new(app.clone()));
                        }
                        apps.is_complete
                    }
                    SharedDataContent::Processes(_) => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is Processes instead of InstalledApps; encountered when reading installed apps from gatherer", 
                        );
                        false
                    }
                    SharedDataContent::Monostate => {
                        g_critical!(
                            "MissionCenter::AppInfo",
                            "Shared data content is Monostate instead of InstalledApps; encountered when reading installed apps from gatherer", 
                        );
                        false
                    }
                }
            },
        );

        result
    }
}

pub fn running_apps(
    root_process: &Process,
    apps: &[App],
) -> std::collections::HashMap<String, App> {
    use gtk::glib::*;
    use std::collections::HashMap;

    fn find_app<'a>(
        app: &App,
        // Iterating over these consumes the iterator, and we need to iterate over them twice
        processes_once: impl IntoIterator<Item = &'a Process>,
        processes_again: impl IntoIterator<Item = &'a Process>,
        result: &mut HashMap<String, App>,
    ) {
        fn update_or_insert_app(app: &App, process: &Process, app_list: &mut HashMap<String, App>) {
            if let Some(app) = app_list.get_mut(app.name()) {
                app.pids.push(process.pid());
                app.stats.cpu_usage += process.stats().cpu_usage;
                app.stats.memory_usage += process.stats().memory_usage;
                app.stats.disk_usage += process.stats().disk_usage;
                app.stats.network_usage += process.stats().network_usage;
                app.stats.gpu_usage += process.stats().gpu_usage;
            } else {
                let mut app = app.clone();
                app.pids.push(process.pid());
                app.stats.cpu_usage = process.stats().cpu_usage;
                app.stats.memory_usage = process.stats().memory_usage;
                app.stats.disk_usage = process.stats().disk_usage;
                app.stats.network_usage = process.stats().network_usage;
                app.stats.gpu_usage = process.stats().gpu_usage;
                app_list.insert(app.name().to_string(), app);
            }
        }

        let mut found = false;
        for process in processes_once {
            if app.is_flatpak() {
                if process.name() == "bwrap" {
                    for arg in process.cmd() {
                        for command in app.commands() {
                            if arg.contains(command.as_str()) {
                                update_or_insert_app(app, process, result);
                                found = true;
                            }
                        }
                    }
                }
            } else {
                for command in app.commands() {
                    if process.exe() == command.as_str() {
                        update_or_insert_app(app, process, result);
                        found = true;
                        break;
                    }
                }
                if !found {
                    let mut iter = process.cmd().iter();
                    if let Some(cmd) = iter.next() {
                        for command in app.commands() {
                            if cmd.ends_with(command.as_str()) {
                                update_or_insert_app(app, process, result);
                                found = true;
                                break;
                            }
                        }

                        if !found {
                            // The app might use a runtime (bash, python, node, mono, dotnet, etc.) so check the second argument
                            if let Some(cmd) = iter.next() {
                                for command in app.commands() {
                                    if cmd.ends_with(command.as_str()) {
                                        update_or_insert_app(app, process, result);
                                        found = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if found {
            return;
        }

        for processes in processes_again {
            find_app(
                app,
                processes.children.values(),
                processes.children.values(),
                result,
            );
        }
    }

    let start = std::time::Instant::now();

    let mut running_apps = HashMap::new();
    for app in apps {
        if APP_BLACKLIST.contains(&app.name()) {
            continue;
        }

        find_app(
            app,
            root_process.children.values(),
            root_process.children.values(),
            &mut running_apps,
        );
    }

    g_debug!(
        "MissionCenter::AppInfo",
        "[{}:{}] Found running apps in {}ms",
        file!(),
        line!(),
        start.elapsed().as_millis()
    );

    return running_apps;
}
