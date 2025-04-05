use std::fmt::Write;

use arrayvec::ArrayString;

pub use cpu::list_item_factory as cpu_list_item_factory;
pub use drive::list_item_factory as drive_list_item_factory;
pub use gpu::list_item_factory as gpu_list_item_factory;
pub use gpu_memory::list_item_factory as gpu_memory_list_item_factory;
pub use memory::list_item_factory as memory_list_item_factory;
pub use name::list_item_factory as name_list_item_factory;
pub use name_cell::NameCell;
pub use pid::list_item_factory as pid_list_item_factory;
pub use shared_memory::list_item_factory as shared_memory_list_item_factory;

mod cpu;
mod drive;
mod gpu;
mod gpu_memory;
mod memory;
mod name;
mod name_cell;
mod pid;
mod shared_memory;

fn format_bytes(bytes: f32) -> ArrayString<128> {
    let mut buffer = ArrayString::<128>::new();

    let (v, unit, _) = crate::to_human_readable(bytes, 1024.);
    if unit.is_empty() {
        let _ = write!(&mut buffer, "{} B", v.round() as u32);
    } else {
        let _ = write!(&mut buffer, "{} {}iB", v.round() as u32, unit);
    }

    buffer
}
