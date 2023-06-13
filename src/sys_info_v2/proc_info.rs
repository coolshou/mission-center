pub type Pid = libc::pid_t;

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ProcessStats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    pub gpu_usage: f32,

    mem_used_bytes: u64,

    disk_read_bytes: u64,
    disk_write_bytes: u64,

    net_bytes_sent: u64,
    net_bytes_recv: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Process {
    pub name: String,
    pub cmd: Vec<String>,
    pub exe: std::path::PathBuf,
    pub pid: Pid,

    pub parent: Option<Pid>,
    pub children: std::collections::HashMap<Pid, Process>,

    pub process_stats: ProcessStats,
}

impl Process {
    pub fn process_hierarchy(system: &mut sysinfo::System) -> Option<Process> {
        use gtk::glib::g_debug;
        use rand::Rng;
        use std::collections::*;
        use sysinfo::{PidExt, ProcessExt, ProcessRefreshKind, SystemExt};

        type SIPid = sysinfo::Pid;

        system.refresh_processes_specifics(ProcessRefreshKind::everything());
        let processes = system.processes();

        let now = std::time::Instant::now();

        let mut rng = rand::thread_rng();

        let pids = processes.keys().map(|pid| *pid).collect::<BTreeSet<_>>();
        let root_process = processes.get(pids.first().unwrap()).map_or(None, |p| {
            Some(Process {
                name: p.name().to_owned(),
                cmd: p.cmd().iter().map(|s| s.to_owned()).collect::<Vec<_>>(),
                exe: p.exe().to_owned(),
                pid: p.pid().as_u32() as libc::pid_t,

                parent: None,
                children: HashMap::new(),

                process_stats: ProcessStats {
                    cpu_usage: p.cpu_usage(),
                    memory_usage: rng.gen_range(0.0..34359738368.0),
                    disk_usage: rng.gen_range(0.0..34359738368.0),
                    network_usage: rng.gen_range(0.0..34359738368.0),
                    gpu_usage: rng.gen_range(0.0..100.0),

                    mem_used_bytes: 0,

                    disk_read_bytes: 0,
                    disk_write_bytes: 0,

                    net_bytes_sent: 0,
                    net_bytes_recv: 0,
                },
            })
        });
        if root_process.is_none() {
            return None;
        }
        let mut root_process = root_process.unwrap();

        let mut process_tree = BTreeMap::new();
        process_tree.insert(SIPid::from(root_process.pid as usize), 0_usize);

        let mut children = Vec::with_capacity(pids.len());
        children.push(HashMap::new());

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
                    let mut parent_pid = parent_process.pid().as_u32() as Pid;
                    while let Some(ancestor) = stack.pop() {
                        let p = Process {
                            name: ancestor.name().to_owned(),
                            cmd: ancestor
                                .cmd()
                                .iter()
                                .map(|s| s.to_owned())
                                .collect::<Vec<_>>(),
                            exe: ancestor.exe().to_owned(),
                            pid: ancestor.pid().as_u32() as Pid,

                            parent: Some(parent_pid),
                            children: HashMap::new(),

                            process_stats: ProcessStats {
                                cpu_usage: ancestor.cpu_usage(),
                                memory_usage: rng.gen_range(0.0..34359738368.0),
                                disk_usage: rng.gen_range(0.0..34359738368.0),
                                network_usage: rng.gen_range(0.0..34359738368.0),
                                gpu_usage: rng.gen_range(0.0..100.0),

                                mem_used_bytes: 0,

                                disk_read_bytes: 0,
                                disk_write_bytes: 0,

                                net_bytes_sent: 0,
                                net_bytes_recv: 0,
                            },
                        };
                        parent_pid = p.pid;
                        children[index].insert(p.pid, p);

                        visited.insert(ancestor.pid());

                        index = children.len();
                        process_tree.insert(ancestor.pid(), index);
                        children.push(HashMap::new());
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
            children: &mut Vec<HashMap<Pid, Process>>,
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
}
