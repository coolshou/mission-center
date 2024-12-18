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

use adw::{self, subclass::prelude::*};
use glib::{g_critical, g_warning, ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*};

use super::{widgets::GraphWidget, GpuDetails, PageExt};
use crate::{
    application::INTERVAL_STEP,
    i18n::*,
    settings,
    sys_info_v2::{GpuDynamicInfo, GpuStaticInfo, OpenGLApi},
};

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
        pub graph_utilization: TemplateChild<GraphWidget>,
        #[template_child]
        pub container_bottom: TemplateChild<gtk::Box>,
        #[template_child]
        pub encode_decode_graph: TemplateChild<gtk::Box>,
        #[template_child]
        pub usage_graph_encode_decode: TemplateChild<GraphWidget>,
        #[template_child]
        pub memory_graph: TemplateChild<gtk::Box>,
        #[template_child]
        pub total_memory: TemplateChild<gtk::Label>,
        #[template_child]
        pub memory_graph_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph_memory: TemplateChild<GraphWidget>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,
        #[template_child]
        pub graph_max_duration: TemplateChild<gtk::Label>,

        #[property(get = Self::name, set = Self::set_name, type = String)]
        name: Cell<String>,
        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,

        #[property(get, set)]
        encode_decode_available: Cell<bool>,

        #[property(get = Self::infobar_content, type = Option < gtk::Widget >)]
        pub infobar_content: GpuDetails,

        show_enc_dec_action: gio::SimpleAction,
    }

    impl Default for PerformancePageGpu {
        fn default() -> Self {
            Self {
                gpu_id: Default::default(),
                device_name: Default::default(),
                graph_utilization: Default::default(),
                container_bottom: Default::default(),
                encode_decode_graph: Default::default(),
                usage_graph_encode_decode: Default::default(),
                memory_graph: Default::default(),
                total_memory: Default::default(),
                memory_graph_label: Default::default(),
                usage_graph_memory: Default::default(),
                context_menu: Default::default(),
                graph_max_duration: Default::default(),

                name: Cell::new(String::new()),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),

                encode_decode_available: Cell::new(true),

                infobar_content: GpuDetails::new(),

                show_enc_dec_action: gio::SimpleAction::new_stateful(
                    "enc_dec_usage",
                    None,
                    &glib::Variant::from(true),
                ),
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
            Some(self.infobar_content.clone().upcast())
        }
    }

    impl PerformancePageGpu {
        fn configure_actions(this: &super::PerformancePageGpu) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let action = gio::SimpleAction::new("copy", None);
            action.connect_activate({
                let this = this.downgrade();
                move |_, _| {
                    if let Some(this) = this.upgrade() {
                        let clipboard = this.clipboard();
                        clipboard.set_text(this.imp().data_summary().as_str());
                    }
                }
            });
            actions.add_action(&action);

            let show_enc_dec_usage =
                settings!().boolean("performance-page-gpu-encode-decode-usage-visible");

            let action = &this.imp().show_enc_dec_action;
            action.set_enabled(true);
            action.set_state(&glib::Variant::from(show_enc_dec_usage));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    if let Some(this) = this.upgrade() {
                        let this = &this.imp();

                        let visible = !action
                            .state()
                            .and_then(|v| v.get::<bool>())
                            .unwrap_or(false);

                        this.obj().set_encode_decode_available(visible);
                        action.set_state(&glib::Variant::from(visible));

                        // The usage graph is `homogeneous: true`, so we need to hide the container if all
                        // contained graphs are hidden so that the usage graph expands to fill the available
                        // space.
                        this.container_bottom.set_visible(
                            this.memory_graph.is_visible() || this.encode_decode_available.get(),
                        );

                        settings!()
                            .set_boolean(
                                "performance-page-gpu-encode-decode-usage-visible",
                                visible,
                            )
                            .unwrap_or_else(|_| {
                                g_critical!(
                                    "MissionCenter::PerformancePage",
                                    "Failed to save show encode/decode usage"
                                );
                            });
                    }
                }
            });
            actions.add_action(action);
        }

        fn configure_context_menu(this: &super::PerformancePageGpu) {
            let right_click_controller = gtk::GestureClick::new();
            right_click_controller.set_button(3); // Secondary click (AKA right click)
            right_click_controller.connect_released({
                let this = this.downgrade();
                move |_click, _n_press, x, y| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    this.context_menu
                        .set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                            x.round() as i32,
                            y.round() as i32,
                            1,
                            1,
                        )));
                    this.context_menu.popup();
                }
            });
            this.add_controller(right_click_controller);
        }
    }

    impl PerformancePageGpu {
        pub fn set_static_information(
            this: &super::PerformancePageGpu,
            index: Option<usize>,
            static_info: &GpuStaticInfo,
        ) -> bool {
            let this = this.imp();

            this.graph_utilization.connect_local("resize", true, {
                let this = this.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return None,
                    };
                    let this = this.imp();

                    let width = this.graph_utilization.width() as f32;
                    let height = this.graph_utilization.height() as f32;

                    let mut a = width;
                    let mut b = height;
                    if width > height {
                        a = height;
                        b = width;
                    }

                    this.graph_utilization
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                    this.usage_graph_encode_decode
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                    this.usage_graph_memory
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                    None
                }
            });

            if index.is_some() {
                this.gpu_id.set_text(&format!("GPU {}", index.unwrap()));
            } else {
                this.gpu_id.set_text("GPU");
            }

            this.device_name.set_text(&static_info.device_name);

            let settings = settings!();
            let show_enc_dec_usage =
                settings.boolean("performance-page-gpu-encode-decode-usage-visible");
            settings.connect_changed(Some("performance-page-gpu-encode-decode-usage-visible"), {
                let this = this.obj().downgrade();
                move |settings, _| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();

                        let show_enc_dec_usage =
                            settings.boolean("performance-page-gpu-encode-decode-usage-visible");

                        this.obj().set_encode_decode_available(show_enc_dec_usage);
                    }
                }
            });

            if !show_enc_dec_usage {
                this.obj().set_encode_decode_available(false);
            }

            this.infobar_content
                .set_encode_decode_shared(static_info.encode_decode_shared);
            if static_info.encode_decode_shared {
                this.infobar_content
                    .encode_label()
                    .set_label(&i18n("Video encode/decode"));
            } else {
                this.usage_graph_encode_decode.set_dashed(0, true);
                this.usage_graph_encode_decode.set_filled(0, false);
            }

            let ogl_version = if let Some(opengl_version) = static_info.opengl_version.as_ref() {
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
            this.infobar_content.opengl_version().set_text(&ogl_version);

            let vk_version = if let Some(vulkan_version) = static_info.vulkan_version.as_ref() {
                format!(
                    "{}.{}.{}",
                    vulkan_version.major, vulkan_version.minor, vulkan_version.patch
                )
            } else {
                i18n("Unsupported")
            };
            this.infobar_content.vulkan_version().set_text(&vk_version);

            this.infobar_content.set_pcie_info_visible(
                static_info.pcie_gen.is_some() && static_info.pcie_lanes.is_some(),
            );
            if this.infobar_content.pcie_info_visible() {
                this.infobar_content.pcie_speed().set_text(&format!(
                    "PCIe Gen {} x{} ",
                    static_info.pcie_gen.map(|v| v.get()).unwrap_or_default(),
                    static_info.pcie_lanes.map(|v| v.get()).unwrap_or_default()
                ));
            }

            this.infobar_content
                .pci_addr()
                .set_text(static_info.id.as_ref());

            true
        }

        pub(crate) fn update_readings(
            this: &super::PerformancePageGpu,
            static_info: &GpuStaticInfo,
            dynamic_info: &GpuDynamicInfo,
        ) -> bool {
            let this = this.imp();

            this.update_utilization(static_info, dynamic_info);
            this.update_clock_speed(dynamic_info);
            this.update_power_draw(dynamic_info);
            this.update_memory_info(static_info, dynamic_info);
            this.update_memory_speed(dynamic_info);
            this.update_video_encode_decode(static_info, dynamic_info);
            this.update_temperature(dynamic_info);

            // The usage graph is `homogeneous: true`, so we need to hide the container if all
            // contained graphs are hidden so that the usage graph expands to fill the available
            // space.
            this.container_bottom
                .set_visible(this.memory_graph.is_visible() || this.encode_decode_available.get());

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

    Utilization:   {}
    Memory usage:  {} / {}
    GTT usage:     {} / {}
    Clock speed:   {} / {}
    Memory speed:  {} / {}
    Power draw:    {}{}
    Encode/Decode: {} / {}
    Temperature:   {}"#,
                self.gpu_id.label(),
                self.device_name.label(),
                self.infobar_content.opengl_version().label(),
                self.infobar_content.vulkan_version().label(),
                self.infobar_content.pcie_speed().label(),
                self.infobar_content.utilization().label(),
                self.infobar_content.pci_addr().label(),
                self.infobar_content.memory_usage_current().label(),
                self.infobar_content.memory_usage_max().label(),
                self.infobar_content.shared_mem_usage_current().label(),
                self.infobar_content.shared_mem_usage_max().label(),
                self.infobar_content.clock_speed_current().label(),
                self.infobar_content.clock_speed_max().label(),
                self.infobar_content.memory_speed_current().label(),
                self.infobar_content.memory_speed_max().label(),
                self.infobar_content.power_draw_current().label(),
                self.infobar_content.power_draw_max().label(),
                self.infobar_content.encode_percent().label(),
                self.infobar_content.decode_percent().label(),
                self.infobar_content.temperature().label(),
            )
        }

        fn update_utilization(&self, static_info: &GpuStaticInfo, dynamic_info: &GpuDynamicInfo) {
            let overall_usage = dynamic_info.util_percent.unwrap_or_else(|| {
                g_warning!(
                    "MissionCenter::PerformancePage",
                    "GPU '{}' utilization data is missing",
                    static_info.id
                );
                0
            });

            self.graph_utilization
                .add_data_point(0, overall_usage as f32);
            self.infobar_content
                .utilization()
                .set_text(&format!("{}%", overall_usage));
        }

        fn update_clock_speed(&self, dynamic_info: &GpuDynamicInfo) {
            let mut clock_speed_available = false;

            if let Some(max_clock_speed) = dynamic_info.clock_speed_max_mhz {
                self.infobar_content
                    .clock_speed_separator()
                    .set_visible(true);
                self.infobar_content.clock_speed_max().set_visible(true);

                let cs_max =
                    crate::to_human_readable(max_clock_speed.get() as f32 * 1_000_000., 1000.);

                self.infobar_content
                    .clock_speed_max()
                    .set_text(&format!("{0:.2$} {1}Hz", cs_max.0, cs_max.1, cs_max.2));
            } else {
                self.infobar_content
                    .clock_speed_separator()
                    .set_visible(false);
                self.infobar_content.clock_speed_max().set_visible(false);
            }

            if let Some(clock_speed) = dynamic_info.clock_speed_mhz {
                clock_speed_available = true;

                let clock_speed = crate::to_human_readable(clock_speed as f32 * 1_000_000., 1000.);

                self.infobar_content
                    .clock_speed_current()
                    .set_text(&format!(
                        "{0:.2$} {1}Hz",
                        clock_speed.0, clock_speed.1, clock_speed.2
                    ));
            }

            self.infobar_content
                .set_clock_speed_available(clock_speed_available);
        }

        fn update_power_draw(&self, dynamic_info: &GpuDynamicInfo) {
            let mut power_draw_available = false;

            if let Some(power_limit) = dynamic_info.power_draw_max_watts {
                self.infobar_content
                    .power_draw_separator()
                    .set_visible(true);
                self.infobar_content.power_draw_max().set_visible(true);

                let power_limit = crate::to_human_readable(power_limit, 1000.);
                self.infobar_content.power_draw_max().set_text(&format!(
                    "{0:.2$} {1}W",
                    power_limit.0, power_limit.1, power_limit.2
                ));
            } else {
                self.infobar_content
                    .power_draw_separator()
                    .set_visible(false);
                self.infobar_content.power_draw_max().set_visible(false);
            }

            if let Some(power_draw) = dynamic_info.power_draw_watts {
                power_draw_available = true;

                let power_draw = crate::to_human_readable(power_draw, 1000.);
                self.infobar_content.power_draw_current().set_text(&format!(
                    "{0:.2$} {1}W",
                    power_draw.0, power_draw.1, power_draw.2
                ));
            }

            self.infobar_content
                .set_power_draw_available(power_draw_available);
        }

        fn update_memory_info(&self, static_info: &GpuStaticInfo, dynamic_info: &GpuDynamicInfo) {
            fn update_dedicated_memory(
                this: &PerformancePageGpu,
                static_info: &GpuStaticInfo,
                dynamic_info: &GpuDynamicInfo,
                has_memory_info: &mut bool,
            ) -> Option<String> {
                let mut total_memory_str_res = None;

                if let Some(total_memory) = static_info.total_memory {
                    let total_memory = total_memory.get() as f32;
                    let total_memory_hr = crate::to_human_readable(total_memory, 1024.);
                    let total_memory_str = format!(
                        "{0:.2$} {1}{3}B",
                        total_memory_hr.0,
                        total_memory_hr.1,
                        total_memory_hr.2,
                        if total_memory_hr.1.is_empty() {
                            ""
                        } else {
                            "i"
                        },
                    );

                    this.usage_graph_memory
                        .set_scaling(GraphWidget::no_scaling());
                    this.usage_graph_memory.set_value_range_max(total_memory);
                    this.infobar_content.set_total_memory_valid(true);

                    this.infobar_content
                        .memory_usage_max()
                        .set_text(&total_memory_str);

                    total_memory_str_res = Some(total_memory_str);
                } else {
                    this.infobar_content.set_total_memory_valid(false);
                }

                if let Some(used_memory) = dynamic_info.used_memory {
                    *has_memory_info = true;

                    this.infobar_content.set_used_memory_valid(true);
                    this.infobar_content
                        .memory_usage_title()
                        .set_text(&i18n("Memory Usage"));

                    this.usage_graph_memory
                        .add_data_point(0, used_memory as f32);

                    let used_memory = crate::to_human_readable(
                        dynamic_info.used_memory.unwrap_or(0) as f32,
                        1024.,
                    );
                    this.infobar_content
                        .memory_usage_current()
                        .set_text(&format!(
                            "{0:.2$} {1}{3}B",
                            used_memory.0,
                            used_memory.1,
                            used_memory.2,
                            if used_memory.1.is_empty() { "" } else { "i" },
                        ));
                } else {
                    this.infobar_content.set_used_memory_valid(false);

                    if this.infobar_content.total_memory_valid() {
                        this.infobar_content
                            .memory_usage_title()
                            .set_text(&i18n("Total Memory"));
                    }
                }

                total_memory_str_res
            }

            fn update_shared_memory(
                this: &PerformancePageGpu,
                static_info: &GpuStaticInfo,
                dynamic_info: &GpuDynamicInfo,
                total_memory_str: Option<&str>,
                has_memory_info: &mut bool,
            ) {
                if let Some(total_shared_memory) = static_info.total_shared_memory {
                    let total_gtt =
                        crate::to_human_readable(total_shared_memory.get() as f32, 1024.);
                    let total_gtt = format!(
                        "{0:.2$} {1}{3}B",
                        total_gtt.0,
                        total_gtt.1,
                        total_gtt.2,
                        if total_gtt.1.is_empty() { "" } else { "i" },
                    );

                    this.usage_graph_memory.set_dashed(1, true);
                    this.usage_graph_memory.set_filled(1, false);
                    this.infobar_content.set_total_shared_memory_valid(true);

                    if let Some(total_memory_str) = total_memory_str {
                        this.total_memory
                            .set_text(&format!("{total_memory_str} / {total_gtt}"));

                        this.memory_graph_label
                            .set_text(&i18n("Dedicated and shared memory usage over "));

                        this.usage_graph_memory
                            .set_scaling(GraphWidget::no_scaling());
                        let current_max = this.usage_graph_memory.value_range_max();
                        this.usage_graph_memory
                            .set_value_range_max(current_max.max(total_shared_memory.get() as f32));
                    } else {
                        this.total_memory.set_text(&total_gtt);

                        this.usage_graph_memory
                            .set_scaling(GraphWidget::no_scaling());
                        this.usage_graph_memory
                            .set_value_range_max(total_shared_memory.get() as f32);
                    }
                    this.infobar_content
                        .shared_mem_usage_max()
                        .set_text(&total_gtt);
                } else {
                    this.infobar_content.set_total_shared_memory_valid(false);
                }

                if let Some(used_shared_memory) = dynamic_info.used_shared_memory {
                    *has_memory_info = true;

                    this.infobar_content.set_used_shared_memory_valid(true);
                    this.infobar_content
                        .shared_memory_usage_title()
                        .set_text(&i18n("Shared Memory Usage"));

                    let used_shared_mem_str =
                        crate::to_human_readable(used_shared_memory as f32, 1024.);

                    this.infobar_content
                        .shared_mem_usage_current()
                        .set_text(&format!(
                            "{0:.2$} {1}{3}B",
                            used_shared_mem_str.0,
                            used_shared_mem_str.1,
                            used_shared_mem_str.2,
                            if used_shared_mem_str.1.is_empty() {
                                ""
                            } else {
                                "i"
                            },
                        ));
                } else {
                    this.infobar_content.set_used_shared_memory_valid(false);

                    if this.infobar_content.total_shared_memory_valid() {
                        this.infobar_content
                            .shared_memory_usage_title()
                            .set_text(&i18n("Total Shared Memory"));
                    }
                }
            }

            let mut has_memory_info = false;

            let total_memory_str =
                update_dedicated_memory(self, static_info, dynamic_info, &mut has_memory_info);

            update_shared_memory(
                self,
                static_info,
                dynamic_info,
                total_memory_str.as_ref().map(String::as_str),
                &mut has_memory_info,
            );

            if !self.infobar_content.total_memory_valid()
                && !self.infobar_content.total_shared_memory_valid()
            {
                self.usage_graph_memory
                    .set_scaling(GraphWidget::normalized_scaling());
            }

            self.memory_graph.set_visible(has_memory_info);
        }

        fn update_memory_speed(&self, dynamic_info: &GpuDynamicInfo) {
            let mut memory_speed_available = false;

            if let Some(max_memory_speed) = dynamic_info.mem_speed_max_mhz {
                self.infobar_content
                    .memory_speed_separator()
                    .set_visible(true);
                self.infobar_content.memory_speed_max().set_visible(true);

                let ms_max =
                    crate::to_human_readable(max_memory_speed.get() as f32 * 1_000_000., 1000.);
                self.infobar_content
                    .memory_speed_max()
                    .set_text(&format!("{0:.2$} {1}Hz", ms_max.0, ms_max.1, ms_max.2));
            } else {
                self.infobar_content
                    .memory_speed_separator()
                    .set_visible(false);
                self.infobar_content.memory_speed_max().set_visible(false);
            }

            if let Some(memory_speed) = dynamic_info.mem_speed_mhz {
                memory_speed_available = true;

                let memory_speed =
                    crate::to_human_readable(memory_speed as f32 * 1_000_000., 1000.);
                self.infobar_content
                    .memory_speed_current()
                    .set_text(&format!(
                        "{0:.2$} {1}Hz",
                        memory_speed.0, memory_speed.1, memory_speed.2
                    ));
            }

            self.infobar_content
                .set_memory_speed_available(memory_speed_available);
        }

        fn update_video_encode_decode(
            &self,
            static_info: &GpuStaticInfo,
            dynamic_info: &GpuDynamicInfo,
        ) {
            let mut encode_decode_info_available = false;

            if let Some(encoder_percent) = dynamic_info.encoder_percent {
                encode_decode_info_available = true;

                self.usage_graph_encode_decode
                    .add_data_point(0, encoder_percent as f32);

                self.infobar_content
                    .encode_percent()
                    .set_text(&format!("{}%", dynamic_info.encoder_percent.unwrap_or(0)));
            }

            if !static_info.encode_decode_shared {
                if let Some(decoder_percent) = dynamic_info.decoder_percent {
                    encode_decode_info_available = true;

                    self.usage_graph_encode_decode
                        .add_data_point(1, decoder_percent as f32);

                    self.infobar_content
                        .decode_percent()
                        .set_text(&format!("{}%", dynamic_info.decoder_percent.unwrap_or(0)));
                }
            }

            self.show_enc_dec_action
                .set_enabled(encode_decode_info_available);
            self.obj().set_encode_decode_available(
                encode_decode_info_available
                    && self
                        .show_enc_dec_action
                        .state()
                        .and_then(|v| v.get::<bool>())
                        .unwrap_or(false),
            );
        }

        fn update_temperature(&self, dynamic_info: &GpuDynamicInfo) {
            if let Some(temp) = dynamic_info.temp_celsius {
                self.infobar_content.box_temp().set_visible(true);

                self.infobar_content
                    .temperature()
                    .set_text(&format!("{}°C", temp));
            } else {
                self.infobar_content.box_temp().set_visible(false);
            }
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

            let this = self.obj();

            this.as_ref()
                .bind_property(
                    "encode-decode-available",
                    &self.infobar_content,
                    "encode-decode-available",
                )
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);
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

