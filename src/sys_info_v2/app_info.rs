/* sys_info_v2/app_info.rs
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

pub type Stats = super::gatherer::AppStats;

#[derive(Debug, Default, Clone)]
pub struct App {
    base: super::gatherer::AppDescriptor,
    pub pids: Vec<u32>,
}

impl App {
    pub fn new(base: super::gatherer::AppDescriptor) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.base.name
    }

    #[inline]
    pub fn icon(&self) -> Option<&str> {
        self.base.icon.as_deref()
    }

    #[inline]
    pub fn id(&self) -> &str {
        self.base.id.as_str()
    }

    #[inline]
    pub fn command(&self) -> &str {
        self.base.command.as_str()
    }

    #[inline]
    pub fn stats(&self) -> &Stats {
        &self.base.stats
    }
}
