/* performance_page/gpu_details.rs
 *
 * Copyright 2024 Mission Center Developers
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

use glib::{ParamSpec, Properties, Value};
use gtk::{gdk::prelude::*, glib, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::GpuDetails)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/gpu_details.ui")]
    pub struct GpuDetails {
        #[template_child]
        pub utilization: TemplateChild<gtk::Label>,
        #[template_child]
        pub memory_usage_current: TemplateChild<gtk::Label>,
        #[template_child]
        pub memory_usage_max: TemplateChild<gtk::Label>,
        #[template_child]
        pub gtt_usage_current: TemplateChild<gtk::Label>,
        #[template_child]
        pub gtt_usage_max: TemplateChild<gtk::Label>,
        #[template_child]
        pub clock_speed_current: TemplateChild<gtk::Label>,
        #[template_child]
        pub clock_speed_max: TemplateChild<gtk::Label>,
        #[template_child]
        pub memory_speed_current: TemplateChild<gtk::Label>,
        #[template_child]
        pub memory_speed_max: TemplateChild<gtk::Label>,
        #[template_child]
        pub power_draw_current: TemplateChild<gtk::Label>,
        #[template_child]
        pub power_draw_max: TemplateChild<gtk::Label>,
        #[template_child]
        pub encode_percent: TemplateChild<gtk::Label>,
        #[template_child]
        pub decode_percent: TemplateChild<gtk::Label>,
        #[template_child]
        pub temperature: TemplateChild<gtk::Label>,
        #[template_child]
        pub opengl_version: TemplateChild<gtk::Label>,
        #[template_child]
        pub vulkan_version: TemplateChild<gtk::Label>,
        #[template_child]
        pub pcie_speed_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub pcie_speed: TemplateChild<gtk::Label>,
        #[template_child]
        pub pci_addr: TemplateChild<gtk::Label>,

        #[template_child]
        pub box_temp: TemplateChild<gtk::Box>,
        #[template_child]
        pub box_mem_speed: TemplateChild<gtk::Box>,
        #[template_child]
        pub box_mem_usage: TemplateChild<gtk::Box>,
        #[template_child]
        pub box_gtt_usage: TemplateChild<gtk::Box>,
        #[template_child]
        pub box_power_draw: TemplateChild<gtk::Box>,
        #[template_child]
        pub box_decode: TemplateChild<gtk::Box>,
        #[template_child]
        pub encode_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub legend_encode: TemplateChild<gtk::Picture>,
        #[template_child]
        pub legend_decode: TemplateChild<gtk::Picture>,
        #[template_child]
        pub legend_vram: TemplateChild<gtk::Picture>,
        #[template_child]
        pub legend_gtt: TemplateChild<gtk::Picture>,

        #[property(get)]
        total_mem_available: Cell<bool>,
    }

    impl Default for GpuDetails {
        fn default() -> Self {
            Self {
                utilization: TemplateChild::default(),
                memory_usage_current: TemplateChild::default(),
                memory_usage_max: TemplateChild::default(),
                gtt_usage_current: TemplateChild::default(),
                gtt_usage_max: TemplateChild::default(),
                clock_speed_current: TemplateChild::default(),
                clock_speed_max: TemplateChild::default(),
                memory_speed_current: TemplateChild::default(),
                memory_speed_max: TemplateChild::default(),
                power_draw_current: TemplateChild::default(),
                power_draw_max: TemplateChild::default(),
                encode_percent: TemplateChild::default(),
                decode_percent: TemplateChild::default(),
                temperature: TemplateChild::default(),
                opengl_version: TemplateChild::default(),
                vulkan_version: TemplateChild::default(),
                pcie_speed_label: TemplateChild::default(),
                pcie_speed: TemplateChild::default(),
                pci_addr: TemplateChild::default(),

                box_temp: TemplateChild::default(),
                box_mem_speed: TemplateChild::default(),
                box_mem_usage: TemplateChild::default(),
                box_gtt_usage: TemplateChild::default(),
                box_power_draw: TemplateChild::default(),
                box_decode: TemplateChild::default(),
                encode_label: TemplateChild::default(),

                legend_encode: TemplateChild::default(),
                legend_decode: TemplateChild::default(),
                legend_vram: TemplateChild::default(),
                legend_gtt: TemplateChild::default(),

                total_mem_available: Cell::new(true),
            }
        }
    }

    impl GpuDetails {}

    #[glib::object_subclass]
    impl ObjectSubclass for GpuDetails {
        const NAME: &'static str = "GpuDetails";
        type Type = super::GpuDetails;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for GpuDetails {
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for GpuDetails {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl BoxImpl for GpuDetails {}
}

glib::wrapper! {
    pub struct GpuDetails(ObjectSubclass<imp::GpuDetails>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Buildable;
}

impl GpuDetails {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn utilization(&self) -> gtk::Label {
        self.imp().utilization.clone()
    }

    pub fn memory_usage_current(&self) -> gtk::Label {
        self.imp().memory_usage_current.clone()
    }
    pub fn memory_usage_max(&self) -> gtk::Label {
        self.imp().memory_usage_max.clone()
    }

    pub fn gtt_usage_current(&self) -> gtk::Label {
        self.imp().gtt_usage_current.clone()
    }

    pub fn gtt_usage_max(&self) -> gtk::Label {
        self.imp().gtt_usage_max.clone()
    }

    pub fn clock_speed_current(&self) -> gtk::Label {
        self.imp().clock_speed_current.clone()
    }

    pub fn clock_speed_max(&self) -> gtk::Label {
        self.imp().clock_speed_max.clone()
    }

    pub fn memory_speed_current(&self) -> gtk::Label {
        self.imp().memory_speed_current.clone()
    }

    pub fn memory_speed_max(&self) -> gtk::Label {
        self.imp().memory_speed_max.clone()
    }

    pub fn power_draw_current(&self) -> gtk::Label {
        self.imp().power_draw_current.clone()
    }

    pub fn power_draw_max(&self) -> gtk::Label {
        self.imp().power_draw_max.clone()
    }

    pub fn encode_percent(&self) -> gtk::Label {
        self.imp().encode_percent.clone()
    }

    pub fn decode_percent(&self) -> gtk::Label {
        self.imp().decode_percent.clone()
    }

    pub fn temperature(&self) -> gtk::Label {
        self.imp().temperature.clone()
    }

    pub fn opengl_version(&self) -> gtk::Label {
        self.imp().opengl_version.clone()
    }

    pub fn vulkan_version(&self) -> gtk::Label {
        self.imp().vulkan_version.clone()
    }

    pub fn pcie_speed_label(&self) -> gtk::Label {
        self.imp().pcie_speed_label.clone()
    }

    pub fn pcie_speed(&self) -> gtk::Label {
        self.imp().pcie_speed.clone()
    }

    pub fn pci_addr(&self) -> gtk::Label {
        self.imp().pci_addr.clone()
    }

    pub fn box_temp(&self) -> gtk::Box {
        self.imp().box_temp.clone()
    }

    pub fn box_mem_speed(&self) -> gtk::Box {
        self.imp().box_mem_speed.clone()
    }

    pub fn box_mem_usage(&self) -> gtk::Box {
        self.imp().box_mem_usage.clone()
    }

    pub fn box_gtt_usage(&self) -> gtk::Box {
        self.imp().box_gtt_usage.clone()
    }

    pub fn box_power_draw(&self) -> gtk::Box {
        self.imp().box_power_draw.clone()
    }

    pub fn box_decode(&self) -> gtk::Box {
        self.imp().box_decode.clone()
    }

    pub fn encode_label(&self) -> gtk::Label {
        self.imp().encode_label.clone()
    }

    pub fn legend_encode(&self) -> gtk::Picture {
        self.imp().legend_encode.clone()
    }

    pub fn legend_decode(&self) -> gtk::Picture {
        self.imp().legend_decode.clone()
    }

    pub fn legend_vram(&self) -> gtk::Picture {
        self.imp().legend_vram.clone()
    }

    pub fn legend_gtt(&self) -> gtk::Picture {
        self.imp().legend_gtt.clone()
    }
}
