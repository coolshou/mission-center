pub use apps::{AppDescriptor, InstalledApps};
pub use exit_code::ExitCode;

#[allow(dead_code)]
mod apps;
mod exit_code;
pub mod ipc;
#[allow(dead_code)]
mod processes;

pub type ArrayString = arrayvec::ArrayString<128>;

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

impl ToArrayStringLossy for std::borrow::Cow<'_, str> {
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

#[allow(dead_code)]
#[derive(Debug)]
pub enum SharedDataContent {
    Monostate,
    InstalledApps(InstalledApps),
}

#[derive(Debug)]
pub struct SharedData {
    pub content: SharedDataContent,
}

#[allow(dead_code)]
impl SharedData {
    pub fn clear(&mut self) {
        self.content = SharedDataContent::Monostate;
    }
}

#[allow(dead_code)]
#[inline]
pub fn to_binary<T: Sized>(thing: &T) -> &[u8] {
    let ptr = thing as *const T;
    unsafe { core::slice::from_raw_parts(ptr as *const u8, core::mem::size_of::<T>()) }
}

#[allow(dead_code)]
#[inline]
pub fn to_binary_mut<T: Sized>(thing: &mut T) -> &mut [u8] {
    let ptr = thing as *mut T;
    unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, core::mem::size_of::<T>()) }
}
