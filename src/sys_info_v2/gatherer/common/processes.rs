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

#[derive(Debug, Clone)]
pub struct RawStats {
    pub memory_usage: f32,

    user_jiffies: u64,
    kernel_jiffies: u64,

    disk_read_bytes: u64,
    disk_write_bytes: u64,

    net_bytes_sent: u64,
    net_bytes_recv: u64,

    timestamp: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct ProcessDescriptor {
    pub name: String,
    pub cmd: Vec<String>,
    pub exe: std::path::PathBuf,
    pub state: ProcessState,
    pub pid: libc::pid_t,
    pub parent: libc::pid_t,
    pub raw_stats: RawStats,
}
