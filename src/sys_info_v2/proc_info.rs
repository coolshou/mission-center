pub type Pid = libc::pid_t;

#[derive(Debug, Clone, Default)]
pub struct Process {
    pub name: String,
    pub cmd: Vec<String>,
    pub exe: std::path::PathBuf,
    pub pid: Pid,
    pub parent: Option<Pid>,
    pub children: Vec<Process>,
}

impl Process {
    pub fn process_hierarchy(system: &mut sysinfo::System) -> Option<Process> {
        use gtk::glib::g_debug;
        use std::collections::*;
        use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, SystemExt};

        type SIPid = sysinfo::Pid;

        system.refresh_processes_specifics(ProcessRefreshKind::everything());
        let processes = system.processes();

        let now = std::time::Instant::now();

        let pids = processes.keys().map(|pid| *pid).collect::<BTreeSet<_>>();
        let root_process = processes.get(pids.first().unwrap()).map_or(None, |p| {
            Some(Process {
                name: p.name().to_owned(),
                cmd: p.cmd().iter().map(|s| s.to_owned()).collect::<Vec<_>>(),
                exe: p.exe().to_owned(),
                pid: p.pid().as_u32() as libc::pid_t,
                parent: None,
                children: vec![],
            })
        });
        if root_process.is_none() {
            return None;
        }
        let mut root_process = root_process.unwrap();

        let mut process_tree = BTreeMap::new();
        process_tree.insert(SIPid::from(root_process.pid as usize), 0_usize);

        let mut children = Vec::with_capacity(pids.len());
        children.push(vec![]);

        let mut visited = HashSet::new();
        visited.insert(SIPid::from(root_process.pid as usize));

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
            let mut parent = process.parent();
            while parent.is_some() {
                let parent_process = processes.get(&parent.unwrap());
                if parent_process.is_none() {
                    break;
                }
                let parent_process = parent_process.unwrap();

                if visited.contains(&parent_process.pid()) {
                    let mut index = *process_tree.get(&parent_process.pid()).unwrap();
                    while let Some(a_parent) = stack.pop() {
                        let p = Process {
                            name: a_parent.name().to_owned(),
                            cmd: a_parent
                                .cmd()
                                .iter()
                                .map(|s| s.to_owned())
                                .collect::<Vec<_>>(),
                            exe: a_parent.exe().to_owned(),
                            pid: a_parent.pid().as_u32() as libc::pid_t,
                            parent: Some(parent_process.pid().as_u32() as libc::pid_t),
                            children: vec![],
                        };
                        children[index].push(p);

                        visited.insert(a_parent.pid());

                        index = children.len();
                        process_tree.insert(a_parent.pid(), index);
                        children.push(vec![]);
                    }

                    break;
                }

                stack.push(parent_process.clone());
                parent = parent_process.parent();
            }
        }

        fn gather_descendants(
            process: &mut Process,
            process_tree: &BTreeMap<SIPid, usize>,
            children: &mut Vec<Vec<Process>>,
        ) {
            let pid = SIPid::from(process.pid as usize);

            let index = match process_tree.get(&pid) {
                Some(index) => *index,
                None => return,
            };

            if children[index].is_empty() {
                return;
            }

            std::mem::swap(&mut process.children, &mut children[index]);

            for child in &mut process.children {
                gather_descendants(child, process_tree, children);
            }
        }

        let process = &mut root_process;
        std::mem::swap(&mut process.children, &mut children[0]);
        for child in &mut process.children {
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
}
