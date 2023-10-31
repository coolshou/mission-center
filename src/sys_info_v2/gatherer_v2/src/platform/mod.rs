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

pub use apps::*;
pub use processes::*;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform_impl;

#[cfg(target_os = "linux")]
#[allow(unused)]
mod linux {
    use super::*;
    pub type Process = platform_impl::LinuxProcess;
    pub type Processes = platform_impl::LinuxProcesses;
    pub type App = platform_impl::LinuxApp;
    pub type Apps = platform_impl::LinuxApps;
}

#[cfg(target_os = "linux")]
pub use linux::*;

mod apps;
mod processes;
