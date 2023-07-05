include!("../common/util.rs");
include!("../common/app.rs");

const APP_BLACKLIST: &[&'static str] = &["fish", "Fish", "Guake Preferences"];

pub fn running_apps(
    root_process: &crate::sys_info_v2::proc_info::Process,
    apps: &[App],
) -> std::collections::HashMap<String, App> {
    use crate::sys_info_v2::proc_info::*;
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
            if let Some(app) = app_list.get_mut(&app.name) {
                app.pids.push(process.pid);
                app.stats.cpu_usage += process.stats.cpu_usage;
                app.stats.memory_usage += process.stats.memory_usage;
                app.stats.disk_usage += process.stats.disk_usage;
                app.stats.network_usage += process.stats.network_usage;
                app.stats.gpu_usage += process.stats.gpu_usage;
            } else {
                let mut app = app.clone();
                app.pids.push(process.pid);
                app.stats.cpu_usage = process.stats.cpu_usage;
                app.stats.memory_usage = process.stats.memory_usage;
                app.stats.disk_usage = process.stats.disk_usage;
                app.stats.network_usage = process.stats.network_usage;
                app.stats.gpu_usage = process.stats.gpu_usage;
                app_list.insert(app.name.clone(), app);
            }
        }

        let mut found = false;
        for process in processes_once {
            if app.is_flatpak {
                if process.name == "bwrap" {
                    for arg in &process.cmd {
                        if arg.contains(&app.command) {
                            update_or_insert_app(app, process, result);
                            found = true;
                        }
                    }
                }
            } else {
                if process.exe == std::path::Path::new(&app.command) {
                    update_or_insert_app(app, process, result);
                    found = true;
                } else {
                    let mut iter = process.cmd.iter();
                    if let Some(cmd) = iter.next() {
                        if cmd.ends_with(&app.command) {
                            update_or_insert_app(app, process, result);
                            found = true;
                        }
                        // The app might use a runtime (bash, python, node, mono, dotnet, etc.) so check the second argument
                        else if let Some(cmd) = iter.next() {
                            if cmd.ends_with(&app.command) {
                                update_or_insert_app(app, process, result);
                                found = true;
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
        if APP_BLACKLIST.contains(&app.name.as_str()) {
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
