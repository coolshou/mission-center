/* sys_info_v2/gatherer/src/platform/apps.rs
 *
 * Copyright 2023 Romeo Calota
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later versionBecomeMonitor.
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

pub type AppUsageStats = crate::platform::ProcessUsageStats;

/// A running application
pub trait AppExt<'a>: Default + Append + Arg {
    type Iter: Iterator<Item = &'a u32>;

    /// The name of the app in human readable form
    fn name(&self) -> &str;

    /// The icon used by the app
    fn icon(&self) -> Option<&str>;

    /// A platform-specific unique id
    fn id(&self) -> &str;

    /// The command used to launch the app
    fn command(&self) -> &str;

    /// The list of processes that the app uses
    ///
    /// It is expected that the the iterator yields the elements from smallest to largest
    fn pids(&'a self) -> Self::Iter;

    /// The system usage statistics for the app
    fn usage_stats(&self) -> &AppUsageStats;
}

impl Arg for crate::platform::App {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Struct;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("(ssssau(dddddd))")
    }
}

impl Append for crate::platform::App {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append((
            self.name(),
            self.icon().unwrap_or(""),
            self.id(),
            self.command(),
            self.pids().clone().collect::<Vec<_>>(),
            (
                self.usage_stats().cpu_usage as f64,
                self.usage_stats().memory_usage as f64,
                self.usage_stats().disk_usage as f64,
                self.usage_stats().network_usage as f64,
                self.usage_stats().gpu_usage as f64,
                self.usage_stats().gpu_memory_usage as f64,
            ),
        ));
    }
}

/// The public interface that describes how the list of running apps is obtained
pub trait AppsExt<'a>: Default + Append + Arg {
    type A: AppExt<'a>;
    type P: crate::platform::ProcessExt<'a>;

    /// Refresh the internal app cache
    ///
    /// It is expected that implementors of this trait cache the running app list once obtained from
    /// the underlying OS
    fn refresh_cache(&mut self, processes: &std::collections::HashMap<u32, Self::P>);

    /// Implementation specific understanding of whether the cache is too old to be relevant
    fn is_cache_stale(&self) -> bool;

    /// Return the list of (cached) running apps
    fn app_list(&self) -> &[Self::A];
}

impl Arg for crate::platform::Apps {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Array;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("a(ssssau(ddddd))")
    }
}

impl Append for crate::platform::Apps {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append(self.app_list())
    }
}
