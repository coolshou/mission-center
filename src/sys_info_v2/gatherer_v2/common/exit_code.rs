/* sys_info_v2/gatherer/common/exit_code.rs
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

#[repr(u8)]
#[derive(Debug)]
pub enum ExitCode {
    MissingProgramArgument = 1,
    IPCSocketNotFound,
    FileLinkNotFound,
    UnableToCreateSharedMemory,
    SocketConnectionFailed,
    ReadFromSocketFailed,
    SendDataReadyFailed,
    SendAcknowledgeFailed,
    OSSignal,
    UnknownMessageReceived = 254,
    Unknown = 255,
}

impl From<u8> for ExitCode {
    fn from(value: u8) -> Self {
        match value {
            1 => ExitCode::MissingProgramArgument,
            2 => ExitCode::IPCSocketNotFound,
            3 => ExitCode::FileLinkNotFound,
            4 => ExitCode::UnableToCreateSharedMemory,
            5 => ExitCode::SocketConnectionFailed,
            6 => ExitCode::ReadFromSocketFailed,
            7 => ExitCode::SendDataReadyFailed,
            8 => ExitCode::SendAcknowledgeFailed,
            9 => ExitCode::OSSignal,
            254 => ExitCode::UnknownMessageReceived,
            _ => ExitCode::Unknown,
        }
    }
}

impl From<ExitCode> for u8 {
    fn from(value: ExitCode) -> Self {
        value as u8
    }
}
