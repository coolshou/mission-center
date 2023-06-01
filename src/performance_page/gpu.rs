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
use gtk::{gio, glib, prelude::*, Snapshot};

use super::widgets::GraphWidget;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageGpu)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/gpu.ui")]
    pub struct PerformancePageGpu {
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
        refresh_interval: Cell<u32>,
        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,
    }

    impl Default for PerformancePageGpu {
        fn default() -> Self {
            Self {
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
                refresh_interval: Cell::new(1000),
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
            self.update_static_information();
        }
    }

    impl PerformancePageGpu {
        fn configure_actions(this: &super::PerformancePageGpu) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));
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

        fn update_view(&self, this: &super::PerformancePageGpu) {
            use crate::SYS_INFO;

            self.update_graphs_grid_layout();

            let sys_info = SYS_INFO.read().expect("Failed to acquire read lock");
            let gpu_info = sys_info.gpu_info();
            if gpu_info.is_none() {
                return;
            }
            let gpu_info = gpu_info.unwrap();
            if let Some(gpu) = gpu_info
                .gpus()
                .iter()
                .filter(|d| {
                    d.device_name == self.obj().upcast_ref::<super::PerformancePageGpu>().name()
                })
                .take(1)
                .next()
            {
                self.overall_percent
                    .set_text(&format!("{}%", gpu.util_percent));
                self.usage_graph_overall
                    .add_data_point(0, gpu.util_percent as f32);
                self.utilization.set_text(&format!("{}%", gpu.util_percent));

                self.encode_percent
                    .set_text(&format!("{}%", gpu.encoder_percent));
                self.usage_graph_encode
                    .add_data_point(0, gpu.encoder_percent as f32);

                self.decode_percent
                    .set_text(&format!("{}%", gpu.decoder_percent));
                self.usage_graph_decode
                    .add_data_point(0, gpu.decoder_percent as f32);

                self.usage_graph_memory
                    .add_data_point(0, gpu.used_memory as f32);

                let used_memory = crate::to_human_readable(gpu.used_memory as f32, 1024.);
                let total_memory = crate::to_human_readable(gpu.total_memory as f32, 1024.);
                self.memory_usage.set_text(&format!(
                    "{:.2} {}iB / {:.2} {}iB",
                    used_memory.0, used_memory.1, total_memory.0, total_memory.1
                ));

                let clock_speed =
                    crate::to_human_readable(gpu.clock_speed_mhz as f32 * 1_000_000., 1000.);
                let clock_speed_max =
                    crate::to_human_readable(gpu.clock_speed_max_mhz as f32 * 1_000_000., 1000.);
                self.clock_speed.set_text(&format!(
                    "{:.2} {}Hz / {:.2} {}Hz",
                    clock_speed.0, clock_speed.1, clock_speed_max.0, clock_speed_max.1
                ));

                let memory_speed =
                    crate::to_human_readable(gpu.mem_speed_mhz as f32 * 1_000_000., 1000.);
                let memory_speed_max =
                    crate::to_human_readable(gpu.mem_speed_max_mhz as f32 * 1_000_000., 1000.);
                self.memory_speed.set_text(&format!(
                    "{:.2} {}Hz / {:.2} {}Hz",
                    memory_speed.0, memory_speed.1, memory_speed_max.0, memory_speed_max.1
                ));

                let power_draw = crate::to_human_readable(gpu.power_draw_watts as f32, 1000.);
                let power_limit = crate::to_human_readable(gpu.power_draw_max_watts as f32, 1000.);
                self.power_draw.set_text(&format!(
                    "{:.2} {}W / {:.2} {}W",
                    power_draw.0, power_draw.1, power_limit.0, power_limit.1
                ));

                self.temperature
                    .set_text(&format!("{}°C", gpu.temp_celsius));
            }

            let this = this.clone();
            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_view(this.imp(), &this);
                },
            ));
        }

        fn update_static_information(&self) {
            use crate::SYS_INFO;

            self.device_name.set_text(&self.name());

            let sys_info = SYS_INFO.read().expect("Failed to acquire read lock");
            let gpu_info = sys_info.gpu_info();
            if gpu_info.is_none() {
                return;
            }
            let gpu_info = gpu_info.unwrap();
            if let Some(gpu) = gpu_info
                .gpus()
                .iter()
                .filter(|d| {
                    d.device_name == self.obj().upcast_ref::<super::PerformancePageGpu>().name()
                })
                .take(1)
                .next()
            {
                self.usage_graph_memory
                    .set_value_range_max(gpu.total_memory as f32);

                let total_memory = crate::to_human_readable(gpu.total_memory as f32, 1024.);
                self.total_memory.set_text(&format!(
                    "{} {}iB",
                    total_memory.0.round(),
                    total_memory.1
                ));

                let pcie_info_known = if let Some(pcie_gen) = gpu.pcie_gen {
                    if let Some(pcie_lanes) = gpu.pcie_lanes {
                        self.pcie_speed
                            .set_text(&format!("{}x Gen {}", pcie_lanes, pcie_gen));
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !pcie_info_known {
                    self.pcie_speed.set_text("Unknown");
                }

                self.pci_addr.set_text(&gpu.pci_bus_id);
            }
        }

        fn update_graphs_grid_layout(&self) {
            let width = self.usage_graph_overall.allocated_width() as f32;
            let height = self.usage_graph_overall.allocated_height() as f32;

            let mut a = width;
            let mut b = height;
            if width > height {
                a = height;
                b = width;
            }

            self.usage_graph_overall
                .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

            self.usage_graph_memory
                .set_vertical_line_count((width / 40.).round() as u32);

            let width = self.usage_graph_encode.allocated_width() as f32;
            let height = self.usage_graph_encode.allocated_height() as f32;

            let mut a = width;
            let mut b = height;
            if width > height {
                a = height;
                b = width;
            }
            self.usage_graph_encode
                .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);
            self.usage_graph_decode
                .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);
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
            self.update_static_information();
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

    impl WidgetImpl for PerformancePageGpu {
        fn realize(&self) {
            self.parent_realize();

            self.update_view(self.obj().upcast_ref());
        }

        fn snapshot(&self, snapshot: &Snapshot) {
            self.parent_snapshot(snapshot);
            self.update_graphs_grid_layout();
        }
    }

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

    pub fn set_initial_values(&self, values: Vec<f32>) {
        self.imp().usage_graph_overall.set_data(0, values);
    }
}
