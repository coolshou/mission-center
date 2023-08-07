use common::ipc::{LocalSocketListener, LocalSocketStream, SharedMemory, SharedMemoryContent};

const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);
const RETRY_INCREMENT: std::time::Duration = std::time::Duration::from_millis(5);

pub type SharedData = common::SharedData;
pub type SharedDataContent = common::SharedDataContent;
pub type Message = common::ipc::Message;
pub type AppDescriptor = common::AppDescriptor;
pub type ExitCode = common::ExitCode;
pub type ProcessDescriptor = common::ProcessDescriptor;
pub type ProcessState = common::ProcessState;
#[allow(dead_code)]
pub type ProcessStats = common::ProcessStats;

#[path = "gatherer/common/mod.rs"]
mod common;

#[derive(Debug, thiserror::Error)]
pub enum GathererError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    SharedMemory(#[from] common::ipc::SharedMemoryError),
    #[error("Expected message {0:?}, got {1:?}")]
    MessageMissmatch(common::ipc::Message, common::ipc::Message),
    #[error("Disconnected")]
    Disconnected,
    #[error("Timeout")]
    Timeout,
}

pub struct Gatherer<SharedData: Sized> {
    listener: LocalSocketListener,
    shared_memory: SharedMemory<SharedData>,

    command: std::process::Command,

    child: Option<std::process::Child>,

    connection: Option<LocalSocketStream>,
}

unsafe impl<SharedData: Sized> Send for Gatherer<SharedData> {}

impl<SharedData: Sized> Gatherer<SharedData> {
    pub fn new<P: AsRef<std::path::Path>>(
        executable_path: P,
    ) -> Result<Gatherer<SharedData>, GathererError> {
        let executable_path = executable_path.as_ref();

        let process_pid = unsafe { libc::getpid() };

        let socket_path = format!("{}/sock_{}", super::STATE_DIR.as_str(), process_pid);
        if (std::path::Path::new(socket_path.as_str())).exists() {
            std::fs::remove_file(socket_path.as_str())?;
        }
        let listener = LocalSocketListener::bind(socket_path.as_str())?;
        listener.set_nonblocking(true)?;

        let shm_file_link = format!("{}/shm_{}", super::STATE_DIR.as_str(), process_pid);
        let shared_memory = SharedMemory::<SharedData>::new(shm_file_link.as_str(), true)?;

        let commandline = format!(
            "{} {} {}",
            executable_path.display(),
            socket_path.as_str(),
            shm_file_link.as_str()
        );
        let mut command = cmd!(&commandline);
        command
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        Ok(Gatherer {
            listener,
            shared_memory,

            command,

            child: None,

            connection: None,
        })
    }

    pub fn start(&mut self) -> Result<(), GathererError> {
        if self.is_running().is_ok() {
            return Ok(());
        }

        let child = self.command.spawn()?;
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
            return Err(GathererError::Timeout);
        }

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), GathererError> {
        if self.is_running().is_err() {
            return Ok(());
        }

        let child = core::mem::take(&mut self.child);
        if let Some(mut child) = child {
            'outter: for _ in 0..2 {
                let _ = self.send_message(Message::Exit);

                for _ in 0..2 {
                    match child.try_wait()? {
                        Some(_) => break 'outter,
                        None => {
                            // Wait a bit and try again, the child process might just be slow to stop
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            continue;
                        }
                    }
                }

                // Try to get the child to wake up in case it's stuck
                unsafe {
                    libc::kill(child.id() as i32, libc::SIGCONT);
                }
            }

            // Either way sever the connection with the child process
            self.connection = None;

            if child.try_wait()?.is_none() {
                match child.kill() {
                    Ok(_) => {}
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::InvalidInput {
                            // The process has already exited
                            return Ok(());
                        }

                        return Err(e.into());
                    }
                }
            }
        }

        Ok(())
    }

    pub fn send_message(&mut self, message: Message) -> Result<(), GathererError> {
        use std::io::Write;

        if let Some(connection) = self.connection.as_mut() {
            connection.write(&[message.into()])?;

            let reply = Self::read_message(connection, DEFAULT_TIMEOUT)?;
            if reply != Message::Acknowledge {
                return Err(GathererError::MessageMissmatch(Message::Acknowledge, reply));
            }

            Ok(())
        } else {
            Err(GathererError::Disconnected)
        }
    }

    pub fn shared_memory(&mut self) -> Result<SharedMemoryContent<SharedData>, GathererError> {
        if let Some(connection) = self.connection.as_mut() {
            let message = Self::read_message(connection, std::time::Duration::from_millis(5000))?;
            if message != Message::DataReady {
                return Err(GathererError::MessageMissmatch(Message::DataReady, message));
            }

            Ok(unsafe { self.shared_memory.acquire() })
        } else {
            Err(GathererError::Disconnected)
        }
    }

    pub unsafe fn shared_memory_unchecked(&mut self) -> SharedMemoryContent<SharedData> {
        self.shared_memory.acquire()
    }

    pub fn is_running(&mut self) -> Result<(), (ExitCode, i32)> {
        let child = match self.child.as_mut() {
            Some(child) => child,
            None => return Err((ExitCode::Unknown, -1)),
        };

        let status = match child.try_wait() {
            Ok(None) => return Ok(()),
            Ok(Some(status)) => status,
            Err(_) => {
                return Err((ExitCode::Unknown, -1));
            }
        };

        match status.code() {
            Some(status_code) => {
                if status_code < u8::MAX as _ && status_code > 0 {
                    Err((ExitCode::from(status_code as u8), status_code))
                } else {
                    Err((ExitCode::Unknown, status_code))
                }
            }
            None => Err((ExitCode::Unknown, -1)),
        }
    }

    fn read_message(
        connection: &mut LocalSocketStream,
        timeout: std::time::Duration,
    ) -> Result<Message, GathererError> {
        use std::io::Read;

        let mut reply = [0_u8];
        let mut timeout_left = timeout.as_millis();
        loop {
            if timeout_left == 0 {
                return Err(GathererError::Timeout);
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
