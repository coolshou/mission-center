/* performance_page/disk.rs
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

use crate::i18n::*;

use super::widgets::GraphWidget;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageDisk)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/disk.ui")]
    pub struct PerformancePageDisk {
        #[template_child]
        pub disk_id: TemplateChild<gtk::Label>,
        #[template_child]
        pub model: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub max_y: TemplateChild<gtk::Label>,
        #[template_child]
        pub graph_max_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub disk_transfer_rate_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub active_time: TemplateChild<gtk::Label>,
        #[template_child]
        pub avg_response_time: TemplateChild<gtk::Label>,
        #[template_child]
        pub legend_read: TemplateChild<gtk::Picture>,
        #[template_child]
        pub read_speed: TemplateChild<gtk::Label>,
        #[template_child]
        pub legend_write: TemplateChild<gtk::Picture>,
        #[template_child]
        pub write_speed: TemplateChild<gtk::Label>,
        #[template_child]
        pub capacity: TemplateChild<gtk::Label>,
        #[template_child]
        pub formatted: TemplateChild<gtk::Label>,
        #[template_child]
        pub system_disk: TemplateChild<gtk::Label>,
        #[template_child]
        pub disk_type: TemplateChild<gtk::Label>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get = Self::name, set = Self::set_name, type = String)]
        name: Cell<String>,
        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,
    }

    impl Default for PerformancePageDisk {
        fn default() -> Self {
            Self {
                disk_id: Default::default(),
                model: Default::default(),
                usage_graph: Default::default(),
                max_y: Default::default(),
                graph_max_duration: Default::default(),
                disk_transfer_rate_graph: Default::default(),
                active_time: Default::default(),
                avg_response_time: Default::default(),
                legend_read: Default::default(),
                read_speed: Default::default(),
                legend_write: Default::default(),
                write_speed: Default::default(),
                capacity: Default::default(),
                formatted: Default::default(),
                system_disk: Default::default(),
                disk_type: Default::default(),
                context_menu: Default::default(),

                name: Cell::new(String::new()),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),
            }
        }
    }

    impl PerformancePageDisk {
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

    impl PerformancePageDisk {
        fn configure_actions(this: &super::PerformancePageDisk) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let action = gio::SimpleAction::new("copy", None);
            action.connect_activate(clone!(@weak this => move |_, _| {
                let clipboard = this.clipboard();
                clipboard.set_text(this.imp().data_summary().as_str());
            }));
            actions.add_action(&action);
        }

        fn configure_context_menu(this: &super::PerformancePageDisk) {
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

    impl PerformancePageDisk {
        pub fn set_static_information(
            this: &super::PerformancePageDisk,
            index: usize,
            disk: &crate::sys_info_v2::Disk,
        ) -> bool {
            use crate::sys_info_v2::DiskType;

            let t = this.clone();
            this.imp()
                .usage_graph
                .connect_resize(move |graph_widget, width, height| {
                    let width = width as f32;
                    let height = height as f32;

                    let mut a = width;
                    let mut b = height;
                    if width > height {
                        a = height;
                        b = width;
                    }

                    graph_widget
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                    t.imp()
                        .disk_transfer_rate_graph
                        .set_vertical_line_count((width / 40.).round() as u32);
                });

            let this = this.imp();

            this.disk_id
                .set_text(&i18n_f("Disk {} ({})", &[&format!("{}", index), &disk.id]));
            this.model.set_text(&disk.model);

            this.disk_transfer_rate_graph.set_dashed(0, true);
            this.disk_transfer_rate_graph.set_filled(0, false);

            this.legend_read
                .set_resource(Some("/io/missioncenter/MissionCenter/line-dashed-disk.svg"));
            this.legend_write
                .set_resource(Some("/io/missioncenter/MissionCenter/line-solid-disk.svg"));

            let capacity = crate::to_human_readable(disk.capacity as f32, 1024.);
            this.capacity.set_text(&format!(
                "{:.2} {}{}B",
                capacity.0,
                capacity.1,
                if capacity.1.is_empty() { "" } else { "i" }
            ));

            let formatted = crate::to_human_readable(disk.formatted as f32, 1024.);
            this.formatted.set_text(&format!(
                "{:.2} {}{}B",
                formatted.0,
                formatted.1,
                if formatted.1.is_empty() { "" } else { "i" }
            ));

            let is_system_disk = if disk.system_disk {
                i18n("Yes")
            } else {
                i18n("No")
            };
            this.system_disk.set_text(&is_system_disk);

            this.disk_type.set_text(match disk.r#type {
                DiskType::HDD => "HDD",
                DiskType::SSD => "SSD",
                DiskType::NVMe => "NVMe",
                DiskType::eMMC => "eMMC",
                DiskType::iSCSI => "iSCSI",
                DiskType::Unknown => "Unknown",
            });

            true
        }

        pub fn update_readings(
            this: &super::PerformancePageDisk,
            disk: &crate::sys_info_v2::Disk,
        ) -> bool {
            let this = this.imp();

            let max_y =
                crate::to_human_readable(this.disk_transfer_rate_graph.value_range_max(), 1024.);
            let i = if max_y.1.is_empty() { "" } else { "i" };
            this.max_y.set_text(&i18n_f(
                "{} {}{}B/s",
                &[
                    &format!("{}", max_y.0.round()),
                    &format!("{}", max_y.1),
                    &format!("{}", i),
                ],
            ));

            this.usage_graph.add_data_point(0, disk.busy_percent as f32);

            this.active_time
                .set_text(&format!("{}%", disk.busy_percent.round() as u8));

            this.avg_response_time
                .set_text(&format!("{:.2} ms", disk.response_time_ms));

            this.disk_transfer_rate_graph
                .add_data_point(0, disk.read_speed as f32);
            let read_speed = crate::to_human_readable(disk.read_speed as f32, 1024.);
            let i = if read_speed.1.is_empty() { "" } else { "i" };
            this.read_speed.set_text(&format!(
                "{0:.2$} {1}{3}B/s",
                read_speed.0, read_speed.1, read_speed.2, i,
            ));

            this.disk_transfer_rate_graph
                .add_data_point(1, disk.write_speed as f32);
            let write_speed = crate::to_human_readable(disk.write_speed as f32, 1024.);
            let i = if write_speed.1.is_empty() { "" } else { "i" };
            this.write_speed.set_text(&format!(
                "{0:.2$} {1}{3}B/s",
                write_speed.0, write_speed.1, write_speed.2, i,
            ));

            true
        }

        fn data_summary(&self) -> String {
            format!(
                r#"{}

    {}

    Capacity:    {}
    Formatted:   {}
    System disk: {}
    Type:        {}

    Read speed:            {}
    Write speed:           {}
    Active time:           {}
    Average response time: {}"#,
                self.disk_id.label(),
                self.model.label(),
                self.capacity.label(),
                self.formatted.label(),
                self.system_disk.label(),
                self.disk_type.label(),
                self.read_speed.label(),
                self.write_speed.label(),
                self.active_time.label(),
                self.avg_response_time.label(),
            )
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePageDisk {
        const NAME: &'static str = "PerformancePageDisk";
        type Type = super::PerformancePageDisk;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePageDisk {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageDisk>().clone();

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

    impl WidgetImpl for PerformancePageDisk {}

    impl BoxImpl for PerformancePageDisk {}
}

glib::wrapper! {
    pub struct PerformancePageDisk(ObjectSubclass<imp::PerformancePageDisk>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePageDisk {
    pub fn new(name: &str, settings: &gio::Settings) -> Self {
        let this: Self = glib::Object::builder().property("name", name).build();

        fn update_refresh_rate_sensitive_labels(
            this: &PerformancePageDisk,
            settings: &gio::Settings,
        ) {
            let update_speed_ms = settings.int("update-speed") * 500;
            let graph_max_duration = (update_speed_ms * 60) / 1000;

            let this = this.imp();
            this.graph_max_duration
                .set_text(&i18n_f("{} seconds", &[&format!("{}", graph_max_duration)]))
        }
        update_refresh_rate_sensitive_labels(&this, settings);

        settings.connect_changed(
            Some("update-speed"),
            clone!(@weak this => move |settings, _| {
                update_refresh_rate_sensitive_labels(&this, settings);
            }),
        );

        this
    }

    pub fn set_static_information(&self, index: usize, disk: &crate::sys_info_v2::Disk) -> bool {
        imp::PerformancePageDisk::set_static_information(self, index, disk)
    }

    pub fn update_readings(&self, disk: &crate::sys_info_v2::Disk) -> bool {
        imp::PerformancePageDisk::update_readings(self, disk)
    }
}
