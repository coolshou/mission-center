use lazy_static::lazy_static;

lazy_static! {
    static ref PAGE_SIZE: usize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
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

const PROC_PID_NET_DEV_RECV_BYTES: usize = 0;
const PROC_PID_NET_DEV_SENT_BYTES: usize = 8;

pub type Pid = libc::pid_t;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ProcessStats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    pub gpu_usage: f32,

    user_jiffies: u64,
    kernel_jiffies: u64,

    disk_read_bytes: u64,
    disk_write_bytes: u64,

    net_bytes_sent: u64,
    net_bytes_recv: u64,

    timestamp: std::time::Instant,
}

impl Default for ProcessStats {
    fn default() -> Self {
        Self {
            cpu_usage: 0.,
            memory_usage: 0.,
            disk_usage: 0.,
            network_usage: 0.,
            gpu_usage: 0.,

            user_jiffies: 0,
            kernel_jiffies: 0,

            disk_read_bytes: 0,
            disk_write_bytes: 0,

            net_bytes_sent: 0,
            net_bytes_recv: 0,

            timestamp: std::time::Instant::now(),
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

    pub process_stats: ProcessStats,
}

pub fn load_process_list(
    mut previous: std::collections::HashMap<Pid, Process>,
) -> std::collections::HashMap<Pid, Process> {
    use gtk::glib::{g_critical, g_debug};
    use std::collections::HashMap;

    fn parse_stat_file<'a>(data: &'a str, output: &mut [&'a str; 52]) {
        let mut part_index = 0;

        let mut split = data.split('(').filter(|x| !x.is_empty());
        output[part_index] = split.next().unwrap();
        part_index += 1;

        let mut split = split.next().unwrap().split(')').filter(|x| !x.is_empty());
        output[part_index] = split.next().unwrap();
        part_index += 1;

        for entry in split.next().unwrap().split_whitespace() {
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
            let entry = entry.split_whitespace().last().unwrap();
            output[part_index] = entry.trim().parse::<u64>().unwrap_or(0);
            part_index += 1;
        }
    }

    fn parse_net_dev_line(line: &str, output: &mut [u64; 16]) {
        let mut part_index = 0;

        for entry in line.split_whitespace().skip(1) {
            output[part_index] = entry.trim().parse::<u64>().unwrap_or(0);
            part_index += 1;
        }
    }

    fn stat_name(stat: &[&str; 52]) -> String {
        stat[PROC_PID_STAT_TCOMM].to_owned()
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
            "K" => ProcessState::Wakekill,
            "W" => ProcessState::Waking,
            "P" => ProcessState::Parked,
            _ => ProcessState::Unknown,
        }
    }

    fn stat_parent_pid(stat: &[&str; 52]) -> Pid {
        stat[PROC_PID_STAT_PPID].parse::<Pid>().unwrap_or(0)
    }

    fn stat_user_mode_jiffies(stat: &[&str; 52]) -> u64 {
        stat[PROC_PID_STAT_UTIME].parse::<u64>().unwrap_or(0)
    }

    fn stat_kernel_mode_jiffies(stat: &[&str; 52]) -> u64 {
        stat[PROC_PID_STAT_STIME].parse::<u64>().unwrap_or(0)
    }

    let mut result = HashMap::new();
    result.reserve(previous.len());

    let mut cmd = cmd!("ls -d /proc/[1-9]* || true");
    let output = cmd.output();
    if output.is_err() {
        g_critical!(
            "MissionCenter::ProcInfo",
            "Failed to list /proc: {}",
            output.err().unwrap()
        );
        return result;
    }
    let output = output.unwrap();
    let output = String::from_utf8_lossy(&output.stdout);

    let now = std::time::Instant::now();

    for entry in output.split_whitespace() {
        let pid = entry.split('/').last().and_then(|p| p.parse::<Pid>().ok());
        if pid.is_none() {
            g_debug!(
                "MissionCenter::ProcInfo",
                "Skipping non-numeric directory in /proc: {}",
                entry
            );
            continue;
        }
        let pid = pid.unwrap();

        let mut process = Process::default();
        process.pid = pid;
        process.process_stats.timestamp = now;

        let bench = std::time::Instant::now();
        let output = cmd!(&format!("cat {}/stat", entry)).output();
        let stat_parse = bench.elapsed();
        if output.is_err() {
            g_critical!(
                "MissionCenter::ProcInfo",
                "Failed to read stat information for process {}, skipping: {}",
                pid,
                output.err().unwrap()
            );
            continue;
        }
        let output = output.unwrap();
        let stat_file_content = String::from_utf8_lossy(&output.stdout).to_string();
        if stat_file_content.is_empty() {
            g_critical!(
                "MissionCenter::ProcInfo",
                "Failed to read stat information for process {}, skipping",
                pid
            );
            continue;
        }
        let mut stat_parsed = [""; 52];
        parse_stat_file(&stat_file_content, &mut stat_parsed);
        dbg!(stat_parse);

        let prev_process = previous.get_mut(&pid);

        let utime = stat_user_mode_jiffies(&stat_parsed);
        let stime = stat_kernel_mode_jiffies(&stat_parsed);

        let bench = std::time::Instant::now();
        let mut io_parsed = [0; 7];
        let output = cmd!(&format!("cat {}/io", entry)).output();
        let io_parse = bench.elapsed();
        if output.is_err() {
            g_critical!(
                "MissionCenter::ProcInfo",
                "Failed to read I/O information for process {}: {}",
                pid,
                output.err().unwrap()
            );
        } else {
            parse_io_file(
                String::from_utf8_lossy(&output.unwrap().stdout).as_ref(),
                &mut io_parsed,
            );
        }
        dbg!(io_parse);

        let bench = std::time::Instant::now();
        let mut total_net_sent = 0_u64;
        let mut total_net_recv = 0_u64;
        let output = cmd!(&format!("cat {}/net/dev", entry)).output();
        let stat_parse = bench.elapsed();
        if output.is_err() {
            g_critical!(
                "MissionCenter::ProcInfo",
                "Failed to read network information for process {}: {}",
                pid,
                output.err().unwrap()
            );
        } else {
            let output = output.unwrap();
            let output = String::from_utf8_lossy(&output.stdout);
            let mut lines = output.lines();
            lines.next();
            lines.next();
            for line in lines {
                let line = line.trim();
                if line.is_empty() || line.starts_with("lo") {
                    continue;
                }

                let mut dev_parsed = [0; 16];
                parse_net_dev_line(line, &mut dev_parsed);

                total_net_sent = total_net_sent
                    .overflowing_add(dev_parsed[PROC_PID_NET_DEV_SENT_BYTES])
                    .0;
                total_net_recv = total_net_recv
                    .overflowing_add(dev_parsed[PROC_PID_NET_DEV_RECV_BYTES])
                    .0;
            }
        };
        let net_parse = bench.elapsed();
        dbg!(net_parse);

        if prev_process.is_some() {
            let prev_process = prev_process.unwrap();

            std::mem::swap(&mut process.name, &mut prev_process.name);
            std::mem::swap(&mut process.cmd, &mut prev_process.cmd);
            std::mem::swap(&mut process.exe, &mut prev_process.exe);

            let delta_time = now - prev_process.process_stats.timestamp;

            let prev_utime = prev_process.process_stats.user_jiffies;
            let prev_stime = prev_process.process_stats.kernel_jiffies;

            process.process_stats.cpu_usage =
                ((utime.saturating_sub(prev_utime) + stime.saturating_sub(prev_stime)) as f32
                    / delta_time.as_secs_f32()
                    * 100.)
                    .min(100. * num_cpus::get() as f32);

            let prev_read_bytes = prev_process.process_stats.disk_read_bytes;
            let prev_write_bytes = prev_process.process_stats.disk_write_bytes;

            let read_speed = (io_parsed[PROC_PID_IO_READ_BYTES].saturating_sub(prev_read_bytes))
                as f32
                / delta_time.as_secs_f32();
            let write_speed = (io_parsed[PROC_PID_IO_WRITE_BYTES].saturating_sub(prev_write_bytes))
                as f32
                / delta_time.as_secs_f32();
            process.process_stats.disk_usage = (read_speed + write_speed) / 2.;

            let prev_bytes_recv = prev_process.process_stats.net_bytes_recv;
            let prev_bytes_sent = prev_process.process_stats.net_bytes_sent;

            let bytes_recv_speed = ((total_net_recv.saturating_sub(prev_bytes_recv)) as f32
                / delta_time.as_secs_f32())
                * 8.; // bps
            let bytes_sent_speed = ((total_net_sent.saturating_sub(prev_bytes_sent)) as f32
                / delta_time.as_secs_f32())
                * 8.; // bps

            process.process_stats.network_usage = (bytes_recv_speed + bytes_sent_speed) / 2.;
        } else {
            let bench = std::time::Instant::now();
            let output = cmd!(&format!("cat {}/cmdline", entry)).output();
            let cmd_parse = bench.elapsed();
            let cmd = if output.is_err() {
                g_critical!(
                    "MissionCenter::ProcInfo",
                    "Failed to parse commandline for {}: {}",
                    pid,
                    output.err().unwrap()
                );
                vec![]
            } else {
                let output = output.unwrap();
                String::from_utf8_lossy(&output.stdout)
                    .split_whitespace()
                    .map(|s| s.trim().to_owned())
                    .collect::<Vec<_>>()
            };
            dbg!(cmd_parse);

            let bench = std::time::Instant::now();
            let output = cmd!(&format!("readlink {}/exe", entry)).output();
            let exe_parse = bench.elapsed();
            let exe = if output.is_err() {
                g_debug!(
                    "MissionCenter::ProcInfo",
                    "Failed to read executable path for {}: {}",
                    pid,
                    output.err().unwrap()
                );

                std::path::PathBuf::new()
            } else {
                let output = output.unwrap();
                std::path::PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
            };
            dbg!(exe_parse);

            process.name = stat_name(&stat_parsed);
            process.cmd = cmd;
            process.exe = exe;
        }

        let bench = std::time::Instant::now();
        let mut statm_parsed = [0; 7];
        let output = cmd!(&format!("cat {}/statm", entry)).output();
        let statm_parse = bench.elapsed();
        if output.is_err() {
            g_debug!(
                "MissionCenter::ProcInfo",
                "Failed to read memory information for {}: {}",
                pid,
                output.err().unwrap()
            );
        } else {
            let output = output.unwrap();
            let statm_file_content = String::from_utf8_lossy(&output.stdout);
            parse_statm_file(&statm_file_content, &mut statm_parsed);
        }
        dbg!(statm_parse);

        process.state = stat_state(&stat_parsed);
        process.parent = stat_parent_pid(&stat_parsed);
        process.process_stats.memory_usage =
            (statm_parsed[PROC_PID_STATM_RES] * (*PAGE_SIZE) as u64) as f32;
        process.process_stats.user_jiffies = utime;
        process.process_stats.kernel_jiffies = stime;
        process.process_stats.disk_read_bytes = io_parsed[PROC_PID_IO_READ_BYTES];
        process.process_stats.disk_write_bytes = io_parsed[PROC_PID_IO_WRITE_BYTES];
        process.process_stats.net_bytes_sent = total_net_sent;
        process.process_stats.net_bytes_recv = total_net_recv;

        result.insert(pid, process);
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
}
