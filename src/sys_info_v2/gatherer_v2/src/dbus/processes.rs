/* sys_info_v2/gatherer/src/dbus/processes.rs
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

use crate::platform::{ProcessExt, ProcessState, ProcessUsageStats};
type PlatformProcess = crate::platform::Process;

#[derive(Debug)]
pub struct Process<'a> {
    pub name: &'a str,
    pub cmd: <PlatformProcess as ProcessExt<'a>>::Iter,
    pub exe: &'a str,
    pub state: ProcessState,
    pub pid: u32,
    pub parent: u32,
    pub usage_stats: &'a ProcessUsageStats,
    pub task_count: usize,
}
