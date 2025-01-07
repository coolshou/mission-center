/* sys_info_v2/gatherer/src/platform/disk_info.rs
 *
 * Copyright 2024 Romeo Calota
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

use dbus::arg::IterAppend;
use dbus::{
    arg::{Append, Arg, ArgType},
    Signature,
};

#[allow(non_camel_case_types)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DiskType {
    #[default]
    Unknown = 0,
    HDD,
    SSD,
    NVMe,
    eMMC,
    SD,
    Optical,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DiskSmartInterface {
    #[default]
    Dumb = 0,
    Ata,
    NVMe,
}

/// Describes the static (unchanging) information about a physical disk
pub trait DiskInfoExt: Default + Append + Arg {
    /// The disk's unique identifier
    fn id(&self) -> &str;

    /// The disk's model in human-readable form
    fn model(&self) -> &str;

    /// The disk's type
    fn r#type(&self) -> DiskType;

    fn smart_interface(&self) -> DiskSmartInterface;

    /// The disk's capacity in bytes
    fn capacity(&self) -> u64;

    /// The disk's formatted capacity in bytes
    fn formatted(&self) -> u64;

    /// Check if the disk is the system disk
    fn is_system_disk(&self) -> bool;

    /// The disk's busy percentage
    fn busy_percent(&self) -> f32;

    /// The disk's response time in milliseconds
    fn response_time_ms(&self) -> f32;

    /// The disk's read speed in bytes per second
    fn read_speed(&self) -> u64;

    /// The number of bytes read from this disk
    fn total_read(&self) -> u64;

    /// The disk's write speed in bytes per second
    fn write_speed(&self) -> u64;

    /// The number of bytes written to this disk
    fn total_write(&self) -> u64;

    /// The disk's write speed in bytes per second
    fn ejectable(&self) -> bool;

    fn drive_temperature(&self) -> u32;
}

impl Arg for crate::platform::DiskInfo {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(ssyyttbddttttbu)")
    }
}

impl Append for crate::platform::DiskInfo {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append_struct(|ia| {
            ia.append(self.id());
            ia.append(self.model());
            ia.append(self.r#type() as u8);
            ia.append(self.smart_interface() as u8);
            ia.append(self.capacity());
            ia.append(self.formatted());
            ia.append(self.is_system_disk());
            ia.append(self.busy_percent() as f64);
            ia.append(self.response_time_ms() as f64);
            ia.append(self.read_speed());
            ia.append(self.total_read());
            ia.append(self.write_speed());
            ia.append(self.total_write());
            ia.append(self.ejectable());
            ia.append(self.drive_temperature());
        });
    }
}

impl Append for crate::platform::DiskInfoIter<'_> {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append_array(&crate::platform::DiskInfo::signature(), |a| {
            for v in self.0.clone() {
                a.append(v);
            }
        });
    }
}

/// Provides an interface for gathering disk information
pub trait DisksInfoExt<'a> {
    type S: DiskInfoExt;
    type Iter: Iterator<Item = &'a Self::S>
    where
        <Self as DisksInfoExt<'a>>::S: 'a;

    /// Refresh the internal information cache
    ///
    /// It is expected that implementors of this trait cache this information, once obtained
    /// from the underlying OS
    fn refresh_cache(&mut self);

    /// Returns the static information for the disks present in the system.
    fn info(&'a self) -> Self::Iter;
}
