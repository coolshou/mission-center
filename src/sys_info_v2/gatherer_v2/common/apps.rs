/* sys_info_v2/gatherer/common/apps.rs
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

use super::{ArrayString, ArrayVec, ProcessStats};

pub type Stats = ProcessStats;

#[derive(Debug, Clone)]
pub struct AppDescriptor {
    pub name: ArrayString,
    pub icon: Option<ArrayString>,
    pub id: ArrayString,
    pub command: ArrayString,
    pub stats: Stats,
}

impl Default for AppDescriptor {
    fn default() -> Self {
        Self {
            name: Default::default(),
            icon: None,
            id: Default::default(),
            command: Default::default(),
            stats: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Apps {
    pub apps: ArrayVec<AppDescriptor, 25>,
    pub is_complete: bool,
}

impl Default for Apps {
    fn default() -> Self {
        Self {
            apps: ArrayVec::new(),
            is_complete: false,
        }
    }
}

// PIDs for the running apps. The order is the same as the order of the apps in the `Apps` struct.
// Each group of PIDs for an app is separated by a `0` value.
#[derive(Debug, Default, Clone)]
pub struct AppPIDs {
    pub pids: ArrayVec<u32, 100>,
    pub is_complete: bool,
}
