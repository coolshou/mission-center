include!("../common/process.rs");
include!("../common/util.rs");

pub fn load_app_and_process_list() -> (
    Vec<crate::sys_info_v2::App>,
    std::collections::HashMap<Pid, Process>,
) {
    use super::{App, CACHE_DIR, IS_FLATPAK};
    use gtk::glib::{g_critical, g_debug};
    use std::{
        collections::HashMap,
        io::{Cursor, Read},
    };

    let is_flatpak = *IS_FLATPAK;

    let mut processes = HashMap::new();
    let mut apps = vec![];

    let mut cmd = if is_flatpak {
        match std::fs::create_dir_all(CACHE_DIR.as_str()) {
            Err(err) => {
                g_critical!(
                    "MissionCenter::ProcInfo",
                    "Failed to load process information: {:?}",
                    err
                );
                return (apps, processes);
            }
            _ => {}
        }

        let proxy_bin_path = CACHE_DIR.clone() + "/missioncenter-proxy";

        match std::fs::copy("/app/bin/missioncenter-proxy", &proxy_bin_path) {
            Err(err) => {
                g_critical!(
                    "MissionCenter::ProcInfo",
                    "Failed to load process information: {:?}",
                    err
                );
                return (apps, processes);
            }
            _ => {}
        }

        cmd_flatpak_host!(&format!(
            "{} apps-processes --process-cache {}/proc_cache.bin",
            proxy_bin_path,
            CACHE_DIR.as_str()
        ))
    } else {
        cmd!(&format!(
            "missioncenter-proxy apps-processes --process-cache {}/proc_cache.bin",
            CACHE_DIR.as_str()
        ))
    };

    let output = cmd.output();
    if output.is_err() {
        g_critical!(
            "MissionCenter::ProcInfo",
            "Failed to load process information, failed to spawn proxy process: {}",
            output.err().unwrap()
        );
        return (apps, processes);
    }
    let output = output.unwrap();

    let stderr = String::from_utf8(output.stderr);
    if stderr.is_err() {
        g_critical!(
            "MissionCenter::ProcInfo",
            "Failed to load process information, failed to read stderr: {}",
            stderr.err().unwrap()
        );
        return (apps, processes);
    }
    let stderr = stderr.unwrap();

    for line in stderr.lines() {
        if line.starts_with("CRT") {
            g_critical!(
                "MissionCenter::ProcInfo",
                "{}",
                line.trim_start_matches("CRT")
            );
        } else if line.starts_with("DBG") {
            g_debug!(
                "MissionCenter::ProcInfo",
                "{}",
                line.trim_start_matches("DBG")
            );
        }
    }

    let mut cursor = Cursor::new(output.stdout);

    let mut process_count = 0_usize;
    cursor.read(to_binary_mut(&mut process_count)).unwrap();
    processes.reserve(process_count);
    for _ in 0..process_count {
        Process::deserialize(&mut cursor)
            .map(|process| {
                processes.insert(process.pid, process);
            })
            .unwrap();
    }

    let mut app_count = 0_usize;
    cursor.read(to_binary_mut(&mut app_count)).unwrap();
    apps.reserve(app_count);
    for _ in 0..app_count {
        App::deserialize(&mut cursor)
            .map(|app| {
                apps.push(app);
            })
            .unwrap();
    }

    (apps, processes)
}

pub fn process_hierarchy(processes: &std::collections::HashMap<Pid, Process>) -> Option<Process> {
    use gtk::glib::g_debug;
    use std::collections::*;

    let now = std::time::Instant::now();

    let pids = processes.keys().map(|pid| *pid).collect::<BTreeSet<_>>();
    if pids.len() == 0 {
        return None;
    }

    let root_process = processes
        .get(pids.first().unwrap())
        .map_or(None, |p| Some(p.clone()));
    if root_process.is_none() {
        return None;
    }
    let mut root_process = root_process.unwrap();

    let mut process_tree = BTreeMap::new();
    process_tree.insert(root_process.pid, 0_usize);

    let mut children = Vec::with_capacity(pids.len());
    children.push(HashMap::new());

    let mut visited = HashSet::new();
    visited.insert(root_process.pid);

    for pid in pids.iter().skip(1).rev() {
        if visited.contains(pid) {
            continue;
        }

        let process = processes.get(pid);
        if process.is_none() {
            continue;
        }
        let process = process.unwrap();

        let mut stack = vec![process];
        let mut parent = process.parent;
        while parent != 0 {
            let parent_process = processes.get(&parent);
            if parent_process.is_none() {
                break;
            }
            let parent_process = parent_process.unwrap();

            if visited.contains(&parent_process.pid) {
                let mut index = *process_tree.get(&parent_process.pid).unwrap();
                while let Some(ancestor) = stack.pop() {
                    let p = ancestor.clone();
                    children[index].insert(p.pid, p);

                    visited.insert(ancestor.pid);

                    index = children.len();
                    process_tree.insert(ancestor.pid, index);
                    children.push(HashMap::new());
                }

                break;
            }

            stack.push(parent_process);
            parent = parent_process.parent;
        }
    }

    fn gather_descendants(
        process: &mut Process,
        process_tree: &BTreeMap<Pid, usize>,
        children: &mut Vec<HashMap<Pid, Process>>,
    ) {
        let pid = process.pid;

        let index = match process_tree.get(&pid) {
            Some(index) => *index,
            None => return,
        };

        if children[index].is_empty() {
            return;
        }

        std::mem::swap(&mut process.children, &mut children[index]);

        for (_, child) in &mut process.children {
            gather_descendants(child, process_tree, children);
        }
    }

    let process = &mut root_process;
    std::mem::swap(&mut process.children, &mut children[0]);
    for (_, child) in &mut process.children {
        gather_descendants(child, &process_tree, &mut children);
    }

    g_debug!(
        "MissionCenter::ProcInfo",
        "[{}:{}] Loading process hierarchy took {}ms",
        file!(),
        line!(),
        now.elapsed().as_millis()
    );

    Some(root_process)
}
