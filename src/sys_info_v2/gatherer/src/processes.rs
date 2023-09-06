/* sys_info_v2/gatherer/src/processes.rs
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

use std::collections::HashMap;

use lazy_static::lazy_static;

mod state {
    use std::{cell::Cell, collections::HashMap, thread_local};

    use super::*;

    thread_local! {
        pub static PERSISTENT_PROCESS_CACHE: Cell<HashMap<u32, Process>> = Cell::new(HashMap::new());
        pub static PROCESS_HIERARCHY: Cell<Process> = Cell::new(Process::default());
        pub static PROCESS_CACHE: Cell<Vec<ProcessDescriptor>> = Cell::new(vec![]);
    }
}

lazy_static! {
    static ref PAGE_SIZE: usize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
    static ref HZ: usize = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as usize };
}

const PROC_PID_STAT_TCOMM: usize = 1;
const PROC_PID_STAT_STATE: usize = 2;
const PROC_PID_STAT_PPID: usize = 3;
const PROC_PID_STAT_UTIME: usize = 13;
const PROC_PID_STAT_STIME: usize = 14;

#[allow(dead_code)]
const PROC_PID_STATM_VIRT: usize = 0;
const PROC_PID_STATM_RES: usize = 1;

const PROC_PID_IO_READ_BYTES: usize = 4;
const PROC_PID_IO_WRITE_BYTES: usize = 5;

#[allow(dead_code)]
const PROC_PID_NET_DEV_RECV_BYTES: usize = 0;
#[allow(dead_code)]
const PROC_PID_NET_DEV_SENT_BYTES: usize = 8;

include!("../common/processes.rs");

impl Default for ProcessState {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Copy, Clone)]
struct RawStats {
    pub user_jiffies: u64,
    pub kernel_jiffies: u64,

    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,

    pub net_bytes_sent: u64,
    pub net_bytes_recv: u64,

    pub timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct Process {
    pub descriptor: ProcessDescriptor,
    raw_stats: RawStats,
    pub children: Vec<Process>,
    pub cgroup: Option<String>,
    pub task_count: usize,
}

impl Default for Process {
    fn default() -> Self {
        Self {
            descriptor: ProcessDescriptor {
                name: ArrayString::new(),
                cmd: ArrayVec::new(),
                exe: ArrayString::new(),
                state: ProcessState::Unknown,
                pid: 0,
                parent: 0,
                stats: Stats {
                    cpu_usage: 0.0,
                    memory_usage: 0.0,
                    disk_usage: 0.0,
                    network_usage: 0.0,
                    gpu_usage: 0.0,
                },
            },
            raw_stats: RawStats {
                user_jiffies: 0,
                kernel_jiffies: 0,
                disk_read_bytes: 0,
                disk_write_bytes: 0,
                net_bytes_sent: 0,
                net_bytes_recv: 0,
                timestamp: std::time::Instant::now(),
            },
            children: vec![],
            cgroup: None,
            task_count: 0,
        }
    }
}

impl Default for Processes {
    fn default() -> Self {
        Self {
            processes: ArrayVec::new(),
            is_complete: false,
        }
    }
}

impl Processes {
    pub fn new() -> Self {
        let mut this = Self::default();

        let process_cache = state::PROCESS_CACHE.with(|state| unsafe { &mut *state.as_ptr() });
        if process_cache.is_empty() {
            Self::update_process_cache();
            let full_process_cache =
                state::PERSISTENT_PROCESS_CACHE.with(|state| unsafe { &*state.as_ptr() });
            for (_, p) in full_process_cache {
                process_cache.push(p.descriptor.clone());
            }
        }

        let drop_count = process_cache
            .chunks(this.processes.capacity())
            .next()
            .unwrap_or(&[])
            .len();

        let it = process_cache.drain(0..drop_count);
        this.processes.extend(it);
        this.is_complete = process_cache.is_empty();

        this
    }

    pub fn process_cache() -> &'static HashMap<u32, Process> {
        state::PERSISTENT_PROCESS_CACHE.with(|state| unsafe { &*state.as_ptr() })
    }

    #[allow(dead_code)]
    pub fn process_hierarchy() -> Option<Process> {
        use std::collections::*;

        let processes = state::PERSISTENT_PROCESS_CACHE.with(|state| unsafe { &*state.as_ptr() });

        let pids = processes.keys().map(|pid| *pid).collect::<BTreeSet<_>>();
        let root_pid = match pids.first() {
            None => return None,
            Some(pid) => *pid,
        };

        let mut root_process = match processes.get(&root_pid).map_or(None, |p| Some(p.clone())) {
            None => return None,
            Some(p) => p,
        };

        let mut process_tree = BTreeMap::new();
        process_tree.insert(root_process.descriptor.pid, 0_usize);

        let mut children = Vec::with_capacity(pids.len());
        children.push(vec![]);

        let mut visited = HashSet::new();
        visited.insert(root_process.descriptor.pid);

        for pid in pids.iter().skip(1).rev() {
            if visited.contains(pid) {
                continue;
            }

            let process = match processes.get(pid) {
                None => continue,
                Some(p) => p,
            };

            let mut stack = vec![process];
            let mut parent = process.descriptor.parent;
            while parent != 0 {
                let parent_process = match processes.get(&parent) {
                    None => break,
                    Some(pp) => pp,
                };

                if visited.contains(&parent_process.descriptor.pid) {
                    let mut index = match process_tree.get(&parent_process.descriptor.pid) {
                        None => {
                            // TODO: Fully understand if this could happen, and what to do if it does.
                            eprintln!(
                                "Gatherer: Process {} has been visited, but it's not in the process_tree?",
                                process.descriptor.pid
                            );
                            break;
                        }
                        Some(index) => *index,
                    };
                    while let Some(ancestor) = stack.pop() {
                        let p = ancestor.clone();
                        children[index].push(p);

                        visited.insert(ancestor.descriptor.pid);

                        index = children.len();
                        process_tree.insert(ancestor.descriptor.pid, index);
                        children.push(vec![]);
                    }

                    break;
                }

                stack.push(parent_process);
                parent = parent_process.descriptor.parent;
            }
        }

        fn gather_descendants(
            process: &mut Process,
            process_tree: &BTreeMap<u32, usize>,
            children: &mut Vec<Vec<Process>>,
        ) {
            let pid = process.descriptor.pid;

            let index = match process_tree.get(&pid) {
                Some(index) => *index,
                None => return,
            };

            if children[index].is_empty() {
                return;
            }

            std::mem::swap(&mut process.children, &mut children[index]);

            let mut process_stats = Stats::default();
            for child in &mut process.children {
                gather_descendants(child, process_tree, children);
                process_stats.merge(&child.descriptor.stats);
            }
            process.descriptor.stats.merge(&process_stats);
        }

        let process = &mut root_process;
        std::mem::swap(&mut process.children, &mut children[0]);

        let mut process_stats = Stats::default();
        for child in &mut process.children {
            gather_descendants(child, &process_tree, &mut children);
            process_stats.merge(&child.descriptor.stats);
        }
        process.descriptor.stats.merge(&process_stats);

        Some(root_process)
    }

    fn update_process_cache() {
        use super::ToArrayStringLossy;

        fn parse_stat_file<'a>(data: &'a str, output: &mut [&'a str; 52]) {
            let mut part_index = 0;

            let mut split = data.split('(').filter(|x| !x.is_empty());
            output[part_index] = match split.next() {
                Some(x) => x,
                None => return,
            };
            part_index += 1;

            let mut split = match split.next() {
                Some(x) => x.split(')').filter(|x| !x.is_empty()),
                None => return,
            };

            output[part_index] = match split.next() {
                Some(x) => x,
                None => return,
            };
            part_index += 1;

            let split = match split.next() {
                Some(x) => x,
                None => return,
            };
            for entry in split.split_whitespace() {
                output[part_index] = entry;
                part_index += 1;
            }
        }

        fn parse_statm_file(data: &str, output: &mut [u64; 7]) {
            let mut part_index = 0;

            for entry in data.split_whitespace() {
                output[part_index] = entry.trim().parse::<u64>().unwrap_or(0);
                part_index += 1;
            }
        }

        fn parse_io_file(data: &str, output: &mut [u64; 7]) {
            let mut part_index = 0;

            for entry in data.lines() {
                let entry = entry.split_whitespace().last().unwrap_or("");
                output[part_index] = entry.trim().parse::<u64>().unwrap_or(0);
                part_index += 1;
            }
        }

        fn stat_name(stat: &[&str; 52]) -> ArrayString {
            stat[PROC_PID_STAT_TCOMM].to_array_string_lossy()
        }

        fn stat_state(stat: &[&str; 52]) -> ProcessState {
            match stat[PROC_PID_STAT_STATE] {
                "R" => ProcessState::Running,
                "S" => ProcessState::Sleeping,
                "D" => ProcessState::SleepingUninterruptible,
                "Z" => ProcessState::Zombie,
                "T" => ProcessState::Stopped,
                "t" => ProcessState::Tracing,
                "X" | "x" => ProcessState::Dead,
                "K" => ProcessState::WakeKill,
                "W" => ProcessState::Waking,
                "P" => ProcessState::Parked,
                _ => ProcessState::Unknown,
            }
        }

        fn stat_parent_pid(stat: &[&str; 52]) -> u32 {
            stat[PROC_PID_STAT_PPID].parse::<u32>().unwrap_or(0)
        }

        fn stat_user_mode_jiffies(stat: &[&str; 52]) -> u64 {
            stat[PROC_PID_STAT_UTIME].parse::<u64>().unwrap_or(0)
        }

        fn stat_kernel_mode_jiffies(stat: &[&str; 52]) -> u64 {
            stat[PROC_PID_STAT_STIME].parse::<u64>().unwrap_or(0)
        }

        let mut previous = state::PERSISTENT_PROCESS_CACHE.with(|prev| prev.take());
        let result = state::PERSISTENT_PROCESS_CACHE.with(|prev| unsafe { &mut *prev.as_ptr() });
        result.reserve(previous.len());

        let now = std::time::Instant::now();

        let proc = match std::fs::read_dir("/proc") {
            Ok(proc) => proc,
            Err(e) => {
                eprintln!("Failed to read /proc directory: {}", e);
                return;
            }
        };
        let proc_entries = proc
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false));
        for entry in proc_entries {
            let pid = match entry.file_name().to_string_lossy().parse::<u32>() {
                Ok(pid) => pid,
                Err(_) => continue,
            };

            let entry_path = entry.path();

            let stat_file_content = match std::fs::read_to_string(entry_path.join("stat")) {
                Ok(sfc) => {
                    if sfc.is_empty() {
                        eprintln!(
                            "Failed to read stat information for process {}, skipping",
                            pid
                        );
                        continue;
                    }

                    sfc
                }
                Err(e) => {
                    eprintln!(
                        "Failed to read stat information for process {}, skipping: {}",
                        pid, e,
                    );
                    continue;
                }
            };
            let mut stat_parsed = [""; 52];
            parse_stat_file(&stat_file_content, &mut stat_parsed);

            let utime = stat_user_mode_jiffies(&stat_parsed);
            let stime = stat_kernel_mode_jiffies(&stat_parsed);

            let mut io_parsed = [0; 7];
            match std::fs::read_to_string(entry_path.join("io")) {
                Ok(output) => {
                    parse_io_file(&output, &mut io_parsed);
                }
                _ => {}
            }

            let total_net_sent = 0_u64;
            let total_net_recv = 0_u64;

            let mut process = match previous.remove(&pid) {
                None => Process::default(),
                Some(mut process) => {
                    let delta_time = now - process.raw_stats.timestamp;

                    let prev_utime = process.raw_stats.user_jiffies;
                    let prev_stime = process.raw_stats.kernel_jiffies;

                    let delta_utime =
                        ((utime.saturating_sub(prev_utime) as f32) * 1000.) / *HZ as f32;
                    let delta_stime =
                        ((stime.saturating_sub(prev_stime) as f32) * 1000.) / *HZ as f32;

                    process.descriptor.stats.cpu_usage =
                        (((delta_utime + delta_stime) / delta_time.as_millis() as f32) * 100.)
                            .min(100. * num_cpus::get() as f32);

                    let prev_read_bytes = process.raw_stats.disk_read_bytes;
                    let prev_write_bytes = process.raw_stats.disk_write_bytes;

                    let read_speed =
                        (io_parsed[PROC_PID_IO_READ_BYTES].saturating_sub(prev_read_bytes)) as f32
                            / delta_time.as_secs_f32();
                    let write_speed = (io_parsed[PROC_PID_IO_WRITE_BYTES]
                        .saturating_sub(prev_write_bytes))
                        as f32
                        / delta_time.as_secs_f32();
                    process.descriptor.stats.disk_usage = (read_speed + write_speed) / 2.;

                    process
                }
            };

            let cmd = match std::fs::read_to_string(entry_path.join("cmdline")) {
                Ok(output) => {
                    let mut cmd = ArrayVec::new();
                    for c in output
                        .split('\0')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_array_string_lossy())
                    {
                        cmd.push(c);
                        if cmd.is_full() {
                            break;
                        }
                    }

                    cmd
                }
                Err(e) => {
                    eprintln!("Failed to parse commandline for {}: {}", pid, e);

                    ArrayVec::new()
                }
            };

            let output = entry_path.join("exe").read_link();
            let exe = output
                .map(|p| p.as_os_str().to_string_lossy().to_array_string_lossy())
                .unwrap_or(ArrayString::new());

            let mut statm_parsed = [0; 7];
            match std::fs::read_to_string(entry_path.join("statm")) {
                Ok(statm_file_content) => {
                    parse_statm_file(&statm_file_content, &mut statm_parsed);
                }
                Err(e) => {
                    eprintln!("Failed to read memory information for {}: {}", pid, e);
                }
            };

            let cgroup = match std::fs::read_to_string(entry_path.join("cgroup")) {
                Ok(cfc) => {
                    if cfc.is_empty() {
                        eprintln!("Failed to read cgroup information for process {}: No cgroup associated with process", pid);
                        None
                    } else {
                        let mut cgroup = None;

                        let cfc = cfc
                            .trim()
                            .split(':')
                            .nth(2)
                            .unwrap_or("/")
                            .trim_start_matches('/')
                            .trim_end_matches(&format!("/{}", pid));

                        let cgroup_path = std::path::Path::new("/sys/fs/cgroup").join(cfc);
                        if !cfc.is_empty() && cgroup_path.exists() && cgroup_path.is_dir() {
                            let app_scope = cfc.split('/').last().unwrap_or("");
                            if (app_scope.starts_with("app") || app_scope.starts_with("snap"))
                                && app_scope.ends_with(".scope")
                            {
                                cgroup = Some(cgroup_path.to_string_lossy().into());
                            }
                        }

                        cgroup
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to read cgroup information for process {}: {}",
                        pid, e
                    );
                    None
                }
            };

            let mut task_count = 0_usize;
            match std::fs::read_dir(entry_path.join("task")) {
                Ok(tasks) => {
                    for task in tasks.filter_map(|t| t.ok()) {
                        match task.file_name().to_string_lossy().parse::<u32>() {
                            Err(_) => continue,
                            _ => {}
                        };
                        task_count += 1;
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Gatherer: Failed to read task directory for process {}: {}",
                        pid, e
                    );
                }
            }

            process.descriptor.pid = pid;
            process.descriptor.name = stat_name(&stat_parsed);
            process.descriptor.cmd = cmd;
            process.descriptor.exe = exe;
            process.descriptor.state = stat_state(&stat_parsed);
            process.descriptor.parent = stat_parent_pid(&stat_parsed);
            process.descriptor.stats.memory_usage =
                (statm_parsed[PROC_PID_STATM_RES] * (*PAGE_SIZE) as u64) as f32;
            process.raw_stats.user_jiffies = utime;
            process.raw_stats.kernel_jiffies = stime;
            process.raw_stats.disk_read_bytes = io_parsed[PROC_PID_IO_READ_BYTES];
            process.raw_stats.disk_write_bytes = io_parsed[PROC_PID_IO_WRITE_BYTES];
            process.raw_stats.net_bytes_sent = total_net_sent;
            process.raw_stats.net_bytes_recv = total_net_recv;
            process.raw_stats.timestamp = now;
            process.cgroup = cgroup;
            process.task_count = task_count;

            result.insert(pid, process);
        }
    }
}
