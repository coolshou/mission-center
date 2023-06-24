/* apps_page/view_models/process_model.rs
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

use std::cell::Cell;

use gtk::{
    gio, glib,
    glib::{prelude::*, subclass::prelude::*, ParamSpec, Properties, Value},
};

pub type Pid = crate::sys_info_v2::Pid;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ProcessModel)]
    pub struct ProcessModel {
        #[property(get, set)]
        pid: Cell<Pid>,
        #[property(get, set)]
        cpu_usage: Cell<f32>,
        #[property(get, set)]
        memory_usage: Cell<f32>,
        #[property(get, set)]
        disk_usage: Cell<f32>,
        #[property(get, set)]
        network_usage: Cell<f32>,
        #[property(get, set)]
        gpu_usage: Cell<f32>,

        pub children: Cell<gio::ListStore>,
    }

    impl Default for ProcessModel {
        fn default() -> Self {
            Self {
                pid: Cell::new(0),
                cpu_usage: Cell::new(0.),
                memory_usage: Cell::new(0.),
                disk_usage: Cell::new(0.),
                network_usage: Cell::new(0.),
                gpu_usage: Cell::new(0.),

                children: Cell::new(gio::ListStore::new(super::super::ViewModel::static_type())),
            }
        }
    }

    impl ProcessModel {}

    #[glib::object_subclass]
    impl ObjectSubclass for ProcessModel {
        const NAME: &'static str = "ProcessModel";
        type Type = super::ProcessModel;
    }

    impl ObjectImpl for ProcessModel {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }
}

glib::wrapper! {
    pub struct ProcessModel(ObjectSubclass<imp::ProcessModel>);
}

impl super::ViewModelContent for ProcessModel {}

impl ProcessModel {
    pub fn new(
        pid: Pid,
        cpu_usage: f32,
        memory_usage: f32,
        disk_usage: f32,
        network_usage: f32,
        gpu_usage: f32,
    ) -> Self {
        let this: Self = glib::Object::builder()
            .property("pid", pid)
            .property("cpu-usage", cpu_usage)
            .property("memory-usage", memory_usage)
            .property("disk-usage", disk_usage)
            .property("network-usage", network_usage)
            .property("gpu-usage", gpu_usage)
            .build();
        this
    }

    pub fn children(&self) -> &gio::ListStore {
        unsafe { &*self.imp().children.as_ptr() }
    }

    pub fn set_children(&self, children: gio::ListStore) {
        self.imp().children.set(children);
    }
}
