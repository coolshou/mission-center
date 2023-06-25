/* common/app.rs
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

#[derive(Debug, Default, Copy, Clone)]
pub struct Stats {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_usage: f32,
    pub gpu_usage: f32,
}

#[derive(Debug, Clone)]
pub struct App {
    pub name: String,
    pub command: String,
    pub icon: Option<String>,

    pub app_id: Option<String>,
    pub is_flatpak: bool,

    pub pids: Vec<libc::pid_t>,
    pub stats: Stats,
}

impl App {
    #[allow(dead_code)]
    pub fn serialize<W: std::io::Write>(&self, output: &mut W) -> std::io::Result<()> {
        output.write(to_binary(&self.name.len()))?;
        output.write(self.name.as_bytes())?;
        output.write(to_binary(&self.command.len()))?;
        output.write(self.command.as_bytes())?;
        if self.icon.is_some() {
            output.write(to_binary(&true))?;
            let icon = self.icon.as_ref().unwrap();
            output.write(to_binary(&icon.len()))?;
            output.write(icon.as_bytes())?;
        } else {
            output.write(to_binary(&false))?;
        }
        if self.app_id.is_some() {
            output.write(to_binary(&true))?;
            let app_id = self.app_id.as_ref().unwrap();
            output.write(to_binary(&app_id.len()))?;
            output.write(app_id.as_bytes())?;
        } else {
            output.write(to_binary(&false))?;
        }
        output.write(to_binary(&self.is_flatpak))?;
        output.write(to_binary(&self.pids.len()))?;
        for pid in &self.pids {
            output.write(to_binary(&pid))?;
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn deserialize<R: std::io::Read>(input: &mut R) -> std::io::Result<App> {
        let mut this = Self {
            name: "".to_string(),
            command: "".to_string(),
            icon: None,
            app_id: None,
            is_flatpak: false,
            pids: vec![],
            stats: Stats {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                disk_usage: 0.0,
                network_usage: 0.0,
                gpu_usage: 0.0,
            },
        };

        let mut len = 0_usize;

        input.read_exact(to_binary_mut(&mut len))?;
        let mut name = vec![0; len];
        input.read_exact(&mut name)?;
        this.name = unsafe { String::from_utf8_unchecked(name) };

        input.read_exact(to_binary_mut(&mut len))?;
        let mut command = vec![0; len];
        input.read_exact(&mut command)?;
        this.command = unsafe { String::from_utf8_unchecked(command) };

        let mut has_icon = false;
        input.read_exact(to_binary_mut(&mut has_icon))?;
        if has_icon {
            input.read_exact(to_binary_mut(&mut len))?;
            let mut icon = vec![0; len];
            input.read_exact(&mut icon)?;
            this.icon = Some(unsafe { String::from_utf8_unchecked(icon) });
        } else {
            this.icon = None;
        }

        let mut has_app_id = false;
        input.read_exact(to_binary_mut(&mut has_app_id))?;
        if has_app_id {
            input.read_exact(to_binary_mut(&mut len))?;
            let mut app_id = vec![0; len];
            input.read_exact(&mut app_id)?;
            this.app_id = Some(unsafe { String::from_utf8_unchecked(app_id) });
        } else {
            this.app_id = None;
        }

        input.read_exact(to_binary_mut(&mut this.is_flatpak))?;

        input.read_exact(to_binary_mut(&mut len))?;
        this.pids = Vec::with_capacity(len);
        for _ in 0..len {
            let mut pid = 0;
            input.read_exact(to_binary_mut(&mut pid))?;
            this.pids.push(pid);
        }

        Ok(this)
    }
}
