/* sys_info_v2/gatherer/common/mod.rs
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

pub use apps::{AppDescriptor, AppPIDs, Apps};
pub use exit_code::ExitCode;
pub use processes::{ProcessDescriptor, ProcessState, Processes};

#[allow(dead_code)]
mod apps;
mod cpu;
mod exit_code;
pub mod ipc;
#[allow(dead_code)]
mod processes;

pub type ArrayString = arrayvec::ArrayString<256>;

#[allow(dead_code)]
pub type AppStats = apps::Stats;
#[allow(dead_code)]
pub type ProcessStats = processes::Stats;

#[allow(dead_code)]
pub type CpuStaticInfo = cpu::StaticInfo;

pub trait ToArrayStringLossy {
    fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY>;
}

impl ToArrayStringLossy for str {
    fn to_array_string_lossy<const CAPACITY: usize>(&self) -> arrayvec::ArrayString<CAPACITY> {
        let mut result = arrayvec::ArrayString::new();
        if self.len() > CAPACITY {
            for i in (0..CAPACITY).rev() {
                if self.is_char_boundary(i) {
                    result.push_str(&self[0..i]);
                    break;
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
                    result.push_str(&self[0..i]);
                    break;
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
    Processes(Processes),
    Apps(Apps),
    AppPIDs(AppPIDs),
    CpuStaticInfo(CpuStaticInfo),
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
