/* apps_page/row_model.rs
 *
 * Copyright 2025 Mission Center Developers
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

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::RowModel)]
    pub struct RowModel {
        #[property(get, set)]
        pub pid: Cell<u32>,

        #[property(get = Self::app_id, set = Self::set_app_id)]
        pub app_id: Cell<glib::GString>,

        #[property(get = Self::icon, set = Self::set_icon)]
        pub icon: Cell<glib::GString>,
        #[property(get = Self::name, set = Self::set_name)]
        pub name: Cell<glib::GString>,

        #[property(get = Self::content_type, type = ContentType, builder(ContentType::SectionHeader))]
        pub content_type: Cell<ContentType>,
        #[property(get = Self::section_type, type = SectionType, builder(SectionType::Apps))]
        pub section_type: Cell<SectionType>,

        #[property(get, set = Self::set_cpu_usage)]
        pub cpu_usage: Cell<f32>,
        #[property(get, set = Self::set_memory_usage)]
        pub memory_usage: Cell<u64>,
        #[property(get, set = Self::set_shared_memory_usage)]
        pub shared_memory_usage: Cell<u64>,
        #[property(get, set)]
        pub disk_usage: Cell<f32>,
        #[property(get, set)]
        pub network_usage: Cell<f32>,
        #[property(get, set)]
        pub gpu_usage: Cell<f32>,
        #[property(get, set = Self::set_gpu_memory_usage)]
        pub gpu_memory_usage: Cell<u64>,

        #[property(get)]
        pub cpu_usage_percent: Cell<f32>,
        #[property(get)]
        pub memory_usage_percent: Cell<f32>,
        #[property(get)]
        pub gpu_memory_usage_percent: Cell<f32>,

        pub max_cpu_usage: Cell<f32>,
        pub max_memory_usage: Cell<u64>,
        pub max_gpu_memory_usage: Cell<u64>,

        pub merged_stats: Cell<Option<crate::magpie_client::ProcessUsageStats>>,

        pub children: gio::ListStore,
    }

    impl Default for RowModel {
        fn default() -> Self {
            Self {
                app_id: Cell::new(glib::GString::default()),
                pid: Cell::new(0),

                icon: Cell::new(glib::GString::default()),
                name: Cell::new(glib::GString::default()),

                content_type: Cell::new(ContentType::SectionHeader),
                section_type: Cell::new(SectionType::Apps),

                cpu_usage: Cell::new(0.),
                memory_usage: Cell::new(0),
                shared_memory_usage: Cell::new(0),
                disk_usage: Cell::new(0.),
                network_usage: Cell::new(0.),
                gpu_usage: Cell::new(0.),
                gpu_memory_usage: Cell::new(0),

                cpu_usage_percent: Cell::new(0.),
                memory_usage_percent: Cell::new(0.),
                gpu_memory_usage_percent: Cell::new(0.),

                max_cpu_usage: Cell::new(0.),
                max_memory_usage: Cell::new(0),
                max_gpu_memory_usage: Cell::new(0),

                merged_stats: Cell::new(None),

                children: gio::ListStore::new::<super::RowModel>(),
            }
        }
    }

    impl RowModel {
        pub fn app_id(&self) -> glib::GString {
            let app_id = self.app_id.take();
            let result = app_id.clone();
            self.app_id.set(app_id);

            result
        }

        pub fn set_app_id(&self, app_id: &str) {
            let current_app_id = self.app_id.take();
            if current_app_id == app_id {
                self.app_id.set(current_app_id);
                return;
            }

            self.app_id.set(glib::GString::from(app_id));
        }

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

        pub fn name(&self) -> glib::GString {
            let name = self.name.take();
            let result = name.clone();
            self.name.set(name);

            result
        }

        pub fn set_name(&self, name: &str) {
            let current_name = self.name.take();
            if current_name == name {
                self.name.set(current_name);
                return;
            }

            self.name.set(glib::GString::from(name));
        }

        pub fn set_cpu_usage(&self, cpu_usage: f32) {
            self.cpu_usage.set(cpu_usage);

            let usage_percent = if self.max_cpu_usage.get() == 0. {
                0.
            } else {
                self.cpu_usage.get() * 100.0 / self.max_cpu_usage.get()
            };

            self.cpu_usage_percent.set(usage_percent);
            self.obj().notify_cpu_usage_percent();
        }

        pub fn set_memory_usage(&self, memory_usage: u64) {
            self.memory_usage.set(memory_usage);

            let usage_percent = if self.max_memory_usage.get() == 0 {
                0.
            } else {
                (self.memory_usage.get() * 100) as f64 / self.max_memory_usage.get() as f64
            };

            self.memory_usage_percent.set(usage_percent as f32);
            self.obj().notify_memory_usage_percent();
        }

        pub fn set_shared_memory_usage(&self, memory_shared: u64) {
            self.shared_memory_usage.set(memory_shared);
            self.obj().notify_shared_memory_usage();
        }

        pub fn set_gpu_memory_usage(&self, memory_usage: u64) {
            self.gpu_memory_usage.set(memory_usage);

            let usage_percent = if self.max_gpu_memory_usage.get() == 0 {
                0.
            } else {
                (self.gpu_memory_usage.get() * 100) as f64 / self.max_gpu_memory_usage.get() as f64
            };

            self.gpu_memory_usage_percent.set(usage_percent as f32);
            self.obj().notify_gpu_memory_usage_percent();
        }

        pub fn content_type(&self) -> ContentType {
            self.content_type.get()
        }

        pub fn section_type(&self) -> SectionType {
            self.section_type.get()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RowModel {
        const NAME: &'static str = "RowModel";
        type Type = super::RowModel;
    }

    impl ObjectImpl for RowModel {
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, glib::Enum)]
#[enum_type(name = "ContentType")]
pub enum ContentType {
    SectionHeader,
    App,
    Process,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, glib::Enum)]
#[enum_type(name = "SectionType")]
pub enum SectionType {
    Apps,
    Processes,
}

pub struct RowModelBuilder {
    pid: u32,

    app_id: glib::GString,

    icon: glib::GString,
    name: glib::GString,

    content_type: ContentType,
    section_type: SectionType,

    cpu_usage: f32,
    memory_usage: u64,
    shared_memory_usage: u64,
    disk_usage: f32,
    network_usage: f32,
    gpu_usage: f32,
    gpu_mem_usage: u64,

    max_cpu_usage: f32,
    max_memory_usage: u64,
    max_gpu_memory_usage: u64,
}

impl RowModelBuilder {
    pub fn new() -> Self {
        Self {
            pid: 0,

            app_id: glib::GString::default(),

            icon: "application-x-executable-symbolic".into(),
            name: glib::GString::default(),

            content_type: ContentType::SectionHeader,
            section_type: SectionType::Apps,

            cpu_usage: 0.,
            memory_usage: 0,
            shared_memory_usage: 0,
            disk_usage: 0.,
            network_usage: 0.,
            gpu_usage: 0.,
            gpu_mem_usage: 0,

            max_cpu_usage: 0.,
            max_memory_usage: 0,
            max_gpu_memory_usage: 0,
        }
    }

    pub fn app_id(mut self, app_id: &str) -> Self {
        self.app_id = app_id.into();
        self
    }

    pub fn pid(mut self, pid: u32) -> Self {
        self.pid = pid;
        self
    }

    pub fn icon(mut self, icon: &str) -> Self {
        self.icon = icon.into();
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    pub fn content_type(mut self, content_type: ContentType) -> Self {
        self.content_type = content_type;
        self
    }

    pub fn section_type(mut self, section_type: SectionType) -> Self {
        self.section_type = section_type;
        self
    }

    pub fn cpu_usage(mut self, cpu_usage: f32) -> Self {
        self.cpu_usage = cpu_usage;
        self
    }

    pub fn memory_usage(mut self, memory_usage: u64) -> Self {
        self.memory_usage = memory_usage;
        self
    }

    pub fn shared_memory_usage(mut self, shared_memory_usage: u64) -> Self {
        self.shared_memory_usage = shared_memory_usage;
        self
    }

    pub fn disk_usage(mut self, disk_usage: f32) -> Self {
        self.disk_usage = disk_usage;
        self
    }

    pub fn network_usage(mut self, network_usage: f32) -> Self {
        self.network_usage = network_usage;
        self
    }

    pub fn gpu_usage(mut self, gpu_usage: f32) -> Self {
        self.gpu_usage = gpu_usage;
        self
    }

    pub fn gpu_mem_usage(mut self, gpu_mem_usage: u64) -> Self {
        self.gpu_mem_usage = gpu_mem_usage;
        self
    }

    pub fn max_cpu_usage(mut self, v: f32) -> Self {
        self.max_cpu_usage = v;
        self
    }

    pub fn max_memory_usage(mut self, v: u64) -> Self {
        self.max_memory_usage = v;
        self
    }

    pub fn max_gpu_memory_usage(mut self, v: u64) -> Self {
        self.max_gpu_memory_usage = v;
        self
    }

    pub fn build(self) -> RowModel {
        let this = RowModel::new(self.content_type);

        {
            let this = this.imp();

            this.app_id.set(self.app_id);
            this.pid.set(self.pid);
            this.icon.set(self.icon);
            this.name.set(self.name);

            this.section_type.set(self.section_type);

            this.set_cpu_usage(self.cpu_usage);
            this.set_memory_usage(self.memory_usage);
            this.set_shared_memory_usage(self.shared_memory_usage);
            this.disk_usage.set(self.disk_usage);
            this.network_usage.set(self.network_usage);
            this.gpu_usage.set(self.gpu_usage);
            this.set_gpu_memory_usage(self.gpu_mem_usage);

            this.max_cpu_usage.set(self.max_cpu_usage);
            this.max_memory_usage.set(self.max_memory_usage);
            this.max_gpu_memory_usage.set(self.max_gpu_memory_usage);
        }

        this
    }
}

glib::wrapper! {
    pub struct RowModel(ObjectSubclass<imp::RowModel>);
}

impl RowModel {
    pub fn new(content_type: ContentType) -> Self {
        let this: Self = glib::Object::builder().build();
        this.imp().content_type.set(content_type);

        this
    }

    pub fn merged_stats(&self) -> Option<crate::magpie_client::ProcessUsageStats> {
        self.imp().merged_stats.get()
    }

    pub fn set_merged_stats(&self, stats: crate::magpie_client::ProcessUsageStats) {
        self.imp().merged_stats.set(Some(stats));
    }

    pub fn children(&self) -> &gio::ListStore {
        &self.imp().children
    }
}
