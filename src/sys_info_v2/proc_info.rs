pub type Pid = libc::pid_t;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Stats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    pub gpu_usage: f32,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            cpu_usage: 0.,
            memory_usage: 0.,
            disk_usage: 0.,
            network_usage: 0.,
            gpu_usage: 0.,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ProcessState {
    Running,
    Sleeping,
    SleepingUninterruptible,
    Zombie,
    Stopped,
    Tracing,
    Dead,
    Wakekill,
    Waking,
    Parked,
    Unknown,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Default)]
pub struct Process {
    pub name: String,
    pub cmd: Vec<String>,
    pub exe: std::path::PathBuf,
    pub state: ProcessState,
    pub pid: Pid,

    #[allow(dead_code)]
    pub parent: Pid,
    pub children: std::collections::HashMap<Pid, Process>,

    pub process_stats: Stats,
}

fn to_binary_mut<T: Sized>(thing: &mut T) -> &mut [u8] {
    let ptr = thing as *mut T;
    unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, core::mem::size_of::<T>()) }
}

pub fn load_process_list() -> std::collections::HashMap<Pid, Process> {
    use super::{CACHE_DIR, IS_FLATPAK};
    use gtk::glib::g_critical;
    use std::{
        collections::HashMap,
        io::{Cursor, Read},
    };

    let is_flatpak = *IS_FLATPAK;

    let mut result = HashMap::new();

    let mut cmd = if is_flatpak {
        match std::fs::create_dir_all(CACHE_DIR.as_str()) {
            Err(err) => {
                g_critical!(
                    "MissionCenter::ProcInfo",
                    "Failed to load process information: {:?}",
                    err
                );
                return result;
            }
            _ => {}
        }

        let proxy_bin_path = CACHE_DIR.clone() + "/missioncenter_proxy";

        match std::fs::copy("/app/bin/missioncenter_proxy", &proxy_bin_path) {
            Err(err) => {
                g_critical!(
                    "MissionCenter::ProcInfo",
                    "Failed to load process information: {:?}",
                    err
                );
                return result;
            }
            _ => {}
        }

        cmd_flatpak_host!(&format!(
            "{} --process-cache {}/proc_cache.bin",
            proxy_bin_path,
            CACHE_DIR.as_str()
        ))
    } else {
        cmd!(&format!(
            "missioncenter_proxy --process-cache {}/proc_cache.bin",
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
        return result;
    }
    let output = output.unwrap();
    let mut cursor = Cursor::new(output.stdout);

    let mut process_count = 0_usize;
    cursor.read(to_binary_mut(&mut process_count)).unwrap();
    result.reserve(process_count);
    for _ in 0..process_count {
        Process::deserialize(&mut cursor)
            .map(|process| {
                result.insert(process.pid, process);
            })
            .unwrap();
    }

    result
}

impl Process {
    pub fn process_hierarchy(
        processes: &std::collections::HashMap<Pid, Process>,
    ) -> Option<Process> {
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

    fn deserialize<R: std::io::Read>(input: &mut R) -> std::io::Result<Self> {
        let mut process = Self::default();

        let mut len = 0;

        input.read_exact(to_binary_mut(&mut len))?;
        let mut name = vec![0; len];
        input.read_exact(&mut name)?;
        process.name = unsafe { String::from_utf8_unchecked(name) };

        input.read_exact(to_binary_mut(&mut len))?;
        for _ in 0..len {
            input.read_exact(to_binary_mut(&mut len))?;
            let mut arg = vec![0; len];
            input.read_exact(&mut arg)?;
            process
                .cmd
                .push(unsafe { String::from_utf8_unchecked(arg) });
        }

        input.read_exact(to_binary_mut(&mut len))?;
        let mut exe = vec![0; len];
        input.read_exact(&mut exe)?;
        process.exe = std::path::PathBuf::from(unsafe { String::from_utf8_unchecked(exe) });

        let mut state = 0_u8;
        input.read_exact(to_binary_mut(&mut state))?;
        process.state = match state {
            0 => ProcessState::Running,
            1 => ProcessState::Sleeping,
            2 => ProcessState::SleepingUninterruptible,
            3 => ProcessState::Zombie,
            4 => ProcessState::Stopped,
            5 => ProcessState::Tracing,
            6 => ProcessState::Dead,
            7 => ProcessState::Wakekill,
            8 => ProcessState::Waking,
            9 => ProcessState::Parked,
            _ => ProcessState::Unknown,
        };

        input.read_exact(to_binary_mut(&mut process.pid))?;

        input.read_exact(to_binary_mut(&mut process.parent))?;

        input.read_exact(to_binary_mut(&mut len))?;
        for _ in 0..len {
            let child = Process::deserialize(input)?;
            process.children.insert(child.pid, child);
        }

        input.read_exact(to_binary_mut(&mut process.process_stats.cpu_usage))?;
        input.read_exact(to_binary_mut(&mut process.process_stats.memory_usage))?;
        input.read_exact(to_binary_mut(&mut process.process_stats.disk_usage))?;
        input.read_exact(to_binary_mut(&mut process.process_stats.network_usage))?;
        input.read_exact(to_binary_mut(&mut process.process_stats.gpu_usage))?;

        Ok(process)
    }
}
