#[path = "../common/mod.rs"]
mod common;

fn main() {
    use common::{ipc, ExitCode, InstalledApps, SharedData, SharedDataContent};
    use interprocess::local_socket::*;
    use std::io::Read;

    let parent_pid = unsafe { libc::getppid() };

    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        eprintln!("Usage: {} <ipc_socket_path> <shm_file_link> ", args[0]);
        std::process::exit(ExitCode::MissingProgramArgument as i32);
    }

    if !std::path::Path::new(&args[1]).exists() {
        eprintln!("IPC socket '{}' does not exist", args[1]);
        std::process::exit(ExitCode::SocketConnectionFailed as i32);
    }
    let connection = LocalSocketStream::connect(args[1].as_str());
    if connection.is_err() {
        eprintln!("Unable to connect to parent: {}", connection.err().unwrap());
        std::process::exit(ExitCode::SocketConnectionFailed as i32);
    }
    let mut connection = connection.unwrap();

    if !std::path::Path::new(&args[2]).exists() {
        eprintln!("File link '{}' does not exist", args[2]);
        std::process::exit(ExitCode::FileLinkNotFound as i32);
    }
    let shared_memory = ipc::SharedMemory::<SharedData>::new(&args[2], false);
    if shared_memory.is_err() {
        eprintln!(
            "Unable to create shared memory: {:?}",
            shared_memory.err().unwrap()
        );
        std::process::exit(ExitCode::UnableToCreateSharedMemory as i32);
    }
    let mut shared_memory = shared_memory.unwrap();

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
            ipc::Message::GetInstalledApps => {
                let data = shared_memory.lock(raw_sync::Timeout::Infinite);
                if data.is_none() {
                    eprintln!("Unable to obtain shared memory data lock");
                    continue;
                }
                let mut data = data.unwrap();
                data.content = SharedDataContent::InstalledApps(InstalledApps::new());
            }
            ipc::Message::Exit => {
                std::process::exit(0);
            }
            ipc::Message::Unknown => {
                eprintln!("Unknown message received");
                std::process::exit(ExitCode::UnknownMessageReceived as i32);
            }
        }
    }
}
