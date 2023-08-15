/* apps_page/view_model.rs
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

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ViewModel)]
    pub struct ViewModel {
        #[property(get, set)]
        pub pid: Cell<u32>,

        #[property(get = Self::icon, set = Self::set_icon, type = glib::GString)]
        pub icon: Cell<glib::GString>,
        #[property(get = Self::name, set = Self::set_name, type = glib::GString)]
        pub name: Cell<glib::GString>,
        #[property(get = Self::id, set = Self::set_id, type = glib::GString)]
        pub id: Cell<glib::GString>,

        #[property(get = Self::content_type, type = u8)]
        pub content_type: Cell<ContentType>,
        #[property(get = Self::section_type, type = u8)]
        pub section_type: Cell<SectionType>,
        #[property(get, set)]
        pub show_expander: Cell<bool>,

        #[property(get, set = Self::set_cpu_usage)]
        pub cpu_usage: Cell<f32>,
        #[property(get, set = Self::set_memory_usage)]
        pub memory_usage: Cell<f32>,
        #[property(get, set)]
        pub disk_usage: Cell<f32>,
        #[property(get, set)]
        pub network_usage: Cell<f32>,
        #[property(get, set)]
        pub gpu_usage: Cell<f32>,

        #[property(get)]
        pub cpu_usage_percent: Cell<f32>,
        #[property(get)]
        pub memory_usage_percent: Cell<f32>,

        pub max_cpu_usage: Cell<f32>,
        pub max_memory_usage: Cell<f32>,

        pub children: Cell<gio::ListStore>,
    }

    impl Default for ViewModel {
        fn default() -> Self {
            Self {
                pid: Cell::new(0),

                icon: Cell::new(glib::GString::default()),
                name: Cell::new(glib::GString::default()),
                id: Cell::new(glib::GString::default()),

                content_type: Cell::new(ContentType::SectionHeader),
                section_type: Cell::new(SectionType::Apps),
                show_expander: Cell::new(true),

                cpu_usage: Cell::new(0.),
                memory_usage: Cell::new(0.),
                disk_usage: Cell::new(0.),
                network_usage: Cell::new(0.),
                gpu_usage: Cell::new(0.),

                cpu_usage_percent: Cell::new(0.),
                memory_usage_percent: Cell::new(0.),

                max_cpu_usage: Cell::new(0.),
                max_memory_usage: Cell::new(0.),

                children: Cell::new(gio::ListStore::new(super::ViewModel::static_type())),
            }
        }
    }

    impl ViewModel {
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

        pub fn id(&self) -> glib::GString {
            let id = self.id.take();
            let result = id.clone();
            self.id.set(id);

            result
        }

        pub fn set_id(&self, id: &str) {
            let current_id = self.id.take();
            if current_id == id {
                self.id.set(current_id);
                return;
            }

            self.id.set(glib::GString::from(id));
        }

        pub fn set_cpu_usage(&self, cpu_usage: f32) {
            self.cpu_usage.set(cpu_usage);

            let usage_precent = if self.max_cpu_usage.get() == 0. {
                0.
            } else {
                self.cpu_usage.get() * 100.0 / self.max_cpu_usage.get()
            };

            self.cpu_usage_percent.set(usage_precent);
            self.obj().notify_cpu_usage_percent();
        }

        pub fn set_memory_usage(&self, memory_usage: f32) {
            self.memory_usage.set(memory_usage);

            let usage_precent = if self.max_memory_usage.get() == 0. {
                0.
            } else {
                self.memory_usage.get() * 100.0 / self.max_memory_usage.get()
            };

            self.memory_usage_percent.set(usage_precent);
            self.obj().notify_memory_usage_percent();
        }

        pub fn content_type(&self) -> u8 {
            self.content_type.get() as _
        }

        pub fn section_type(&self) -> u8 {
            self.section_type.get() as _
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ViewModel {
        const NAME: &'static str = "ViewModel";
        type Type = super::ViewModel;
    }

    impl ObjectImpl for ViewModel {
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

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentType {
    SectionHeader,
    App,
    Process,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SectionType {
    Apps,
    Processes,
}

pub struct ViewModelBuilder {
    pid: u32,
    icon: glib::GString,
    name: glib::GString,
    id: glib::GString,

    content_type: ContentType,
    section_type: SectionType,
    show_expander: Option<bool>,

    cpu_usage: f32,
    memory_usage: f32,
    disk_usage: f32,
    network_usage: f32,
    gpu_usage: f32,
    max_cpu_usage: f32,
    max_memory_usage: f32,
}

impl ViewModelBuilder {
    pub fn new() -> Self {
        Self {
            pid: 0,
            icon: "application-x-executable-symbolic".into(),
            name: glib::GString::default(),
            id: glib::GString::default(),

            content_type: ContentType::SectionHeader,
            section_type: SectionType::Apps,
            show_expander: None,

            cpu_usage: 0.,
            memory_usage: 0.,
            disk_usage: 0.,
            network_usage: 0.,
            gpu_usage: 0.,

            max_cpu_usage: 0.,
            max_memory_usage: 0.,
        }
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

    pub fn id(mut self, id: &str) -> Self {
        self.id = id.into();
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

    pub fn show_expander(mut self, show_expander: bool) -> Self {
        self.show_expander = Some(show_expander);
        self
    }

    pub fn cpu_usage(mut self, cpu_usage: f32) -> Self {
        self.cpu_usage = cpu_usage;
        self
    }

    pub fn memory_usage(mut self, memory_usage: f32) -> Self {
        self.memory_usage = memory_usage;
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

    pub fn max_cpu_usage(mut self, v: f32) -> Self {
        self.max_cpu_usage = v;
        self
    }

    pub fn max_memory_usage(mut self, v: f32) -> Self {
        self.max_memory_usage = v;
        self
    }

    pub fn build(self) -> ViewModel {
        let this = ViewModel::new(self.content_type, self.show_expander);

        {
            let this = this.imp();
            this.pid.set(self.pid);
            this.icon.set(self.icon);
            this.name.set(self.name);
            this.id.set(self.id);
            this.section_type.set(self.section_type);
            this.cpu_usage.set(self.cpu_usage);
            this.memory_usage.set(self.memory_usage);
            this.disk_usage.set(self.disk_usage);
            this.network_usage.set(self.network_usage);
            this.gpu_usage.set(self.gpu_usage);
            this.max_cpu_usage.set(self.max_cpu_usage);
            this.max_memory_usage.set(self.max_memory_usage);
        }

        this
    }
}

glib::wrapper! {
    pub struct ViewModel(ObjectSubclass<imp::ViewModel>);
}

impl ViewModel {
    pub fn new(content_type: ContentType, show_expander: Option<bool>) -> Self {
        use gtk::glib::clone;
        use gtk::prelude::*;

        let this: Self = glib::Object::builder().build();
        this.imp().content_type.set(content_type);

        if show_expander.is_none() {
            glib::idle_add_local_once(clone!(@weak this => move || {
                this.set_show_expander(this.children().n_items() > 0);
            }));

            this.children()
                .connect_items_changed(clone!(@weak this => move |_, _, _, _| {
                    this.set_show_expander(this.children().n_items() > 0);
                }));
        } else {
            this.set_show_expander(show_expander.unwrap());
        }

        this
    }

    pub fn children(&self) -> &gio::ListStore {
        unsafe { &*self.imp().children.as_ptr() }
    }
}
