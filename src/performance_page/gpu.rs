/* performance_page/gpu.rs
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

use adw;
use adw::subclass::prelude::*;
use glib::{clone, ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*};

use super::widgets::GraphWidget;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageGpu)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/gpu.ui")]
    pub struct PerformancePageGpu {
        #[template_child]
        pub gpu_id: TemplateChild<gtk::Label>,
        #[template_child]
        pub device_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub overall_percent: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph_overall: TemplateChild<GraphWidget>,
        #[template_child]
        pub encode_percent: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph_encode: TemplateChild<GraphWidget>,
        #[template_child]
        pub decode_percent: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph_decode: TemplateChild<GraphWidget>,
        #[template_child]
        pub total_memory: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph_memory: TemplateChild<GraphWidget>,
        #[template_child]
        pub utilization: TemplateChild<gtk::Label>,
        #[template_child]
        pub memory_usage: TemplateChild<gtk::Label>,
        #[template_child]
        pub clock_speed: TemplateChild<gtk::Label>,
        #[template_child]
        pub memory_speed: TemplateChild<gtk::Label>,
        #[template_child]
        pub power_draw: TemplateChild<gtk::Label>,
        #[template_child]
        pub temperature: TemplateChild<gtk::Label>,
        #[template_child]
        pub opengl_version: TemplateChild<gtk::Label>,
        #[template_child]
        pub vulkan_version: TemplateChild<gtk::Label>,
        #[template_child]
        pub pcie_speed: TemplateChild<gtk::Label>,
        #[template_child]
        pub pci_addr: TemplateChild<gtk::Label>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get = Self::name, set = Self::set_name, type = String)]
        name: Cell<String>,
        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,
    }

    impl Default for PerformancePageGpu {
        fn default() -> Self {
            Self {
                gpu_id: Default::default(),
                device_name: Default::default(),
                overall_percent: Default::default(),
                usage_graph_overall: Default::default(),
                encode_percent: Default::default(),
                usage_graph_encode: Default::default(),
                decode_percent: Default::default(),
                usage_graph_decode: Default::default(),
                total_memory: Default::default(),
                usage_graph_memory: Default::default(),
                utilization: Default::default(),
                memory_usage: Default::default(),
                clock_speed: Default::default(),
                memory_speed: Default::default(),
                power_draw: Default::default(),
                temperature: Default::default(),
                opengl_version: Default::default(),
                vulkan_version: Default::default(),
                pcie_speed: Default::default(),
                pci_addr: Default::default(),
                context_menu: Default::default(),

                name: Cell::new(String::new()),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),
            }
        }
    }

    impl PerformancePageGpu {
        fn name(&self) -> String {
            unsafe { &*self.name.as_ptr() }.clone()
        }

        fn set_name(&self, name: String) {
            {
                let if_name = unsafe { &*self.name.as_ptr() };
                if if_name == &name {
                    return;
                }
            }

            self.name.replace(name);
        }
    }

    impl PerformancePageGpu {
        fn configure_actions(this: &super::PerformancePageGpu) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let action = gio::SimpleAction::new("copy", None);
            action.connect_activate(clone!(@weak this => move |_, _| {
                let clipboard = this.clipboard();
                clipboard.set_text(this.imp().data_summary().as_str());
            }));
            actions.add_action(&action);
        }

        fn configure_context_menu(this: &super::PerformancePageGpu) {
            let right_click_controller = gtk::GestureClick::new();
            right_click_controller.set_button(3); // Secondary click (AKA right click)
            right_click_controller.connect_released(
                clone!(@weak this => move |_click, _n_press, x, y| {
                    this
                        .imp()
                        .context_menu
                        .set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                            x.round() as i32,
                            y.round() as i32,
                            1,
                            1,
                        )));
                    this.imp().context_menu.popup();
                }),
            );
            this.add_controller(right_click_controller);
        }
    }

    impl PerformancePageGpu {
        pub fn set_static_information(
            this: &super::PerformancePageGpu,
            index: usize,
            gpu: &crate::sys_info_v2::GPU,
        ) -> bool {
            use gettextrs::gettext;

            this.imp()
                .usage_graph_overall
                .connect_resize(clone!(@weak this => move |_, _, _| {
                    let this = this.imp();

                    let width = this.usage_graph_overall.allocated_width() as f32;
                    let height = this.usage_graph_overall.allocated_height() as f32;

                    let mut a = width;
                    let mut b = height;
                    if width > height {
                        a = height;
                        b = width;
                    }

                    this.usage_graph_overall
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                    this.usage_graph_memory
                        .set_vertical_line_count((width / 40.).round() as u32);

                    let width = this.usage_graph_encode.allocated_width() as f32;
                    let height = this.usage_graph_encode.allocated_height() as f32;

                    let mut a = width;
                    let mut b = height;
                    if width > height {
                        a = height;
                        b = width;
                    }
                    this.usage_graph_encode
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);
                    this.usage_graph_decode
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);
                }));

            let this = this.imp();

            this.gpu_id.set_text(&format!("GPU {}", index));

            this.device_name.set_text(&gpu.static_info.device_name);

            this.usage_graph_memory
                .set_value_range_max(gpu.dynamic_info.total_memory as f32);

            let total_memory =
                crate::to_human_readable(gpu.dynamic_info.total_memory as f32, 1024.);
            this.total_memory
                .set_text(&format!("{} {}iB", total_memory.0.round(), total_memory.1));

            let opengl_version =
                if let Some(opengl_version) = gpu.static_info.opengl_version.as_ref() {
                    format!(
                        "{}{}.{}",
                        if opengl_version.2 { "ES " } else { "" },
                        opengl_version.0,
                        opengl_version.1
                    )
                } else {
                    gettext("Unknown")
                };
            this.opengl_version.set_text(&opengl_version);

            let vulkan_version =
                if let Some(vulkan_version) = gpu.static_info.vulkan_version.as_ref() {
                    format!(
                        "{}.{}.{}",
                        vulkan_version.0, vulkan_version.1, vulkan_version.2
                    )
                } else {
                    gettext("Unsupported")
                };
            this.vulkan_version.set_text(&vulkan_version);

            let pcie_info_known = if let Some(pcie_gen) = gpu.static_info.pcie_gen {
                if let Some(pcie_lanes) = gpu.static_info.pcie_lanes {
                    this.pcie_speed
                        .set_text(&format!("PCIe Gen {} x{} ", pcie_gen, pcie_lanes));
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !pcie_info_known {
                this.pcie_speed.set_text("Unknown");
            }

            this.pci_addr.set_text(&gpu.static_info.pci_slot_name);

            true
        }

        pub(crate) fn update_readings(
            this: &super::PerformancePageGpu,
            gpu: &crate::sys_info_v2::GPU,
        ) -> bool {
            let this = this.imp();

            this.overall_percent
                .set_text(&format!("{}%", gpu.dynamic_info.util_percent));
            this.usage_graph_overall
                .add_data_point(0, gpu.dynamic_info.util_percent as f32);
            this.utilization
                .set_text(&format!("{}%", gpu.dynamic_info.util_percent));

            this.encode_percent
                .set_text(&format!("{}%", gpu.dynamic_info.encoder_percent));
            this.usage_graph_encode
                .add_data_point(0, gpu.dynamic_info.encoder_percent as f32);

            this.decode_percent
                .set_text(&format!("{}%", gpu.dynamic_info.decoder_percent));
            this.usage_graph_decode
                .add_data_point(0, gpu.dynamic_info.decoder_percent as f32);

            this.usage_graph_memory
                .add_data_point(0, gpu.dynamic_info.used_memory as f32);

            let used_memory = crate::to_human_readable(gpu.dynamic_info.used_memory as f32, 1024.);
            let total_memory =
                crate::to_human_readable(gpu.dynamic_info.total_memory as f32, 1024.);
            this.memory_usage.set_text(&format!(
                "{:.2} {}iB / {:.2} {}iB",
                used_memory.0, used_memory.1, total_memory.0, total_memory.1
            ));

            let clock_speed = crate::to_human_readable(
                gpu.dynamic_info.clock_speed_mhz as f32 * 1_000_000.,
                1000.,
            );
            let clock_speed_max = crate::to_human_readable(
                gpu.dynamic_info.clock_speed_max_mhz as f32 * 1_000_000.,
                1000.,
            );
            this.clock_speed.set_text(&format!(
                "{:.2} {}Hz / {:.2} {}Hz",
                clock_speed.0, clock_speed.1, clock_speed_max.0, clock_speed_max.1
            ));

            let memory_speed =
                crate::to_human_readable(gpu.dynamic_info.mem_speed_mhz as f32 * 1_000_000., 1000.);
            let memory_speed_max = crate::to_human_readable(
                gpu.dynamic_info.mem_speed_max_mhz as f32 * 1_000_000.,
                1000.,
            );
            this.memory_speed.set_text(&format!(
                "{:.2} {}Hz / {:.2} {}Hz",
                memory_speed.0, memory_speed.1, memory_speed_max.0, memory_speed_max.1
            ));

            let power_draw =
                crate::to_human_readable(gpu.dynamic_info.power_draw_watts as f32, 1000.);
            let power_limit =
                crate::to_human_readable(gpu.dynamic_info.power_draw_max_watts as f32, 1000.);
            this.power_draw.set_text(&format!(
                "{:.2} {}W / {:.2} {}W",
                power_draw.0, power_draw.1, power_limit.0, power_limit.1
            ));

            this.temperature
                .set_text(&format!("{}°C", gpu.dynamic_info.temp_celsius));

            true
        }

        fn data_summary(&self) -> String {
            format!(
                r#"{}

    {}

    OpenGL version:    {}
    Vulkan version:    {}
    PCI Express speed: {}
    PCI bus address:   {}

    Utilization:  {}
    Memory usage: {}
    Clock speed:  {}
    Memory speed: {}
    Power draw:   {}
    Temperature:  {}"#,
                self.gpu_id.label(),
                self.device_name.label(),
                self.opengl_version.label(),
                self.vulkan_version.label(),
                self.pcie_speed.label(),
                self.pci_addr.label(),
                self.overall_percent.label(),
                self.memory_usage.label(),
                self.clock_speed.label(),
                self.memory_speed.label(),
                self.power_draw.label(),
                self.temperature.label(),
            )
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePageGpu {
        const NAME: &'static str = "PerformancePageGpu";
        type Type = super::PerformancePageGpu;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePageGpu {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageGpu>().clone();

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);
        }

        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for PerformancePageGpu {}

    impl BoxImpl for PerformancePageGpu {}
}

glib::wrapper! {
    pub struct PerformancePageGpu(ObjectSubclass<imp::PerformancePageGpu>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePageGpu {
    pub fn new(name: &str) -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(Self::static_type(), &mut [("name", name.into())])
                .downcast()
                .unwrap()
        };

        this
    }

    pub fn set_static_information(&self, index: usize, gpu: &crate::sys_info_v2::GPU) -> bool {
        imp::PerformancePageGpu::set_static_information(self, index, gpu)
    }

    pub fn update_readings(&self, gpu: &crate::sys_info_v2::GPU) -> bool {
        imp::PerformancePageGpu::update_readings(self, gpu)
    }
}
