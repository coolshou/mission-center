pub type Pid = libc::pid_t;

#[derive(Debug, Clone)]
pub struct Stats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    pub gpu_usage: f32,

    #[allow(dead_code)]
    user_jiffies: u64,
    #[allow(dead_code)]
    kernel_jiffies: u64,

    #[allow(dead_code)]
    disk_read_bytes: u64,
    #[allow(dead_code)]
    disk_write_bytes: u64,

    #[allow(dead_code)]
    net_bytes_sent: u64,
    #[allow(dead_code)]
    net_bytes_recv: u64,

    #[allow(dead_code)]
    timestamp: std::time::Instant,
}

impl Default for Stats {
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

impl Stats {
    #[allow(dead_code)]
    pub fn serialize<W: std::io::Write>(&self, output: &mut W) -> std::io::Result<()> {
        output.write(to_binary(&self.cpu_usage))?;
        output.write(to_binary(&self.memory_usage))?;
        output.write(to_binary(&self.disk_usage))?;
        output.write(to_binary(&self.network_usage))?;
        output.write(to_binary(&self.gpu_usage))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn deserialize<R: std::io::Read>(&mut self, input: &mut R) -> std::io::Result<()> {
        input.read_exact(to_binary_mut(&mut self.cpu_usage))?;
        input.read_exact(to_binary_mut(&mut self.memory_usage))?;
        input.read_exact(to_binary_mut(&mut self.disk_usage))?;
        input.read_exact(to_binary_mut(&mut self.network_usage))?;
        input.read_exact(to_binary_mut(&mut self.gpu_usage))?;

        Ok(())
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

impl Process {
    #[allow(dead_code)]
    pub fn serialize<W: std::io::Write>(&self, output: &mut W) -> std::io::Result<()> {
        output.write(to_binary(&self.name.len()))?;
        output.write(self.name.as_bytes())?;
        output.write(to_binary(&self.cmd.len()))?;
        for arg in &self.cmd {
            output.write(to_binary(&arg.len()))?;
            output.write(arg.as_bytes())?;
        }
        let exe = self.exe.to_string_lossy();
        output.write(to_binary(&exe.len()))?;
        output.write(exe.as_bytes())?;
        output.write(to_binary(&(self.state as u8)))?;
        output.write(to_binary(&self.pid))?;
        output.write(to_binary(&self.parent))?;
        output.write(to_binary(&self.children.len()))?;
        for child in self.children.values() {
            child.serialize(output)?;
        }
        self.process_stats.serialize(output)?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn deserialize<R: std::io::Read>(input: &mut R) -> std::io::Result<Self> {
        let mut this = Self::default();

        let mut len = 0;

        input.read_exact(to_binary_mut(&mut len))?;
        let mut name = vec![0; len];
        input.read_exact(&mut name)?;
        this.name = unsafe { String::from_utf8_unchecked(name) };

        input.read_exact(to_binary_mut(&mut len))?;
        for _ in 0..len {
            input.read_exact(to_binary_mut(&mut len))?;
            let mut arg = vec![0; len];
            input.read_exact(&mut arg)?;
            this.cmd.push(unsafe { String::from_utf8_unchecked(arg) });
        }

        input.read_exact(to_binary_mut(&mut len))?;
        let mut exe = vec![0; len];
        input.read_exact(&mut exe)?;
        this.exe = std::path::PathBuf::from(unsafe { String::from_utf8_unchecked(exe) });

        let mut state = 0_u8;
        input.read_exact(to_binary_mut(&mut state))?;
        this.state = match state {
            0 => ProcessState::Running,
            1 => ProcessState::Sleeping,
            2 => ProcessState::SleepingUninterruptible,
            3 => ProcessState::Zombie,
            4 => ProcessState::Stopped,
            5 => ProcessState::Tracing,
            6 => ProcessState::Dead,
            7 => ProcessState::WakeKill,
            8 => ProcessState::Waking,
            9 => ProcessState::Parked,
            _ => ProcessState::Unknown,
        };

        input.read_exact(to_binary_mut(&mut this.pid))?;

        input.read_exact(to_binary_mut(&mut this.parent))?;

        input.read_exact(to_binary_mut(&mut len))?;
        for _ in 0..len {
            let child = Process::deserialize(input)?;
            this.children.insert(child.pid, child);
        }

        this.process_stats.deserialize(input)?;

        Ok(this)
    }
}