impl PageExt for PerformancePageGpu {
    fn infobar_collapsed(&self) {
        self.imp().infobar_content.set_margin_top(10);
    }

    fn infobar_uncollapsed(&self) {
        self.imp().infobar_content.set_margin_top(65);
    }
}

impl PerformancePageGpu {
    pub fn new(name: &str, settings: &gio::Settings) -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(Self::static_type(), &mut [("name", name.into())])
                .downcast()
                .unwrap()
        };

        fn update_refresh_rate_sensitive_labels(
            this: &PerformancePageGpu,
            settings: &gio::Settings,
        ) {
            let this = this.imp();

            let data_points = settings.int("performance-page-data-points") as u32;
            let smooth = settings.boolean("performance-smooth-graphs");

            let graph_max_duration = (((settings.uint64("app-update-interval-u64") as f64)
                * INTERVAL_STEP)
                * (data_points as f64))
                .round() as u32;

            let mins = graph_max_duration / 60;
            let seconds_to_string = &i18n_f(
                "{} second{}",
                &[
                    &format!("{}", graph_max_duration % 60),
                    if (graph_max_duration % 60) != 1 {
                        "s"
                    } else {
                        ""
                    },
                ],
            );
            let mins_to_string = &i18n_f(
                "{} minute{} ",
                &[&format!("{:}", mins), if mins > 1 { "s" } else { "" }],
            );
            this.graph_max_duration.set_text(&*format!(
                "{}{}",
                if mins > 0 {
                    mins_to_string.clone()
                } else {
                    "".to_string()
                },
                if graph_max_duration % 60 > 0 {
                    seconds_to_string.clone()
                } else {
                    "".to_string()
                }
            ));

            this.graph_utilization.set_data_points(data_points);
            this.graph_utilization.set_smooth_graphs(smooth);
            this.usage_graph_encode_decode.set_data_points(data_points);
            this.usage_graph_encode_decode.set_smooth_graphs(smooth);
            this.usage_graph_memory.set_data_points(data_points);
            this.usage_graph_memory.set_smooth_graphs(smooth);
        }
        update_refresh_rate_sensitive_labels(&this, settings);

        settings.connect_changed(Some("performance-page-data-points"), {
            let this = this.downgrade();
            move |settings, _| {
                if let Some(this) = this.upgrade() {
                    update_refresh_rate_sensitive_labels(&this, settings);
                }
            }
        });

        settings.connect_changed(Some("app-update-interval"), {
            let this = this.downgrade();
            move |settings, _| {
                if let Some(this) = this.upgrade() {
                    update_refresh_rate_sensitive_labels(&this, settings);
                }
            }
        });

        settings.connect_changed(Some("performance-smooth-graphs"), {
            let this = this.downgrade();
            move |settings, _| {
                if let Some(this) = this.upgrade() {
                    update_refresh_rate_sensitive_labels(&this, settings);
                }
            }
        });

        this
    }

    pub fn set_static_information(
        &self,
        index: Option<usize>,
        static_info: &GpuStaticInfo,
    ) -> bool {
        imp::PerformancePageGpu::set_static_information(self, index, static_info)
    }

    pub fn update_readings(
        &self,
        static_info: &GpuStaticInfo,
        dynamic_info: &GpuDynamicInfo,
    ) -> bool {
        imp::PerformancePageGpu::update_readings(self, static_info, dynamic_info)
    }
}
