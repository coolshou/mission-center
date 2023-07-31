use common::{
    ipc::{LocalSocketListener, LocalSocketStream, SharedMemory, SharedMemoryContent},
    ExitCode,
};

const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);
const RETRY_INCREMENT: std::time::Duration = std::time::Duration::from_millis(5);

pub type SharedData = common::SharedData;
pub type SharedDataContent = common::SharedDataContent;
pub type Message = common::ipc::Message;
pub type AppDescriptor = common::AppDescriptor;

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

unsafe impl<SharedData: Sized> Send for Gatherer<SharedData> {}

impl<SharedData: Sized> Gatherer<SharedData> {
    pub fn new<P: AsRef<std::path::Path>>(
        executable_path: P,
    ) -> anyhow::Result<Gatherer<SharedData>> {
        let executable_path = executable_path.as_ref();

        let process_pid = unsafe { libc::getpid() };

        let socket_path = format!("{}/sock_{}", super::STATE_DIR.as_str(), process_pid);
        let shm_file_link = format!("{}/shm_{}", super::STATE_DIR.as_str(), process_pid);

        let listener = LocalSocketListener::bind(socket_path.as_str())?;
        listener.set_nonblocking(true)?;
        let shared_memory = SharedMemory::<SharedData>::new(shm_file_link.as_str(), true)?;

        let mut command = std::process::Command::new(executable_path);
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
        self.child = Some(child);

        // Let the child process start up
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Try to connect twice, if not successful, then bail
        for _ in 0..2 {
            self.connection = match self.listener.accept() {
                Ok(connection) => {
                    connection.set_nonblocking(true)?;
                    Some(connection)
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::WouldBlock => {
                        // Wait a bit and try again, the child process might just be slow to start
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                    _ => return Err(e.into()),
                },
            };

            break;
        }

        if self.connection.is_none() {
            return Err(anyhow::anyhow!("Failed to start daemon"));
        }

        Ok(())
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        if self.is_running().is_err() {
            return Ok(());
        }

        let child = core::mem::take(&mut self.child);
        if let Some(mut child) = child {
            self.send_message(Message::Exit)?;

            for _ in 0..2 {
                match child.try_wait()? {
                    Some(_) => break,
                    None => {
                        // Wait a bit and try again, the child process might just be slow to stop
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                }
            }

            if child.try_wait()?.is_none() {
                match child.kill() {
                    Ok(_) => {}
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::InvalidInput {
                            // The process has already exited
                            return Ok(());
                        }

                        return Err(anyhow::anyhow!(
                            "Failed to kill gatherer process: {}",
                            e.to_string()
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn send_message(&mut self, message: Message) -> anyhow::Result<()> {
        use std::io::Write;

        if let Some(connection) = self.connection.as_mut() {
            connection.write(&[message.into()])?;

            let reply = Self::read_message(connection, DEFAULT_TIMEOUT)?;
            if reply != Message::Acknowledge {
                return Err(anyhow::anyhow!(
                    "Failed to send message, received {:?} instead of Acknowledge",
                    reply
                ));
            }

            Ok(())
        } else {
            Err(anyhow::anyhow!("No connection to gatherer"))
        }
    }

    pub fn shared_memory(&mut self) -> anyhow::Result<SharedMemoryContent<SharedData>> {
        if let Some(connection) = self.connection.as_mut() {
            let message = Self::read_message(connection, std::time::Duration::from_millis(5000))?;
            if message != Message::DataReady {
                return Err(anyhow::anyhow!(
                    "Unknown if shared memory is ready, received {:?} instead of DataReady",
                    message
                ));
            }

            Ok(unsafe { self.shared_memory.acquire() })
        } else {
            Err(anyhow::anyhow!("No connection to gatherer"))
        }
    }

    pub unsafe fn shared_memory_unchecked(&mut self) -> SharedMemoryContent<SharedData> {
        self.shared_memory.acquire()
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
        loop {
            let count = match stdout.read(&mut buffer) {
                Ok(count) => count,
                Err(e) => match e.kind() {
                    std::io::ErrorKind::Interrupted => continue,
                    _ => return Err(e.into()),
                },
            };

            if count == 0 {
                break;
            }
        }

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
        loop {
            let count = match stderr.read(&mut buffer) {
                Ok(count) => count,
                Err(e) => match e.kind() {
                    std::io::ErrorKind::Interrupted => continue,
                    _ => return Err(e.into()),
                },
            };

            if count == 0 {
                break;
            }
        }

        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    fn read_message(
        connection: &mut LocalSocketStream,
        timeout: std::time::Duration,
    ) -> anyhow::Result<Message> {
        use std::io::Read;

        let mut reply = [0_u8];
        let mut timeout_left = timeout.as_millis();
        loop {
            if timeout_left == 0 {
                return Err(anyhow::anyhow!("Timeout while waiting for reply"));
            }

            match connection.read_exact(&mut reply) {
                Ok(_) => break,
                Err(e) => match e.kind() {
                    std::io::ErrorKind::WouldBlock => {
                        // Wait a bit and try again, the child process might just be slow to respond
                        timeout_left = timeout_left.saturating_sub(RETRY_INCREMENT.as_millis());
                        std::thread::sleep(RETRY_INCREMENT);
                        continue;
                    }
                    _ => return Err(e.into()),
                },
            }
        }

        Ok(Message::from(reply[0]))
    }
}
