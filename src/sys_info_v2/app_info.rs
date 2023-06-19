include!("../common/util.rs");
include!("../common/app.rs");

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
                return;
            } else {
                let mut app = app.clone();
                app.pids.push(process.pid);
                app_list.insert(app.name.clone(), app);
            }
        }

        for process in processes_once {
            if app.is_flatpak {
                if process.name == "bwrap" {
                    for arg in &process.cmd {
                        if arg.contains(&app.command) {
                            update_or_insert_app(app, process, result);
                            return;
                        }
                    }
                }
            } else {
                if process.exe == std::path::Path::new(&app.command) {
                    update_or_insert_app(app, process, result);
                    return;
                } else {
                    if let Some(cmd) = process.cmd.first() {
                        if cmd.ends_with(&app.command) {
                            update_or_insert_app(app, process, result);
                            return;
                        }
                    }
                }
            }
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
