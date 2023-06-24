/* apps_page/view_models/app_model.rs
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
    glib,
    glib::{prelude::*, subclass::prelude::*, ParamSpec, Properties, Value},
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::AppModel)]
    pub struct AppModel {
        #[property(get = Self::icon, set = Self::set_icon, type = glib::GString)]
        icon: Cell<glib::GString>,

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
    }

    impl Default for AppModel {
        fn default() -> Self {
            Self {
                icon: Cell::new(glib::GString::default()),

                cpu_usage: Cell::new(0.),
                memory_usage: Cell::new(0.),
                disk_usage: Cell::new(0.),
                network_usage: Cell::new(0.),
                gpu_usage: Cell::new(0.),
            }
        }
    }

    impl AppModel {
        pub fn icon(&self) -> glib::GString {
            let icon = self.icon.take();
            let result = icon.clone();
            self.icon.set(icon);

            result
        }

        pub fn set_icon(&self, icon: &str) {
            let current_icon = self.icon.take();
            if current_icon == icon {
                self.icon.set(current_icon);
                return;
            }

            self.icon.set(glib::GString::from(icon));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppModel {
        const NAME: &'static str = "AppModel";
        type Type = super::AppModel;
    }

    impl ObjectImpl for AppModel {
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
    pub struct AppModel(ObjectSubclass<imp::AppModel>);
}

impl super::ViewModelContent for AppModel {}

impl AppModel {
    pub fn new(
        icon: &str,
        cpu_usage: f32,
        memory_usage: f32,
        disk_usage: f32,
        network_usage: f32,
        gpu_usage: f32,
    ) -> Self {
        let this: Self = glib::Object::builder()
            .property("icon", icon)
            .property("cpu-usage", cpu_usage)
            .property("memory-usage", memory_usage)
            .property("disk-usage", disk_usage)
            .property("network-usage", network_usage)
            .property("gpu-usage", gpu_usage)
            .build();
        this
    }
}
