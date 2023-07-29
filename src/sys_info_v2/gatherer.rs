use common::{
    ipc::{LocalSocketListener, LocalSocketStream, SharedMemory, SharedMemoryGuard},
    ExitCode,
};

pub type SharedData = common::SharedData;
pub type Message = common::ipc::Message;

#[path = "gatherer/common/mod.rs"]
mod common;

pub struct Gatherer<SharedData: Sized> {
    listener: LocalSocketListener,
    shared_memory: SharedMemory<SharedData>,

    command: std::process::Command,

    child: Option<std::process::Child>,
    stdout: Option<std::process::ChildStdout>,
    stderr: Option<std::process::ChildStderr>,

    connection: Option<LocalSocketStream>,
}

impl<SharedData: Sized> Gatherer<SharedData> {
    pub fn new<P: AsRef<std::path::Path>>(daemon_path: P) -> anyhow::Result<Gatherer<SharedData>> {
        let daemon_path = daemon_path.as_ref();
        if !daemon_path.exists() {
            return Err(anyhow::anyhow!("Daemon path does not exist"));
        }

        let process_pid = unsafe { libc::getpid() };

        let socket_path = format!("/tmp/sock_{}", process_pid);
        let shm_file_link = format!("/tmp/shm_{}", process_pid);

        let listener = LocalSocketListener::bind(socket_path.as_str())?;
        let shared_memory = SharedMemory::<SharedData>::new(shm_file_link.as_str(), true)?;

        let mut command = std::process::Command::new(daemon_path);
        command
            .arg(socket_path.as_str())
            .arg(shm_file_link.as_str())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        Ok(Gatherer {
            listener,
            shared_memory,

            command,

            child: None,
            stdout: None,
            stderr: None,

            connection: None,
        })
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.is_running().is_ok() {
            return Ok(());
        }

        let mut child = self.command.spawn()?;
        self.stdout = child.stdout.take();
        self.stderr = child.stderr.take();

        self.connection = Some(self.listener.accept()?);

        self.child = Some(child);

        Ok(())
    }

    pub fn send_message(&mut self, message: Message) -> anyhow::Result<()> {
        use std::io::Write;

        if let Some(connection) = self.connection.as_mut() {
            connection.write(&[message.into()])?;

            Ok(())
        } else {
            Err(anyhow::anyhow!("No connection to daemon"))
        }
    }

    pub fn shared_memory(&mut self) -> anyhow::Result<SharedMemoryGuard<SharedData>> {
        if let Some(lock) = self.shared_memory.lock(raw_sync::Timeout::Infinite) {
            Ok(lock)
        } else {
            Err(anyhow::anyhow!("Could not lock shared memory"))
        }
    }

    pub fn is_running(&mut self) -> Result<(), (ExitCode, i32)> {
        if self.child.is_none() {
            return Err((ExitCode::Unknown, -1));
        }

        let status = self.child.as_mut().unwrap().try_wait();
        if status.is_err() {
            return Err((ExitCode::Unknown, -1));
        }
        let status = status.unwrap();
        if status.is_none() {
            return Ok(());
        }

        let status = status.unwrap();
        if status.code().is_some() {
            let status_code = status.code().unwrap();
            if status_code < u8::MAX as _ && status_code > 0 {
                Err((ExitCode::from(status_code as u8), status_code))
            } else {
                Err((ExitCode::Unknown, status_code))
            }
        } else {
            return Err((ExitCode::Unknown, -1));
        }
    }

    pub fn stdout(&mut self) -> anyhow::Result<String> {
        use std::io::Read;

        let stdout = self.stdout.as_mut();
        if stdout.is_none() {
            return Ok(String::new());
        }

        let stdout = stdout.unwrap();
        let mut buffer = vec![];
        stdout.read_to_end(&mut buffer)?;

        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    pub fn stderr(&mut self) -> anyhow::Result<String> {
        use std::io::Read;

        let stderr = self.stderr.as_mut();
        if stderr.is_none() {
            return Ok(String::new());
        }

        let stderr = stderr.unwrap();
        let mut buffer = vec![];
        stderr.read_to_end(&mut buffer)?;

        Ok(String::from_utf8_lossy(&buffer).to_string())
    }
}
