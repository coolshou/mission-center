/* sys_info_v2/gatherer/src/platform/mod.rs
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

pub use cpu_ext::{cpu, CpuInfoExt};
pub use gpu_ext::{gpu, GpuInfoExt};
pub use platform_impl::{CpuInfo, GpuInfo};

mod cpu_ext;
mod gpu_ext;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform_impl;
