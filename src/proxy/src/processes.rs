/* proxy/src/processes.rs
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

use lazy_static::lazy_static;

lazy_static! {
    static ref PAGE_SIZE: usize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
    static ref HZ: usize = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as usize };
    static ref XDG_DATA_DIRS: Vec<String> = {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| format!("/usr/share:{}/.local/share", home));

        let mut dirs = xdg_data_dirs
            .split(':')
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        dirs.push(format!("{}/.local/share", home));

        dirs
    };
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

include!("../../common/util.rs");
include!("../../common/process.rs");

pub fn load_process_list(
    mut previous: std::collections::HashMap<Pid, Process>,
) -> std::collections::HashMap<Pid, Process> {
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
            "K" => ProcessState::WakeKill,
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

    let now = std::time::Instant::now();

    let proc = std::fs::read_dir("/proc");
    if proc.is_err() {
        eprintln!("CRTFailed to read /proc directory: {}", proc.err().unwrap());
        return previous;
    }
    let proc_entries = proc
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false));
    for entry in proc_entries {
        let pid = entry.file_name().to_string_lossy().parse::<Pid>();
        if pid.is_err() {
            eprintln!(
                "DBGSkipping non-numeric directory in /proc: {}: {}",
                entry.path().display(),
                pid.err().unwrap()
            );
            continue;
        }
        let pid = pid.unwrap();
        let entry_path = entry.path();

        let output = std::fs::read_to_string(entry_path.join("stat"));
        if output.is_err() {
            eprintln!(
                "DBGFailed to read stat information for process {}, skipping: {}",
                pid,
                output.err().unwrap()
            );
            continue;
        }
        let stat_file_content = output.unwrap();
        if stat_file_content.is_empty() {
            eprintln!(
                "DBGFailed to read stat information for process {}, skipping",
                pid
            );
            continue;
        }
        let mut stat_parsed = [""; 52];
        parse_stat_file(&stat_file_content, &mut stat_parsed);

        let utime = stat_user_mode_jiffies(&stat_parsed);
        let stime = stat_kernel_mode_jiffies(&stat_parsed);

        let mut io_parsed = [0; 7];
        let output = std::fs::read_to_string(entry_path.join("io"));
        if output.is_err() {
            eprintln!(
                "DBGFailed to read I/O information for process {}: {}",
                pid,
                output.err().unwrap()
            );
        } else {
            parse_io_file(output.unwrap().as_str(), &mut io_parsed);
        }

        let total_net_sent = 0_u64;
        let total_net_recv = 0_u64;

        let mut process;

        let prev_process = previous.remove(&pid);
        if prev_process.is_some() {
            process = prev_process.unwrap();

            let delta_time = now - process.stats.timestamp;

            let prev_utime = process.stats.user_jiffies;
            let prev_stime = process.stats.kernel_jiffies;

            let delta_utime = ((utime.saturating_sub(prev_utime) as f32) * 1000.) / *HZ as f32;
            let delta_stime = ((stime.saturating_sub(prev_stime) as f32) * 1000.) / *HZ as f32;

            process.stats.cpu_usage =
                (((delta_utime + delta_stime) / delta_time.as_millis() as f32) * 100.)
                    .min(100. * num_cpus::get() as f32);

            let prev_read_bytes = process.stats.disk_read_bytes;
            let prev_write_bytes = process.stats.disk_write_bytes;

            let read_speed = (io_parsed[PROC_PID_IO_READ_BYTES].saturating_sub(prev_read_bytes))
                as f32
                / delta_time.as_secs_f32();
            let write_speed = (io_parsed[PROC_PID_IO_WRITE_BYTES].saturating_sub(prev_write_bytes))
                as f32
                / delta_time.as_secs_f32();
            process.stats.disk_usage = (read_speed + write_speed) / 2.;
        } else {
            process = Process::default();
        }

        let output = std::fs::read_to_string(entry_path.join("cmdline"));
        let cmd = if output.is_err() {
            eprintln!(
                "DBGFailed to parse commandline for {}: {}",
                pid,
                output.err().unwrap()
            );
            vec![]
        } else {
            output
                .unwrap()
                .split('\0')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned())
                .collect::<Vec<_>>()
        };

        let output = entry_path.join("exe").read_link();
        let exe = if output.is_err() {
            eprintln!(
                "DBGFailed to read executable path for {}: {}",
                pid,
                output.err().unwrap()
            );

            std::path::PathBuf::new()
        } else {
            output.unwrap()
        };

        let mut statm_parsed = [0; 7];
        let output = std::fs::read_to_string(entry_path.join("statm"));
        if output.is_err() {
            eprintln!(
                "DBGFailed to read memory information for {}: {}",
                pid,
                output.err().unwrap()
            );
        } else {
            let statm_file_content = output.unwrap();
            parse_statm_file(&statm_file_content, &mut statm_parsed);
        }

        process.pid = pid;
        process.stats.timestamp = now;
        process.name = stat_name(&stat_parsed);
        process.cmd = cmd;
        process.exe = exe;
        process.state = stat_state(&stat_parsed);
        process.parent = stat_parent_pid(&stat_parsed);
        process.stats.memory_usage =
            (statm_parsed[PROC_PID_STATM_RES] * (*PAGE_SIZE) as u64) as f32;
        process.stats.user_jiffies = utime;
        process.stats.kernel_jiffies = stime;
        process.stats.disk_read_bytes = io_parsed[PROC_PID_IO_READ_BYTES];
        process.stats.disk_write_bytes = io_parsed[PROC_PID_IO_WRITE_BYTES];
        process.stats.net_bytes_sent = total_net_sent;
        process.stats.net_bytes_recv = total_net_recv;

        result.insert(pid, process);
    }

    result
}

pub fn save_stats_to_cache<'a, P: AsRef<std::path::Path>>(
    file_path: P,
    processes: impl IntoIterator<Item = &'a Process>,
) -> std::io::Result<()> {
    use std::io::{Seek, Write};

    let mut file = std::fs::File::create(file_path)?;

    let mut process_count = 0_usize;
    // Write a placeholder value
    file.write(to_binary(&process_count))?;
    for p in processes {
        file.write(to_binary(&p.pid))?;
        file.write(to_binary(&p.stats.user_jiffies))?;
        file.write(to_binary(&p.stats.kernel_jiffies))?;
        file.write(to_binary(&p.stats.disk_read_bytes))?;
        file.write(to_binary(&p.stats.disk_write_bytes))?;
        file.write(to_binary(&p.stats.net_bytes_sent))?;
        file.write(to_binary(&p.stats.net_bytes_recv))?;
        file.write(to_binary(&p.stats.timestamp))?;

        process_count += 1;
    }
    file.flush()?;
    file.rewind()?;
    file.write(to_binary(&process_count))?;

    file.flush()
}

pub fn load_stats_from_cache<'a, P: AsRef<std::path::Path>>(
    file_path: P,
) -> std::io::Result<std::collections::HashMap<Pid, Process>> {
    use std::io::Read;

    let mut file = std::fs::File::open(file_path)?;

    let mut result = std::collections::HashMap::new();

    let mut process_count = 0_usize;
    file.read_exact(to_binary_mut(&mut process_count))?;
    for _ in 0..process_count {
        let mut p = Process::default();
        file.read_exact(to_binary_mut(&mut p.pid))?;
        file.read_exact(to_binary_mut(&mut p.stats.user_jiffies))?;
        file.read_exact(to_binary_mut(&mut p.stats.kernel_jiffies))?;
        file.read_exact(to_binary_mut(&mut p.stats.disk_read_bytes))?;
        file.read_exact(to_binary_mut(&mut p.stats.disk_write_bytes))?;
        file.read_exact(to_binary_mut(&mut p.stats.net_bytes_sent))?;
        file.read_exact(to_binary_mut(&mut p.stats.net_bytes_recv))?;
        file.read_exact(to_binary_mut(&mut p.stats.timestamp))?;

        result.insert(p.pid, p);
    }

    Ok(result)
}
