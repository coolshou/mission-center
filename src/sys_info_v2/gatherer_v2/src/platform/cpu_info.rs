/* sys_info_v2/gatherer/src/platform/cpu_info.rs
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

use dbus::arg::{Append, Arg};

#[repr(u8)]
pub enum OptionalBool {
    False,
    True,
    None,
}

impl From<Option<bool>> for OptionalBool {
    fn from(value: Option<bool>) -> Self {
        value.map_or(OptionalBool::None, |b| {
            if b {
                OptionalBool::True
            } else {
                OptionalBool::False
            }
        })
    }
}

pub trait CpuStaticInfoExt: Append + Arg {
    fn name(&self) -> &str;
    fn logical_cpu_count(&self) -> u32;
    fn socket_count(&self) -> Option<u8>;
    fn base_frequency_khz(&self) -> Option<u64>;
    fn virtualization_supported(&self) -> Option<bool>;
    fn is_virtual_machine(&self) -> Option<bool>;
    fn l1_combined_cache(&self) -> Option<u64>;
    fn l2_cache(&self) -> Option<u64>;
    fn l3_cache(&self) -> Option<u64>;
    fn l4_cache(&self) -> Option<u64>;
}

impl Arg for crate::platform::CpuStaticInfo {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Struct;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("(suytyytttt)")
    }
}

impl Append for crate::platform::CpuStaticInfo {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append((
            self.name(),
            self.logical_cpu_count(),
            self.socket_count().unwrap_or(0),
            self.base_frequency_khz().unwrap_or(0),
            OptionalBool::from(self.virtualization_supported()) as u8,
            OptionalBool::from(self.is_virtual_machine()) as u8,
            self.l1_combined_cache().unwrap_or(0),
            self.l2_cache().unwrap_or(0),
            self.l3_cache().unwrap_or(0),
            self.l4_cache().unwrap_or(0),
        ));
    }
}

pub trait CpuDynamicInfoExt<'a>: Append + Arg {
    type Iter: Iterator<Item = &'a f32>;

    fn overall_utilization_percent(&self) -> f32;
    fn overall_kernel_utilization_percent(&self) -> f32;
    fn per_logical_cpu_utilization_percent(&'a self) -> Self::Iter;
    fn per_logical_cpu_kernel_utilization_percent(&'a self) -> Self::Iter;
    fn current_frequency_mhz(&self) -> u64;
    fn temperature(&self) -> Option<f32>;
    fn process_count(&self) -> u64;
    fn thread_count(&self) -> u64;
    fn handle_count(&self) -> u64;
    fn uptime_seconds(&self) -> u64;
}

impl Arg for crate::platform::CpuDynamicInfo {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Struct;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("(ddadadtdtttt)")
    }
}

impl Append for crate::platform::CpuDynamicInfo {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append((
            self.overall_utilization_percent() as f64,
            self.overall_kernel_utilization_percent() as f64,
            self.per_logical_cpu_utilization_percent()
                .map(|v| *v as f64)
                .collect::<Vec<_>>(),
            self.per_logical_cpu_kernel_utilization_percent()
                .map(|v| *v as f64)
                .collect::<Vec<_>>(),
            self.current_frequency_mhz(),
            self.temperature().map_or(0_f64, |v| v as f64),
            self.process_count(),
            self.thread_count(),
            self.handle_count(),
            self.uptime_seconds(),
        ));
    }
}

/// Provides an interface for gathering CPU/System information.
pub trait CpuInfoExt<'a> {
    type S: CpuStaticInfoExt;
    type D: CpuDynamicInfoExt<'a>;
    type P: crate::platform::ProcessesExt<'a>;

    /// Refresh the internal static information cache
    ///
    /// It is expected that implementors of this trait cache this information, once obtained
    /// from the underlying OS
    ///
    /// It is expected that this is only called one during the lifetime of this instance, but
    /// implementation should not rely on this.
    fn refresh_static_info_cache(&mut self);

    /// Refresh the internal dynamic/continuously changing information cache
    ///
    /// It is expected that implementors of this trait cache this information, once obtained
    /// from the underlying OS
    fn refresh_dynamic_info_cache(&mut self, processes: &Self::P);

    /// Implementation specific understanding of whether the cache is too old to be relevant
    fn is_dynamic_info_cache_stale(&self) -> bool;

    /// Returns the static information for the CPU.
    fn static_info(&self) -> &Self::S;

    /// Returns the dynamic information for the CPU.
    fn dynamic_info(&self) -> &Self::D;
}
