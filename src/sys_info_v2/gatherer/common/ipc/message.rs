#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Message {
    GetProcesses,
    GetInstalledApps,
    TerminateProcess(u32 /* PID */),
    KillProcess(u32 /* PID */),
    KillProcessTree(u32 /* Parent PID */),
    Acknowledge,
    DataReady,
    Exit,
    Unknown,
}
