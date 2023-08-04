#[derive(Debug, Clone)]
pub struct Process {
    base: super::gatherer::ProcessDescriptor,
    pub children: std::collections::HashMap<u32, Process>,
}

impl Default for Process {
    fn default() -> Self {
        Self {
            base: super::gatherer::ProcessDescriptor {
                name: arrayvec::ArrayString::new(),
                cmd: Default::default(),
                exe: arrayvec::ArrayString::new(),
                state: super::gatherer::ProcessState::Unknown,
                pid: 0,
                parent: 0,
                stats: Default::default(),
            },
            children: Default::default(),
        }
    }
}

impl Process {
    pub fn new(base: super::gatherer::ProcessDescriptor) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    pub fn name(&self) -> &str {
        &self.base.name
    }

    pub fn cmd(&self) -> &[arrayvec::ArrayString<128>] {
        &self.base.cmd
    }

    pub fn exe(&self) -> &str {
        &self.base.exe
    }

    pub fn state(&self) -> super::gatherer::ProcessState {
        self.base.state
    }

    pub fn pid(&self) -> u32 {
        self.base.pid
    }

    pub fn parent(&self) -> u32 {
        self.base.parent
    }

    pub fn stats(&self) -> &super::gatherer::ProcessStats {
        &self.base.stats
    }

    pub fn stats_mut(&mut self) -> &mut super::gatherer::ProcessStats {
        &mut self.base.stats
    }
}

impl super::GathererSupervisor {
    pub fn processes(&mut self) -> std::collections::HashMap<u32, Process> {
        use super::gatherer::SharedDataContent;
        use gtk::glib::*;
        use std::collections::HashMap;

        let mut result = HashMap::new();

        self.execute(
            super::gatherer::Message::GetProcesses,
            |gatherer, process_restarted| match gatherer.shared_memory().unwrap().content {
                SharedDataContent::Processes(ref proceses) => {
                    if process_restarted {
                        result.clear();
                    }

                    for proc in &proceses.processes {
                        result.insert(proc.pid, Process::new(proc.clone()));
                    }
                    proceses.is_complete
                }
                SharedDataContent::InstalledApps(_) => {
                    g_critical!(
                        "MissionCenter::ProcInfo",
                        "Shared data content is InstalledApps instead of Processes; encountered when reading processes from gatherer", 
                    );
                    false
                }
                SharedDataContent::Monostate => {
                    g_critical!(
                        "MissionCenter::ProcInfo",
                        "Shared data content is Monostate instead of Processes; encountered when reading processes from gatherer", 
                    );
                    false
                }
            },
        );

        result
    }
}

pub fn process_hierarchy(processes: &std::collections::HashMap<u32, Process>) -> Option<Process> {
    use super::gatherer::ProcessStats;
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
    process_tree.insert(root_process.pid(), 0_usize);

    let mut children = Vec::with_capacity(pids.len());
    children.push(HashMap::new());

    let mut visited = HashSet::new();
    visited.insert(root_process.pid());

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
        while parent != 0 {
            let parent_process = processes.get(&parent);
            if parent_process.is_none() {
                break;
            }
            let parent_process = parent_process.unwrap();

            if visited.contains(&parent_process.pid()) {
                let mut index = *process_tree.get(&parent_process.pid()).unwrap();
                while let Some(ancestor) = stack.pop() {
                    let p = ancestor.clone();
                    children[index].insert(p.pid(), p);

                    visited.insert(ancestor.pid());

                    index = children.len();
                    process_tree.insert(ancestor.pid(), index);
                    children.push(HashMap::new());
                }

                break;
            }

            stack.push(parent_process);
            parent = parent_process.parent();
        }
    }

    fn gather_descendants(
        process: &mut Process,
        process_tree: &BTreeMap<u32, usize>,
        children: &mut Vec<HashMap<u32, Process>>,
    ) {
        let pid = process.pid();

        let index = match process_tree.get(&pid) {
            Some(index) => *index,
            None => return,
        };

        if children[index].is_empty() {
            return;
        }

        std::mem::swap(&mut process.children, &mut children[index]);

        let mut process_stats = ProcessStats::default();
        for (_, child) in &mut process.children {
            gather_descendants(child, process_tree, children);
            process_stats.merge(&child.stats());
        }
        process.stats_mut().merge(&process_stats);
    }

    let process = &mut root_process;
    std::mem::swap(&mut process.children, &mut children[0]);

    let mut process_stats = ProcessStats::default();
    for (_, child) in &mut process.children {
        gather_descendants(child, &process_tree, &mut children);
        process_stats.merge(&child.stats());
    }
    process.stats_mut().merge(&process_stats);

    g_debug!(
        "MissionCenter::ProcInfo",
        "[{}:{}] Loading process hierarchy took {}ms",
        file!(),
        line!(),
        now.elapsed().as_millis()
    );

    Some(root_process)
}
