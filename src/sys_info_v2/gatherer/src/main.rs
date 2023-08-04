macro_rules! acknowledge {
    ($connection: ident) => {{
        use std::io::Write;

        if let Err(e) = $connection.write(&[common::ipc::Message::Acknowledge.into()]) {
            eprintln!("Failed to write to IPC socket, exiting: {:#?}", e);
            std::process::exit(common::ExitCode::SendAcknowledgeFailed as i32);
        }
    }};
}

macro_rules! data_ready {
    ($connection: ident) => {{
        use std::io::Write;

        if let Err(e) = $connection.write(&[common::ipc::Message::DataReady.into()]) {
            eprintln!("Failed to write to IPC socket, exiting: {:#?}", e);
            std::process::exit(common::ExitCode::SendDataReadyFailed as i32);
        }
    }};
}

#[path = "../common/mod.rs"]
mod common;

fn main() {
    use common::{ipc, ExitCode, InstalledApps, Processes, SharedData, SharedDataContent};
    use interprocess::local_socket::*;
    use std::io::Read;

    let parent_pid = unsafe { libc::getppid() };

    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        std::process::exit(ExitCode::MissingProgramArgument as i32);
    }

    if !std::path::Path::new(&args[1]).exists() {
        eprintln!("IPC socket '{}' does not exist", args[1]);
        std::process::exit(ExitCode::SocketConnectionFailed as i32);
    }
    let mut connection = match LocalSocketStream::connect(args[1].as_str()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Unable to connect to parent: {}", e);
            std::process::exit(ExitCode::SocketConnectionFailed as i32);
        }
    };

    if !std::path::Path::new(&args[2]).exists() {
        eprintln!("File link '{}' does not exist", args[2]);
        std::process::exit(ExitCode::FileLinkNotFound as i32);
    }
    let mut shared_memory = match ipc::SharedMemory::<SharedData>::new(&args[2], false) {
        Ok(sm) => sm,
        Err(e) => {
            eprintln!("Unable to create shared memory: {}", e);
            std::process::exit(ExitCode::UnableToCreateSharedMemory as i32);
        }
    };

    let mut recv_buffer = [0_u8; 1];
    loop {
        if unsafe { libc::getppid() } != parent_pid {
            eprintln!("Parent process no longer running, exiting");
            break;
        }

        if let Err(e) = connection.read_exact(&mut recv_buffer) {
            eprintln!("Failed to read from IPC socket, exiting: {:#?}", e);
            std::process::exit(ExitCode::ReadFromSocketFailed as i32);
        }

        let message = ipc::Message::from(recv_buffer[0]);
        match message {
            ipc::Message::GetProcesses => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::Processes(Processes::new());

                data_ready!(connection);
            }
            ipc::Message::GetInstalledApps => {
                acknowledge!(connection);

                let mut data = unsafe { shared_memory.acquire() };
                data.content = SharedDataContent::InstalledApps(InstalledApps::new());

                data_ready!(connection);
            }
            ipc::Message::Acknowledge | ipc::Message::DataReady => {
                // Wierd thing to send, but there you go, send Acknowledge back anyway
                acknowledge!(connection);
            }
            ipc::Message::Exit => {
                acknowledge!(connection);

                std::process::exit(0);
            }
            ipc::Message::Unknown => {
                eprintln!("Unknown message received");
                std::process::exit(ExitCode::UnknownMessageReceived as i32);
            }
        }
    }
}
