/* sys_info_v2/dbus-interface/apps.rs
 *
 * Copyright 2024 Romeo Calota
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

use std::{collections::HashMap, sync::Arc};

use dbus::{arg::*, strings::*};
use gtk::glib::g_critical;

use super::{deser_array, deser_str};

#[derive(Debug, Clone, Default)]
pub struct App {
    pub name: Arc<str>,
    pub icon: Option<Arc<str>>,
    pub id: Arc<str>,
    pub command: Arc<str>,
    pub pids: Vec<u32>,
}

impl From<&dyn RefArg> for App {
    fn from(value: &dyn RefArg) -> Self {
        let empty_string = Arc::<str>::from("");

        let mut this = App {
            name: empty_string.clone(),
            icon: None,
            id: empty_string.clone(),
            command: empty_string,
            pids: vec![],
        };

        let mut app = match value.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get App: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return this;
            }
            Some(i) => i,
        };
        let app = app.as_mut();

        this.name = match deser_str(app, "App", 0) {
            Some(name) => name,
            None => return this,
        };

        this.icon = match deser_str(app, "App", 1) {
            Some(icon) => {
                if icon.is_empty() {
                    None
                } else {
                    Some(icon)
                }
            }
            None => return this,
        };

        this.id = match deser_str(app, "App", 2) {
            Some(id) => id,
            None => return this,
        };

        this.command = match deser_str(app, "App", 3) {
            Some(command) => command,
            None => return this,
        };

        match deser_array(app, "App", 4) {
            Some(pids) => {
                for p in pids {
                    if let Some(p) = p.as_u64() {
                        this.pids.push(p as u32);
                    }
                }
            }
            None => {
                return this;
            }
        };

        this
    }
}

pub struct AppMap(HashMap<Arc<str>, App>);

impl From<HashMap<Arc<str>, App>> for AppMap {
    fn from(value: HashMap<Arc<str>, App>) -> Self {
        Self(value)
    }
}

impl From<AppMap> for HashMap<Arc<str>, App> {
    fn from(value: AppMap) -> Self {
        value.0
    }
}

impl Arg for AppMap {
    const ARG_TYPE: ArgType = ArgType::Array;

    fn signature() -> Signature<'static> {
        Signature::from("a(ssssau(dddddd))")
    }
}

impl ReadAll for AppMap {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for AppMap {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        let mut this = HashMap::new();

        match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get HashMap<AppId, App>: Expected '0: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get HashMap<AppId, APp>: Expected '0: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    for a in arr {
                        let a = App::from(a);
                        if a.name.as_ref().is_empty() {
                            continue;
                        }
                        this.insert(a.id.clone(), a);
                    }
                }
            },
        }

        Some(this.into())
    }
}
