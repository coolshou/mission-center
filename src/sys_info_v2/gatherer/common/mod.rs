pub use apps::InstalledApps;
pub use exit_code::ExitCode;

mod apps;
mod exit_code;
pub mod ipc;

pub type FixedString = [u8; 256];

#[repr(C)]
pub enum SharedDataContent {
    InstalledApps(InstalledApps),
}

#[repr(C)]
pub struct SharedData {
    pub is_complete: bool,
    pub content: SharedDataContent,
}

#[inline]
pub fn to_binary<T: Sized>(thing: &T) -> &[u8] {
    let ptr = thing as *const T;
    unsafe { core::slice::from_raw_parts(ptr as *const u8, core::mem::size_of::<T>()) }
}

#[inline]
pub fn to_binary_mut<T: Sized>(thing: &mut T) -> &mut [u8] {
    let ptr = thing as *mut T;
    unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, core::mem::size_of::<T>()) }
}

pub fn fixed_string(string: &str) -> FixedString {
    let mut fixed_string: FixedString = [0; 256];

    let bytes = string.as_bytes();
    let mut len = bytes.len();
    if len > core::mem::size_of::<FixedString>() {
        for i in (0..core::mem::size_of::<FixedString>()).rev() {
            if string.is_char_boundary(i) {
                len = i + 1;
                break;
            }
        }
    }

    fixed_string.copy_from_slice(&bytes[..len]);
    fixed_string
}
