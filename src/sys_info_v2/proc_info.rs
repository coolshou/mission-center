/* sys_info_v2/proc_info.rs
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

    pub fn cmd(&self) -> &[super::gatherer::ArrayString] {
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
            |gatherer, process_restarted| {
                let shared_memory = match gatherer.shared_memory() {
                    Ok(shm) => shm,
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::ProcInfo",
                            "Unable to to access shared memory: {}",
                            e
                        );
                        return false;
                    }
                };

                match shared_memory.content {
                    SharedDataContent::Processes(ref proceses) => {
                        if process_restarted {
                            result.clear();
                        }

                        for proc in &proceses.processes {
                            result.insert(proc.pid, Process::new(proc.clone()));
                        }
                        proceses.is_complete
                    }
                    SharedDataContent::Apps(_) => {
                        g_critical!(
                            "MissionCenter::ProcInfo",
                            "Shared data content is Apps instead of Processes; encountered when reading processes from gatherer", 
                        );
                        false
                    }
                    SharedDataContent::AppPIDs(_) => {
                        g_critical!(
                            "MissionCenter::ProcInfo",
                            "Shared data content is AppPIDs instead of Processes; encountered when reading processes from gatherer", 
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
                }
            },
        );

        result
    }
}

pub fn process_hierarchy(
    processes: &std::collections::HashMap<u32, Process>,
    merged_stats: bool,
) -> Option<Process> {
    use super::gatherer::ProcessStats;
    use gtk::glib::*;
    use std::collections::*;

    let now = std::time::Instant::now();

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
    process_tree.insert(root_process.pid(), 0_usize);

    let mut children = Vec::with_capacity(pids.len());
    children.push(HashMap::new());

    let mut visited = HashSet::new();
    visited.insert(root_process.pid());

    for pid in pids.iter().skip(1).rev() {
        if visited.contains(pid) {
            continue;
        }

        let process = match processes.get(pid) {
            None => continue,
            Some(p) => p,
        };

        let mut stack = vec![process];
        let mut parent = process.parent();
        while parent != 0 {
            let parent_process = match processes.get(&parent) {
                None => break,
                Some(pp) => pp,
            };

            if visited.contains(&parent_process.pid()) {
                let mut index = match process_tree.get(&parent_process.pid()) {
                    None => {
                        // TODO: Fully understand if this could happen, and what to do if it does.
                        g_critical!(
                            "MissionCenter::ProcInfo",
                            "Process {} has been visited, but it's not in the process_tree?",
                            process.pid()
                        );
                        break;
                    }
                    Some(index) => *index,
                };
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
        merged_stats: bool,
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
            gather_descendants(child, process_tree, children, merged_stats);
            if merged_stats {
                process_stats.merge(&child.stats());
            }
        }
        if merged_stats {
            process.stats_mut().merge(&process_stats);
        }
    }

    let process = &mut root_process;
    std::mem::swap(&mut process.children, &mut children[0]);

    let mut process_stats = ProcessStats::default();
    for (_, child) in &mut process.children {
        gather_descendants(child, &process_tree, &mut children, merged_stats);
        if merged_stats {
            process_stats.merge(&child.stats());
        }
    }
    if merged_stats {
        process.stats_mut().merge(&process_stats);
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
