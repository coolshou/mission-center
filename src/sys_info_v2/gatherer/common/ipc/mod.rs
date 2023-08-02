pub use interprocess::{local_socket::LocalSocketListener, local_socket::LocalSocketStream};

pub use message::Message;
pub use shm::{SharedMemory, SharedMemoryContent, SharedMemoryError};

#[path = "message.rs"]
mod message;
#[path = "shm.rs"]
mod shm;
