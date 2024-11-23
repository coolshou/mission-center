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
use glib::{ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*};

use super::{widgets::GraphWidget, GpuDetails, PageExt};
use crate::{application::INTERVAL_STEP, i18n::*, settings};

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
        pub usage_graph_overall: TemplateChild<GraphWidget>,
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

        #[property(get = Self::infobar_content, type = Option < gtk::Widget >)]
        pub infobar_content: GpuDetails,
    }

    impl Default for PerformancePageGpu {
        fn default() -> Self {
            Self {
                gpu_id: Default::default(),
                device_name: Default::default(),
                usage_graph_overall: Default::default(),
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

                infobar_content: GpuDetails::new(),
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
            use gtk::glib::*;
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let show_enc_dec_usage =
                settings!().boolean("performance-page-gpu-encode-decode-usage-visible");

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

            let action = gio::SimpleAction::new_stateful(
                "enc_dec_usage",
                None,
                &glib::Variant::from(show_enc_dec_usage),
            );
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    if let Some(this) = this.upgrade() {
                        let this = &this.imp();

                        let visible = !action
                            .state()
                            .and_then(|v| v.get::<bool>())
                            .unwrap_or(false);

                        this.encode_decode_graph.set_visible(visible);
                        this.infobar_content.legend_encode().set_visible(visible);
                        this.infobar_content.legend_decode().set_visible(visible);
                        action.set_state(&glib::Variant::from(visible));

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
            actions.add_action(&action);
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

                    this.usage_graph_encode_decode
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                    this.usage_graph_memory
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                    None
                });

            let this = this.imp();

            let show_enc_dec_usage =
                settings!().boolean("performance-page-gpu-encode-decode-usage-visible");

            this.encode_decode_graph.set_visible(show_enc_dec_usage);
            this.infobar_content
                .legend_encode()
                .set_visible(show_enc_dec_usage);
            this.infobar_content
                .legend_decode()
                .set_visible(show_enc_dec_usage);

            if let Some(total_memory) = gpu.total_memory {
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

                this.usage_graph_memory.set_value_range_max(total_memory);

                this.infobar_content
                    .memory_usage_max()
                    .set_text(&total_memory_str);

                if let Some(total_shared_memory) = gpu.total_shared_memory {
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

                    this.infobar_content
                        .legend_gtt()
                        .set_resource(Some("/io/missioncenter/MissionCenter/line-dashed-gpu.svg"));
                    this.infobar_content
                        .legend_vram()
                        .set_resource(Some("/io/missioncenter/MissionCenter/line-solid-gpu.svg"));

                    this.total_memory
                        .set_text(&format!("{total_memory_str} / {total_gtt}"));
                    this.infobar_content.gtt_usage_max().set_text(&total_gtt);

                    this.infobar_content
                        .memory_usage_max()
                        .set_text(&total_memory_str);
                    this.memory_graph_label
                        .set_text(&i18n("Dedicated and shared memory usage over "));
                } else {
                    this.infobar_content.legend_vram().set_visible(false);
                    this.infobar_content.box_gtt_usage().set_visible(false);

                    this.total_memory.set_text(&total_memory_str);
                }
            } else {
                this.infobar_content.box_mem_usage().set_visible(false);
                this.infobar_content.box_mem_speed().set_visible(false);
                this.infobar_content.legend_vram().set_visible(false);
                this.infobar_content.box_gtt_usage().set_visible(false);

                this.memory_graph.set_visible(false);
            }

            if gpu.encode_decode_shared {
                this.infobar_content.box_decode().set_visible(false);
                this.infobar_content.legend_encode().set_visible(false);
                this.infobar_content
                    .encode_label()
                    .set_label(&i18n("Video encode/decode"));
            } else {
                this.usage_graph_encode_decode.set_dashed(0, true);
                this.usage_graph_encode_decode.set_filled(0, false);
            }

            if gpu.pcie_gen.is_none() || gpu.pcie_lanes.is_none() {
                this.infobar_content.pcie_speed_label().set_visible(false);
                this.infobar_content.pcie_speed().set_visible(false);
            }

            // Intel GPUs don't offer a great deal of information, and combine video encode and decode data
            // Hide the things that are missing and adjust the graphs
            if gpu.vendor_id == 0x8086 {
                this.infobar_content.box_power_draw().set_visible(false);
                this.infobar_content.box_temp().set_visible(false);
            }

            if index.is_some() {
                this.gpu_id.set_text(&format!("GPU {}", index.unwrap()));
            } else {
                this.gpu_id.set_text("GPU");
            }

            this.device_name.set_text(&gpu.device_name);

            this.infobar_content
                .legend_encode()
                .set_resource(Some("/io/missioncenter/MissionCenter/line-dashed-gpu.svg"));
            this.infobar_content
                .legend_decode()
                .set_resource(Some("/io/missioncenter/MissionCenter/line-solid-gpu.svg"));

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
            this.infobar_content.opengl_version().set_text(&ogl_version);

            let vk_version = if let Some(vulkan_version) = gpu.vulkan_version.as_ref() {
                format!(
                    "{}.{}.{}",
                    vulkan_version.major, vulkan_version.minor, vulkan_version.patch
                )
            } else {
                i18n("Unsupported")
            };
            this.infobar_content.vulkan_version().set_text(&vk_version);

            this.infobar_content.pcie_speed().set_text(&format!(
                "PCIe Gen {} x{} ",
                gpu.pcie_gen.map(|v| v.get()).unwrap_or_default(),
                gpu.pcie_lanes.map(|v| v.get()).unwrap_or_default()
            ));

            this.infobar_content.pci_addr().set_text(gpu.id.as_ref());

            true
        }

        pub(crate) fn update_readings(
            this: &super::PerformancePageGpu,
            gpu: &crate::sys_info_v2::GpuDynamicInfo,
            gpu_static: &crate::sys_info_v2::GpuStaticInfo,
        ) -> bool {
            let this = this.imp();

            this.usage_graph_overall
                .add_data_point(0, gpu.util_percent as f32);
            this.infobar_content
                .utilization()
                .set_text(&format!("{}%", gpu.util_percent));

            this.usage_graph_encode_decode
                .add_data_point(0, gpu.encoder_percent as f32);
            this.usage_graph_encode_decode
                .add_data_point(1, gpu.decoder_percent as f32);

            if let Some(total_memory) = gpu_static.total_memory {
                this.usage_graph_memory
                    .add_data_point(0, gpu.used_memory as f32);

                if let Some(total_shared_memory) = gpu_static.total_shared_memory {
                    let mut gtt_factor =
                        total_memory.get() as f32 / total_shared_memory.get() as f32;
                    if gtt_factor.is_infinite() || gtt_factor.is_nan() || gtt_factor.is_subnormal()
                    {
                        gtt_factor = 0.;
                    }
                    this.usage_graph_memory
                        .add_data_point(1, gpu.used_gtt as f32 * gtt_factor);
                }
            }

            let used_memory = crate::to_human_readable(gpu.used_memory as f32, 1024.);
            this.infobar_content
                .memory_usage_current()
                .set_text(&format!(
                    "{0:.2$} {1}{3}B",
                    used_memory.0,
                    used_memory.1,
                    used_memory.2,
                    if used_memory.1.is_empty() { "" } else { "i" },
                ));

            let used_gtt = crate::to_human_readable(gpu.used_gtt as f32, 1024.);
            this.infobar_content.gtt_usage_current().set_text(&format!(
                "{0:.2$} {1}{3}B",
                used_gtt.0,
                used_gtt.1,
                used_gtt.2,
                if used_gtt.1.is_empty() { "" } else { "i" },
            ));

            let clock_speed =
                crate::to_human_readable(gpu.clock_speed_mhz as f32 * 1_000_000., 1000.);
            let cs_max =
                crate::to_human_readable(gpu.clock_speed_max_mhz as f32 * 1_000_000., 1000.);
            this.infobar_content
                .clock_speed_current()
                .set_text(&format!(
                    "{0:.2$} {1}Hz",
                    clock_speed.0, clock_speed.1, clock_speed.2
                ));
            this.infobar_content
                .clock_speed_max()
                .set_text(&format!("{0:.2$} {1}Hz", cs_max.0, cs_max.1, cs_max.2));

            let memory_speed =
                crate::to_human_readable(gpu.mem_speed_mhz as f32 * 1_000_000., 1000.);
            let ms_max = crate::to_human_readable(gpu.mem_speed_max_mhz as f32 * 1_000_000., 1000.);
            this.infobar_content
                .memory_speed_current()
                .set_text(&format!(
                    "{0:.2$} {1}Hz",
                    memory_speed.0, memory_speed.1, memory_speed.2
                ));
            this.infobar_content
                .memory_speed_max()
                .set_text(&format!("{0:.2$} {1}Hz", ms_max.0, ms_max.1, ms_max.2));

            let power_draw = crate::to_human_readable(gpu.power_draw_watts, 1000.);
            let power_limit = if gpu.power_draw_max_watts != 0.0 {
                Some(crate::to_human_readable(gpu.power_draw_max_watts, 1000.))
            } else {
                None
            };
            this.infobar_content.power_draw_current().set_text(&format!(
                "{0:.2$} {1}W",
                power_draw.0, power_draw.1, power_draw.2
            ));
            if let Some(power_limit) = power_limit {
                this.infobar_content.power_draw_max().set_text(&format!(
                    " / {0:.2$} {1}W",
                    power_limit.0, power_limit.1, power_limit.2
                ));
            }
            this.infobar_content
                .encode_percent()
                .set_text(&format!("{}%", gpu.encoder_percent));
            this.infobar_content
                .decode_percent()
                .set_text(&format!("{}%", gpu.decoder_percent));
            this.infobar_content
                .temperature()
                .set_text(&format!("{}°C", gpu.temp_celsius));

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
                self.infobar_content.gtt_usage_current().label(),
                self.infobar_content.gtt_usage_max().label(),
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

            this.usage_graph_overall.set_data_points(data_points);
            this.usage_graph_overall.set_smooth_graphs(smooth);
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

        fn set_hidden_vram(this: &PerformancePageGpu, settings: &gio::Settings) {
            let visible = settings.boolean("performance-page-gpu-encode-decode-usage-visible");

            let this = this.imp();

            this.encode_decode_graph.set_visible(visible);
        }

        settings.connect_changed(Some("performance-page-gpu-encode-decode-usage-visible"), {
            let this = this.downgrade();
            move |settings, _| {
                if let Some(this) = this.upgrade() {
                    set_hidden_vram(&this, settings);
                }
            }
        });

        this
    }

    pub fn set_static_information(
        &self,
        index: Option<usize>,
        gpu: &crate::sys_info_v2::GpuStaticInfo,
    ) -> bool {
        imp::PerformancePageGpu::set_static_information(self, index, gpu)
    }

    pub fn update_readings(
        &self,
        gpu: &crate::sys_info_v2::GpuDynamicInfo,
        gpu_static: &crate::sys_info_v2::GpuStaticInfo,
    ) -> bool {
        imp::PerformancePageGpu::update_readings(self, gpu, gpu_static)
    }
}
