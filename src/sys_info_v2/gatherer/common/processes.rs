use arrayvec::ArrayVec;
use lazy_static::lazy_static;

use super::ArrayString;

mod state {
    use std::{cell::Cell, collections::HashMap, thread_local};

    thread_local! {
        pub static FULL_PROCESS_CACHE: Cell<HashMap<u32, super::Process>> = Cell::new(HashMap::new());
        pub static PROCESS_CACHE: Cell<Vec<super::ProcessDescriptor>> = Cell::new(vec![]);
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

#[derive(Debug, Copy, Clone)]
pub enum ProcessState {
    Running,
    Sleeping,
    SleepingUninterruptible,
    Zombie,
    Stopped,
    Tracing,
    Dead,
    WakeKill,
    Waking,
    Parked,
    Unknown,
}

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

#[derive(Debug, Default, Copy, Clone)]
pub struct Stats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    pub gpu_usage: f32,
}

impl Stats {
    pub fn merge(&mut self, other: &Self) {
        self.cpu_usage += other.cpu_usage;
        self.memory_usage += other.memory_usage;
        self.disk_usage += other.disk_usage;
        self.network_usage += other.network_usage;
        self.gpu_usage += other.gpu_usage;
    }
}

#[derive(Debug, Clone)]
pub struct ProcessDescriptor {
    pub name: ArrayString,
    pub cmd: ArrayVec<ArrayString, 8>,
    pub exe: ArrayString,
    pub state: ProcessState,
    pub pid: u32,
    pub parent: u32,
    pub stats: Stats,
}

#[derive(Debug, Clone)]
pub struct Process {
    descriptor: ProcessDescriptor,
    raw_stats: RawStats,
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct Processes {
    pub processes: ArrayVec<ProcessDescriptor, 25>,
    pub is_complete: bool,
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
                state::FULL_PROCESS_CACHE.with(|state| unsafe { &*state.as_ptr() });
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

        let mut previous = state::FULL_PROCESS_CACHE.with(|prev| prev.take());
        let result = state::FULL_PROCESS_CACHE.with(|prev| unsafe { &mut *prev.as_ptr() });
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

            result.insert(pid, process);
        }
    }
}
