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

use std::cell::{Cell, OnceCell};

use adw;
use adw::subclass::prelude::*;
use glib::{clone, ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*};

use crate::i18n::*;

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
        pub encode_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph_encode: TemplateChild<GraphWidget>,
        #[template_child]
        pub decode_graph: TemplateChild<gtk::Box>,
        #[template_child]
        pub decode_percent: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph_decode: TemplateChild<GraphWidget>,
        #[template_child]
        pub memory_graph: TemplateChild<gtk::Box>,
        #[template_child]
        pub total_memory: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph_memory: TemplateChild<GraphWidget>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get = Self::name, set = Self::set_name, type = String)]
        name: Cell<String>,
        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,

        #[property(get = Self::infobar_content, type = Option < gtk::Widget >)]
        pub infobar_content: OnceCell<gtk::Box>,

        pub utilization: OnceCell<gtk::Label>,
        pub memory_usage_current: OnceCell<gtk::Label>,
        pub memory_usage_max: OnceCell<gtk::Label>,
        pub clock_speed_current: OnceCell<gtk::Label>,
        pub clock_speed_max: OnceCell<gtk::Label>,
        pub memory_speed_current: OnceCell<gtk::Label>,
        pub memory_speed_max: OnceCell<gtk::Label>,
        pub power_draw_current: OnceCell<gtk::Label>,
        pub power_draw_max: OnceCell<gtk::Label>,
        pub temperature: OnceCell<gtk::Label>,
        pub opengl_version: OnceCell<gtk::Label>,
        pub vulkan_version: OnceCell<gtk::Label>,
        pub pcie_speed: OnceCell<gtk::Label>,
        pub pci_addr: OnceCell<gtk::Label>,

        pub box_temp: OnceCell<gtk::Box>,
        pub box_mem_speed: OnceCell<gtk::Box>,
        pub box_mem_usage: OnceCell<gtk::Box>,
        pub box_power_draw: OnceCell<gtk::Box>,
    }

    impl Default for PerformancePageGpu {
        fn default() -> Self {
            Self {
                gpu_id: Default::default(),
                device_name: Default::default(),
                overall_percent: Default::default(),
                usage_graph_overall: Default::default(),
                encode_percent: Default::default(),
                encode_label: Default::default(),
                usage_graph_encode: Default::default(),
                decode_graph: Default::default(),
                decode_percent: Default::default(),
                usage_graph_decode: Default::default(),
                memory_graph: Default::default(),
                total_memory: Default::default(),
                usage_graph_memory: Default::default(),
                context_menu: Default::default(),

                name: Cell::new(String::new()),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),

                infobar_content: Default::default(),

                utilization: Default::default(),
                memory_usage_current: Default::default(),
                memory_usage_max: Default::default(),
                clock_speed_current: Default::default(),
                clock_speed_max: Default::default(),
                memory_speed_current: Default::default(),
                memory_speed_max: Default::default(),
                power_draw_current: Default::default(),
                power_draw_max: Default::default(),
                temperature: Default::default(),
                opengl_version: Default::default(),
                vulkan_version: Default::default(),
                pcie_speed: Default::default(),
                pci_addr: Default::default(),

                box_temp: Default::default(),
                box_mem_speed: Default::default(),
                box_mem_usage: Default::default(),
                box_power_draw: Default::default(),
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

        fn infobar_content(&self) -> Option<gtk::Widget> {
            self.infobar_content.get().map(|ic| ic.clone().into())
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
            index: Option<usize>,
            gpu: &crate::sys_info_v2::GpuStaticInfo,
        ) -> bool {
            use crate::sys_info_v2::OpenGLApi;

            let t = this.clone();
            this.imp()
                .usage_graph_overall
                .connect_local("resize", true, move |_| {
                    let this = t.imp();

                    let width = this.usage_graph_overall.width() as f32;
                    let height = this.usage_graph_overall.height() as f32;

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

                    let width = this.usage_graph_encode.width() as f32;
                    let height = this.usage_graph_encode.height() as f32;

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

                    None
                });

            let this = this.imp();

            // Intel GPUs don't offer a great deal of information, and combine video encode and decode data
            // Hide the things that are missing and adjust the graphs
            if gpu.vendor_id == 0x8086 {
                this.box_power_draw
                    .get()
                    .and_then(|b| Some(b.set_visible(false)));
                this.box_mem_usage
                    .get()
                    .and_then(|b| Some(b.set_visible(false)));
                this.box_mem_speed
                    .get()
                    .and_then(|b| Some(b.set_visible(false)));
                this.box_temp.get().and_then(|b| Some(b.set_visible(false)));

                this.memory_graph.set_visible(false);
                this.decode_graph.set_visible(false);

                this.encode_label.set_text(&i18n("Video encode/decode"));
            }

            if index.is_some() {
                this.gpu_id.set_text(&format!("GPU {}", index.unwrap()));
            } else {
                this.gpu_id.set_text(&format!("GPU"));
            }

            this.device_name.set_text(&gpu.device_name);

            this.usage_graph_memory
                .set_value_range_max(gpu.total_memory as f32);

            let total_memory = crate::to_human_readable(gpu.total_memory as f32, 1024.);
            this.total_memory.set_text(&format!(
                "{0:.2$} {1}{3}B",
                total_memory.0,
                total_memory.1,
                total_memory.2,
                if total_memory.1.is_empty() { "" } else { "i" }
            ));

            if let Some(memory_usage_max) = this.memory_usage_max.get() {
                memory_usage_max.set_text(&this.total_memory.label());
            }

            let ogl_version = if let Some(opengl_version) = gpu.opengl_version.as_ref() {
                format!(
                    "{}{}.{}",
                    if opengl_version.api == OpenGLApi::OpenGLES {
                        "ES "
                    } else {
                        ""
                    },
                    opengl_version.major,
                    opengl_version.minor
                )
            } else {
                i18n("Unknown")
            };
            if let Some(opengl_version) = this.opengl_version.get() {
                opengl_version.set_text(&ogl_version);
            }

            let vk_version = if let Some(vulkan_version) = gpu.vulkan_version.as_ref() {
                format!(
                    "{}.{}.{}",
                    vulkan_version.major, vulkan_version.minor, vulkan_version.patch
                )
            } else {
                i18n("Unsupported")
            };
            if let Some(vulkan_version) = this.vulkan_version.get() {
                vulkan_version.set_text(&vk_version);
            }

            if let Some(pcie_speed) = this.pcie_speed.get() {
                pcie_speed.set_text(&format!("PCIe Gen {} x{} ", gpu.pcie_gen, gpu.pcie_lanes));
            }

            if let Some(pci_addr) = this.pci_addr.get() {
                pci_addr.set_text(gpu.id.as_ref());
            }

            true
        }

        pub(crate) fn update_readings(
            this: &super::PerformancePageGpu,
            gpu: &crate::sys_info_v2::GpuDynamicInfo,
        ) -> bool {
            let this = this.imp();

            this.overall_percent
                .set_text(&format!("{}%", gpu.util_percent));
            this.usage_graph_overall
                .add_data_point(0, gpu.util_percent as f32);
            if let Some(utilization) = this.utilization.get() {
                utilization.set_text(&format!("{}%", gpu.util_percent));
            }

            this.encode_percent
                .set_text(&format!("{}%", gpu.encoder_percent));
            this.usage_graph_encode
                .add_data_point(0, gpu.encoder_percent as f32);

            this.decode_percent
                .set_text(&format!("{}%", gpu.decoder_percent));
            this.usage_graph_decode
                .add_data_point(0, gpu.decoder_percent as f32);

            this.usage_graph_memory
                .add_data_point(0, gpu.used_memory as f32);

            let used_memory = crate::to_human_readable(gpu.used_memory as f32, 1024.);
            if let Some(memory_usage_current) = this.memory_usage_current.get() {
                memory_usage_current.set_text(&format!(
                    "{0:.2$} {1}{3}B",
                    used_memory.0,
                    used_memory.1,
                    used_memory.2,
                    if used_memory.1.is_empty() { "" } else { "i" },
                ));
            }

            let clock_speed =
                crate::to_human_readable(gpu.clock_speed_mhz as f32 * 1_000_000., 1000.);
            let cs_max =
                crate::to_human_readable(gpu.clock_speed_max_mhz as f32 * 1_000_000., 1000.);
            if let Some(clock_speed_current) = this.clock_speed_current.get() {
                clock_speed_current.set_text(&format!(
                    "{0:.2$} {1}Hz",
                    clock_speed.0, clock_speed.1, clock_speed.2
                ));
            }
            if let Some(clock_speed_max) = this.clock_speed_max.get() {
                clock_speed_max.set_text(&format!("{0:.2$} {1}Hz", cs_max.0, cs_max.1, cs_max.2));
            }

            let memory_speed =
                crate::to_human_readable(gpu.mem_speed_mhz as f32 * 1_000_000., 1000.);
            let ms_max = crate::to_human_readable(gpu.mem_speed_max_mhz as f32 * 1_000_000., 1000.);
            if let Some(memory_speed_current) = this.memory_speed_current.get() {
                memory_speed_current.set_text(&format!(
                    "{0:.2$} {1}Hz",
                    memory_speed.0, memory_speed.1, memory_speed.2
                ));
            }
            if let Some(memory_speed_max) = this.memory_speed_max.get() {
                memory_speed_max.set_text(&format!("{0:.2$} {1}Hz", ms_max.0, ms_max.1, ms_max.2));
            }

            let power_draw = crate::to_human_readable(gpu.power_draw_watts, 1000.);
            let power_limit = if gpu.power_draw_max_watts != 0.0 {
                Some(crate::to_human_readable(gpu.power_draw_max_watts, 1000.))
            } else {
                None
            };
            if let Some(power_draw_current) = this.power_draw_current.get() {
                power_draw_current.set_text(&format!(
                    "{0:.2$} {1}W",
                    power_draw.0, power_draw.1, power_draw.2
                ));
            }
            if let Some(power_draw_max) = this.power_draw_max.get() {
                if let Some(power_limit) = power_limit {
                    power_draw_max.set_text(&format!(
                        " / {0:.2$} {1}W",
                        power_limit.0, power_limit.1, power_limit.2
                    ));
                }
            }

            if let Some(temperature) = this.temperature.get() {
                temperature.set_text(&format!("{}°C", gpu.temp_celsius));
            }

            true
        }

        fn data_summary(&self) -> String {
            let unknown = i18n("Unknown");
            let unknown = unknown.as_str();

            format!(
                r#"{}

    {}

    OpenGL version:    {}
    Vulkan version:    {}
    PCI Express speed: {}
    PCI bus address:   {}

    Utilization:  {}
    Memory usage: {} / {}
    Clock speed:  {} / {}
    Memory speed: {} / {}
    Power draw:   {} / {}
    Temperature:  {}"#,
                self.gpu_id.label(),
                self.device_name.label(),
                self.opengl_version
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.vulkan_version
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.pcie_speed
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.pci_addr
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.overall_percent.label(),
                self.memory_usage_current
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.memory_usage_max
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.clock_speed_current
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.clock_speed_max
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.memory_speed_current
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.memory_speed_max
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.power_draw_current
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.power_draw_max
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.temperature
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
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

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageGpu>().clone();

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);

            let sidebar_content_builder = gtk::Builder::from_resource(
                "/io/missioncenter/MissionCenter/ui/performance_page/gpu_details.ui",
            );

            let _ = self.infobar_content.set(
                sidebar_content_builder
                    .object::<gtk::Box>("root")
                    .expect("Could not find `root` object in details pane"),
            );

            let _ = self.utilization.set(
                sidebar_content_builder
                    .object::<gtk::Label>("utilization")
                    .expect("Could not find `utilization` object in details pane"),
            );
            let _ = self.memory_usage_current.set(
                sidebar_content_builder
                    .object::<gtk::Label>("memory_usage_current")
                    .expect("Could not find `memory_usage_current` object in details pane"),
            );
            let _ = self.memory_usage_max.set(
                sidebar_content_builder
                    .object::<gtk::Label>("memory_usage_max")
                    .expect("Could not find `memory_usage_max` object in details pane"),
            );
            let _ = self.clock_speed_current.set(
                sidebar_content_builder
                    .object::<gtk::Label>("clock_speed_current")
                    .expect("Could not find `clock_speed_current` object in details pane"),
            );
            let _ = self.clock_speed_max.set(
                sidebar_content_builder
                    .object::<gtk::Label>("clock_speed_max")
                    .expect("Could not find `clock_speed_max` object in details pane"),
            );
            let _ = self.memory_speed_current.set(
                sidebar_content_builder
                    .object::<gtk::Label>("memory_speed_current")
                    .expect("Could not find `memory_speed_current` object in details pane"),
            );
            let _ = self.memory_speed_max.set(
                sidebar_content_builder
                    .object::<gtk::Label>("memory_speed_max")
                    .expect("Could not find `memory_speed_max` object in details pane"),
            );
            let _ = self.power_draw_current.set(
                sidebar_content_builder
                    .object::<gtk::Label>("power_draw_current")
                    .expect("Could not find `power_draw_current` object in details pane"),
            );
            let _ = self.power_draw_max.set(
                sidebar_content_builder
                    .object::<gtk::Label>("power_draw_max")
                    .expect("Could not find `power_draw_max` object in details pane"),
            );
            let _ = self.temperature.set(
                sidebar_content_builder
                    .object::<gtk::Label>("temperature")
                    .expect("Could not find `temperature` object in details pane"),
            );
            let _ = self.opengl_version.set(
                sidebar_content_builder
                    .object::<gtk::Label>("opengl_version")
                    .expect("Could not find `opengl_version` object in details pane"),
            );
            let _ = self.vulkan_version.set(
                sidebar_content_builder
                    .object::<gtk::Label>("vulkan_version")
                    .expect("Could not find `vulkan_version` object in details pane"),
            );
            let _ = self.pcie_speed.set(
                sidebar_content_builder
                    .object::<gtk::Label>("pcie_speed")
                    .expect("Could not find `pcie_speed` object in details pane"),
            );
            let _ = self.pci_addr.set(
                sidebar_content_builder
                    .object::<gtk::Label>("pci_addr")
                    .expect("Could not find `pci_addr` object in details pane"),
            );

            let _ = self.box_temp.set(
                sidebar_content_builder
                    .object::<gtk::Box>("box_temp")
                    .expect("Could not find `box_temp` object in details pane"),
            );
            let _ = self.box_mem_speed.set(
                sidebar_content_builder
                    .object::<gtk::Box>("box_mem_speed")
                    .expect("Could not find `box_mem_speed` object in details pane"),
            );
            let _ = self.box_mem_usage.set(
                sidebar_content_builder
                    .object::<gtk::Box>("box_mem_usage")
                    .expect("Could not find `box_mem_usage` object in details pane"),
            );
            let _ = self.box_power_draw.set(
                sidebar_content_builder
                    .object::<gtk::Box>("box_power_draw")
                    .expect("Could not find `box_power_draw` object in details pane"),
            );
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

    pub fn set_static_information(
        &self,
        index: Option<usize>,
        gpu: &crate::sys_info_v2::GpuStaticInfo,
    ) -> bool {
        imp::PerformancePageGpu::set_static_information(self, index, gpu)
    }

    pub fn update_readings(&self, gpu: &crate::sys_info_v2::GpuDynamicInfo) -> bool {
        imp::PerformancePageGpu::update_readings(self, gpu)
    }

    pub fn infobar_collapsed(&self) {
        self.imp()
            .infobar_content
            .get()
            .and_then(|ic| Some(ic.set_margin_top(10)));
    }

    pub fn infobar_uncollapsed(&self) {
        self.imp()
            .infobar_content
            .get()
            .and_then(|ic| Some(ic.set_margin_top(65)));
    }
}
