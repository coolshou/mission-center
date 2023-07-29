pub use apps::InstalledApps;
pub use exit_code::ExitCode;

mod apps;
mod exit_code;
pub mod ipc;

pub trait ToArrayStringLossy {
    fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY>;
}

impl ToArrayStringLossy for str {
    fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY> {
        let mut result = arrayvec::ArrayString::new();
        if self.len() > CAPACITY {
            for i in (0..CAPACITY).rev() {
                if self.is_char_boundary(i) {
                    result.push_str(&self[..i + 1]);
                }
            }
        } else {
            result.push_str(self);
        }

        result
    }
}

#[derive(Debug)]
pub enum SharedDataContent {
    InstalledApps(InstalledApps),
}

#[derive(Debug)]
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
