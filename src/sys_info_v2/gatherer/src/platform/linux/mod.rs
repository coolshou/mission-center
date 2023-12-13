/* sys_info_v2/gatherer/src/platform/linux/mod.rs
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

use lazy_static::lazy_static;

pub use apps::*;
pub use cpu_info::*;
pub use gpu_info::*;
pub use processes::*;
pub use utilities::*;

mod apps;
mod cpu_info;
mod gpu_info;
mod processes;
mod utilities;

extern "C" {
    fn get_nprocs_conf() -> libc::c_int;
}

lazy_static! {
    static ref HZ: usize = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as usize };
    static ref CPU_COUNT: usize = unsafe { get_nprocs_conf() as usize };
}
