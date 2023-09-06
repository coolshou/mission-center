/* sys_info_v2/gatherer.rs
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

use common::ipc::{LocalSocketListener, LocalSocketStream, SharedMemory, SharedMemoryContent};

const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

pub type ArrayString = common::ArrayString;
pub type SharedData = common::SharedData;
pub type SharedDataContent = common::SharedDataContent;
pub type Message = common::ipc::Message;
pub type AppDescriptor = common::AppDescriptor;
pub type AppStats = common::AppStats;
pub type ExitCode = common::ExitCode;
pub type ProcessDescriptor = common::ProcessDescriptor;
pub type ProcessState = common::ProcessState;
#[allow(dead_code)]
pub type ProcessStats = common::ProcessStats;
pub type CpuStaticInfo = common::CpuStaticInfo;

#[path = "gatherer/common/mod.rs"]
mod common;

#[derive(Debug, thiserror::Error)]
pub enum GathererError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    SharedMemory(#[from] common::ipc::SharedMemoryError),
    #[error("Expected message {0:?}, got {1:?}")]
    MessageMissmatch(common::ipc::Message, Message),
    #[error("Failed to send message: {0:?}")]
    MessageSendFailed(#[from] std::sync::mpsc::SendError<Message>),
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

        // Try to connect multiple times, if not successful, then bail
        for _ in 0..10 {
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
                    libc::kill(child.id() as _, libc::SIGCONT);
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
            connection.write(common::to_binary(&message))?;

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
        use gtk::glib::ffi::*;
        use std::{io::Read, os::fd::AsRawFd};

        let mut poll_fd = [GPollFD {
            fd: connection.as_raw_fd(),
            events: (G_IO_IN | G_IO_HUP | G_IO_ERR) as _,
            revents: 0,
        }];
        let ret = unsafe {
            g_poll(
                poll_fd.as_mut_ptr(),
                poll_fd.len() as _,
                timeout.as_millis() as _,
            )
        };
        if ret == 0 {
            return Err(GathererError::Timeout);
        } else if ret < 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        if poll_fd[0].revents & G_IO_HUP as u16 > 0 {
            return Err(GathererError::Disconnected);
        }

        if poll_fd[0].revents & G_IO_ERR as u16 > 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        let mut reply = Message::Unknown;
        let reply_bin = common::to_binary_mut(&mut reply);

        for byte in 0..core::mem::size_of::<Message>() {
            const RETRIES: usize = 2;
            for i in 0..RETRIES {
                // We know there is data ready on the file descriptor, so we can safely read
                match connection.read_exact(&mut reply_bin[byte..byte + 1]) {
                    Err(e) => {
                        // In case there was only a partial read, try again
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            if i == RETRIES - 1 {
                                return Err(GathererError::Timeout);
                            } else {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                                continue;
                            }
                        }
                        return if e.kind() == std::io::ErrorKind::UnexpectedEof {
                            Err(GathererError::Disconnected)
                        } else {
                            Err(e.into())
                        };
                    }
                    _ => break,
                }
            }
        }

        Ok(reply)
    }
}
