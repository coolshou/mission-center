/* performance_page/view_models
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

use std::{cell::Cell, collections::HashMap};

use adw::subclass::prelude::*;
use glib::{clone, ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*};

use widgets::GraphWidget;

use crate::i18n::*;

mod cpu;
mod disk;
mod gpu;
mod memory;
mod network;
mod summary_graph;
mod widgets;

type SummaryGraph = summary_graph::SummaryGraph;
type CpuPage = cpu::PerformancePageCpu;
type DiskPage = disk::PerformancePageDisk;
type MemoryPage = memory::PerformancePageMemory;
type NetworkPage = network::PerformancePageNetwork;
type GpuPage = gpu::PerformancePageGpu;

mod imp {
    use super::*;

    enum Pages {
        Cpu((SummaryGraph, CpuPage)),
        Memory((SummaryGraph, MemoryPage)),
        Disk(HashMap<String, (SummaryGraph, DiskPage)>),
        Network(HashMap<String, (SummaryGraph, NetworkPage)>),
        Gpu(HashMap<String, (SummaryGraph, GpuPage)>),
    }

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePage)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/page.ui")]
    pub struct PerformancePage {
        #[template_child]
        pub sidebar: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub page_stack: TemplateChild<gtk::Stack>,

        #[property(get, set)]
        summary_mode: Cell<bool>,

        pages: Cell<Vec<Pages>>,
        context_menu_view_actions: Cell<HashMap<String, gio::SimpleAction>>,
        current_view_action: Cell<gio::SimpleAction>,

        pub settings: Cell<Option<gio::Settings>>,
    }

    impl Default for PerformancePage {
        fn default() -> Self {
            Self {
                sidebar: Default::default(),
                page_stack: Default::default(),

                summary_mode: Cell::new(false),

                pages: Cell::new(Vec::new()),
                context_menu_view_actions: Cell::new(HashMap::new()),
                current_view_action: Cell::new(gio::SimpleAction::new("", None)),

                settings: Cell::new(None),
            }
        }
    }

    impl PerformancePage {
        fn configure_actions(this: &super::PerformancePage) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let mut view_actions = HashMap::new();

            let action = gio::SimpleAction::new_stateful(
                "summary",
                None,
                glib::Variant::from(this.imp().summary_mode.get()),
            );
            action.connect_activate(clone!(@weak this => move |action, _| {
                let new_state = !this.summary_mode();
                action.set_state(glib::Variant::from(new_state));
                this.set_summary_mode(new_state);
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new_stateful("cpu", None, glib::Variant::from(true));
            action.connect_activate(clone!(@weak this => move |action, _| {
                let row= this.imp()
                    .sidebar
                    .row_at_index(0)
                    .expect("Failed to select CPU row");
                this.imp().sidebar.select_row(Some(&row));

                let prev_action = this.imp().current_view_action.replace(action.clone());
                prev_action.set_state(glib::Variant::from(false));
                action.set_state(glib::Variant::from(true));
            }));
            actions.add_action(&action);
            view_actions.insert("cpu".to_string(), action.clone());
            this.imp().current_view_action.set(action);

            let action =
                gio::SimpleAction::new_stateful("memory", None, glib::Variant::from(false));
            action.connect_activate(clone!(@weak this => move |action, _| {
                let row= this.imp()
                    .sidebar
                    .row_at_index(1)
                    .expect("Failed to select Memory row");
                this.imp().sidebar.select_row(Some(&row));

                let prev_action = this.imp().current_view_action.replace(action.clone());
                prev_action.set_state(glib::Variant::from(false));
                action.set_state(glib::Variant::from(true));
            }));
            actions.add_action(&action);
            view_actions.insert("memory".to_string(), action);

            let action = gio::SimpleAction::new_stateful("disk", None, glib::Variant::from(false));
            action.connect_activate(clone!(@weak this => move |action, _| {
                let pages = this.imp().pages.take();
                for page in &pages {
                    let disk_pages = match page {
                        Pages::Disk(disk_pages) => {
                            disk_pages
                        }
                        _ => continue,
                    };

                    let disk_page = disk_pages.values().next();
                    if disk_page.is_none() {
                        continue;
                    }
                    let disk_page = disk_page.unwrap();

                    let row = disk_page.0.parent();
                    if row.is_none() {
                        continue;
                    }
                    let row = row.unwrap();

                    this.imp().sidebar.select_row(row.downcast_ref::<gtk::ListBoxRow>());

                    let prev_action = this.imp().current_view_action.replace(action.clone());
                    prev_action.set_state(glib::Variant::from(false));
                    action.set_state(glib::Variant::from(true));

                    break;
                }
                this.imp().pages.set(pages);
            }));
            actions.add_action(&action);
            view_actions.insert("disk".to_string(), action);

            let action =
                gio::SimpleAction::new_stateful("network", None, glib::Variant::from(false));
            action.connect_activate(clone!(@weak this => move |action, _| {
                let pages = this.imp().pages.take();
                for page in &pages {
                    let network_pages= match page {
                        Pages::Network(network_pages) => {
                            network_pages
                        }
                        _ => continue,
                    };

                    let network_page = network_pages.values().next();
                    if network_page.is_none() {
                        continue;
                    }
                    let network_page = network_page.unwrap();

                    let row = network_page.0.parent();
                    if row.is_none() {
                        continue;
                    }
                    let row = row.unwrap();

                    this.imp().sidebar.select_row(row.downcast_ref::<gtk::ListBoxRow>());

                    let prev_action = this.imp().current_view_action.replace(action.clone());
                    prev_action.set_state(glib::Variant::from(false));
                    action.set_state(glib::Variant::from(true));

                    break;
                }
                this.imp().pages.set(pages);
            }));
            actions.add_action(&action);
            view_actions.insert("network".to_string(), action);

            let action = gio::SimpleAction::new_stateful("gpu", None, glib::Variant::from(false));
            action.connect_activate(clone!(@weak this => move |action, _| {
                let pages = this.imp().pages.take();
                for page in &pages {
                    let gpu_pages= match page {
                        Pages::Gpu(gpu_pages) => {
                            gpu_pages
                        }
                        _ => continue,
                    };

                    let gpu_page = gpu_pages.values().next();
                    if gpu_page.is_none() {
                        continue;
                    }
                    let gpu_page = gpu_page.unwrap();

                    let row = gpu_page.0.parent();
                    if row.is_none() {
                        continue;
                    }
                    let row = row.unwrap();

                    this.imp().sidebar.select_row(row.downcast_ref::<gtk::ListBoxRow>());

                    let prev_action = this.imp().current_view_action.replace(action.clone());
                    prev_action.set_state(glib::Variant::from(false));
                    action.set_state(glib::Variant::from(true));

                    break;
                }
                this.imp().pages.set(pages);
            }));
            actions.add_action(&action);
            view_actions.insert("gpu".to_string(), action);

            this.imp().context_menu_view_actions.set(view_actions);
        }

        fn set_up_cpu_page(&self, pages: &mut Vec<Pages>, readings: &crate::sys_info_v2::Readings) {
            // GNOME color palette: Blue 4
            const BASE_COLOR: [u8; 3] = [0x1c, 0x71, 0xd8];

            let summary = SummaryGraph::new();
            summary.set_widget_name("cpu");

            summary.set_heading(i18n("CPU"));
            summary.set_info1("0% 0.00 GHz");
            match readings.cpu_info.dynamic_info.temperature.as_ref() {
                Some(v) => summary.set_info2(format!("{:.2} °C", *v)),
                _ => {}
            }

            summary.set_base_color(gtk::gdk::RGBA::new(
                BASE_COLOR[0] as f32 / 255.,
                BASE_COLOR[1] as f32 / 255.,
                BASE_COLOR[2] as f32 / 255.,
                1.,
            ));

            let settings = self.settings.take();
            if settings.is_none() {
                panic!("Settings not set");
            }
            let page = CpuPage::new(settings.as_ref().unwrap());
            page.set_base_color(gtk::gdk::RGBA::new(
                BASE_COLOR[0] as f32 / 255.,
                BASE_COLOR[1] as f32 / 255.,
                BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            self.settings.set(settings);
            page.set_static_information(readings);

            self.obj()
                .as_ref()
                .bind_property("summary-mode", &page, "summary-mode")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            self.sidebar.append(&summary);
            self.page_stack.add_named(&page, Some("cpu"));

            pages.push(Pages::Cpu((summary, page)));
        }

        fn set_up_memory_page(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            // GNOME color palette: Blue 2
            const BASE_COLOR: [u8; 3] = [0x62, 0xa0, 0xea];

            let summary = SummaryGraph::new();
            summary.set_widget_name("memory");

            summary
                .graph_widget()
                .set_value_range_max(readings.mem_info.mem_total as f32);

            summary.set_heading(i18n("Memory"));
            summary.set_info1("0/0 GiB (100%)");

            summary.set_base_color(gtk::gdk::RGBA::new(
                BASE_COLOR[0] as f32 / 255.,
                BASE_COLOR[1] as f32 / 255.,
                BASE_COLOR[2] as f32 / 255.,
                1.,
            ));

            let settings = self.settings.take();
            if settings.is_none() {
                panic!("Settings not set");
            }
            let page = MemoryPage::new(settings.as_ref().unwrap());
            page.set_base_color(gtk::gdk::RGBA::new(
                BASE_COLOR[0] as f32 / 255.,
                BASE_COLOR[1] as f32 / 255.,
                BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            self.settings.set(settings);
            page.set_static_information(readings);

            self.obj()
                .as_ref()
                .bind_property("summary-mode", &page, "summary-mode")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            self.sidebar.append(&summary);
            self.page_stack.add_named(&page, Some("memory"));

            pages.push(Pages::Memory((summary, page)));
        }

        fn set_up_disk_pages(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            use crate::sys_info_v2::DiskType;
            use glib::g_critical;

            // GNOME color palette: Green 5
            const BASE_COLOR: [u8; 3] = [0x26, 0xa2, 0x69];

            let mut disks = HashMap::new();
            for (i, disk) in readings.disks.iter().enumerate() {
                let summary = SummaryGraph::new();
                summary.set_widget_name(&disk.id);

                summary.set_heading(i18n_f(
                    "Disk {} ({})",
                    &[&format!("{}", i), &format!("{}", &disk.id)],
                ));
                summary.set_info1(match disk.r#type {
                    DiskType::HDD => i18n("HDD"),
                    DiskType::SSD => i18n("SSD"),
                    DiskType::NVMe => i18n("NVMe"),
                    DiskType::eMMC => i18n("eMMC"),
                    DiskType::iSCSI => i18n("iSCSI"),
                    DiskType::Unknown => i18n("Unknown"),
                });
                summary.set_info2(format!("{:.2}%", disk.busy_percent));
                summary.set_base_color(gtk::gdk::RGBA::new(
                    BASE_COLOR[0] as f32 / 255.,
                    BASE_COLOR[1] as f32 / 255.,
                    BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));

                let settings = self.settings.take();
                if settings.is_none() {
                    panic!("Settings not set");
                }
                let page = DiskPage::new(&disk.id, settings.as_ref().unwrap());
                page.set_base_color(gtk::gdk::RGBA::new(
                    BASE_COLOR[0] as f32 / 255.,
                    BASE_COLOR[1] as f32 / 255.,
                    BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));
                self.settings.set(settings);
                page.set_static_information(i, disk);

                self.obj()
                    .as_ref()
                    .bind_property("summary-mode", &page, "summary-mode")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();

                self.sidebar.append(&summary);
                self.page_stack.add_named(&page, Some(&disk.id));

                let mut actions = self.context_menu_view_actions.take();
                match actions.get("disk") {
                    None => {
                        g_critical!(
                            "MissionCenter::PerformancePage",
                            "Failed to wire up disk action for {}, logic bug?",
                            &disk.id
                        );
                    }
                    Some(action) => {
                        actions.insert(disk.id.clone(), action.clone());
                    }
                }
                self.context_menu_view_actions.set(actions);

                disks.insert(disk.id.clone(), (summary, page));
            }

            pages.push(Pages::Disk(disks));
        }

        fn set_up_network_pages(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            use crate::sys_info_v2::NetDeviceType;
            use glib::g_critical;

            // GNOME color palette: Purple 1
            const BASE_COLOR: [u8; 3] = [0xdc, 0x8a, 0xdd];

            let mut networks = HashMap::new();
            for network_device in &readings.network_devices {
                let if_name = network_device.descriptor.if_name.as_str();

                let conn_type = match network_device.descriptor.r#type {
                    NetDeviceType::Wired => i18n("Ethernet"),
                    NetDeviceType::Wireless => i18n("Wi-Fi"),
                    NetDeviceType::Other => i18n("Other"),
                };

                let summary = SummaryGraph::new();
                summary.set_widget_name(if_name);

                summary.set_heading(conn_type.clone());
                summary.set_info1(if_name.to_string());

                {
                    let graph_widget = summary.graph_widget();

                    graph_widget.set_data_set_count(2);
                    graph_widget.set_auto_scale(true);
                    graph_widget.set_auto_scale_pow2(true);
                    graph_widget.set_filled(0, false);
                    graph_widget.set_dashed(0, true);
                    graph_widget.set_base_color(gtk::gdk::RGBA::new(
                        BASE_COLOR[0] as f32 / 255.,
                        BASE_COLOR[1] as f32 / 255.,
                        BASE_COLOR[2] as f32 / 255.,
                        1.,
                    ));
                }

                let settings = self.settings.take();
                if settings.is_none() {
                    panic!("Settings not set");
                }
                let page = NetworkPage::new(
                    if_name,
                    network_device.descriptor.r#type,
                    settings.as_ref().unwrap(),
                );
                page.set_base_color(gtk::gdk::RGBA::new(
                    BASE_COLOR[0] as f32 / 255.,
                    BASE_COLOR[1] as f32 / 255.,
                    BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));
                self.settings.set(settings);
                page.set_static_information(network_device);

                self.obj()
                    .as_ref()
                    .bind_property("summary-mode", &page, "summary-mode")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();

                self.sidebar.append(&summary);
                self.page_stack.add_named(&page, Some(if_name));

                let mut actions = self.context_menu_view_actions.take();
                match actions.get("network") {
                    None => {
                        g_critical!(
                            "MissionCenter::PerformancePage",
                            "Failed to wire up network action for {}, logic bug?",
                            if_name
                        );
                    }
                    Some(action) => {
                        actions.insert(if_name.to_owned(), action.clone());
                    }
                }
                self.context_menu_view_actions.set(actions);

                networks.insert(if_name.to_owned(), (summary, page));
            }

            pages.push(Pages::Network(networks));
        }

        fn set_up_gpu_pages(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            use gtk::glib::*;

            // GNOME color palette: Red 1
            const BASE_COLOR: [u8; 3] = [0xf6, 0x61, 0x51];

            let mut gpus = HashMap::new();

            for (i, gpu) in readings.gpus.iter().enumerate() {
                let summary = SummaryGraph::new();
                summary.set_widget_name(&gpu.static_info.id);

                summary.set_heading(i18n_f("GPU {}", &[&format!("{}", i)]));
                summary.set_info1(gpu.static_info.device_name.clone());
                summary.set_info2(format!(
                    "{}% ({} °C)",
                    gpu.dynamic_info.util_percent, gpu.dynamic_info.temp_celsius
                ));
                summary.set_base_color(gtk::gdk::RGBA::new(
                    BASE_COLOR[0] as f32 / 255.,
                    BASE_COLOR[1] as f32 / 255.,
                    BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));

                let page = GpuPage::new(&gpu.static_info.device_name);
                page.set_base_color(gtk::gdk::RGBA::new(
                    BASE_COLOR[0] as f32 / 255.,
                    BASE_COLOR[1] as f32 / 255.,
                    BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));
                page.set_static_information(i, gpu);

                self.obj()
                    .as_ref()
                    .bind_property("summary-mode", &page, "summary-mode")
                    .flags(glib::BindingFlags::SYNC_CREATE)
                    .build();

                self.sidebar.append(&summary);
                self.page_stack.add_named(&page, Some(&gpu.static_info.id));

                let mut actions = self.context_menu_view_actions.take();
                match actions.get("gpu") {
                    None => {
                        g_critical!(
                            "MissionCenter::PerformancePage",
                            "Failed to wire up GPU action for {}, logic bug?",
                            &gpu.static_info.device_name
                        );
                    }
                    Some(action) => {
                        actions.insert(gpu.static_info.id.clone(), action.clone());
                    }
                }
                self.context_menu_view_actions.set(actions);

                gpus.insert(gpu.static_info.device_name.clone(), (summary, page));
            }

            pages.push(Pages::Gpu(gpus));
        }
    }

    impl PerformancePage {
        pub fn set_up_pages(
            this: &super::PerformancePage,
            readings: &crate::sys_info_v2::Readings,
        ) -> bool {
            let this = this.imp();

            let mut pages = vec![];
            this.set_up_cpu_page(&mut pages, &readings);
            this.set_up_memory_page(&mut pages, &readings);
            this.set_up_disk_pages(&mut pages, &readings);
            this.set_up_network_pages(&mut pages, &readings);
            this.set_up_gpu_pages(&mut pages, &readings);
            this.pages.set(pages);

            if let Some(settings) = this.settings.take() {
                let view_actions = this.context_menu_view_actions.take();
                let action = if let Some(action) =
                    view_actions.get(settings.string("performance-selected-page").as_str())
                {
                    action
                } else {
                    view_actions.get("cpu").expect("All computers have a CPU")
                };
                action.activate(None);

                this.context_menu_view_actions.set(view_actions);
                this.settings.set(Some(settings));
            }

            true
        }

        pub fn update_readings(
            this: &super::PerformancePage,
            readings: &crate::sys_info_v2::Readings,
        ) -> bool {
            let pages = this.imp().pages.take();

            let mut result = true;

            for page in &pages {
                match page {
                    Pages::Cpu((summary, page)) => {
                        summary
                            .graph_widget()
                            .add_data_point(0, readings.cpu_info.dynamic_info.utilization_percent);
                        summary.set_info1(format!(
                            "{}% {:.2} Ghz",
                            readings.cpu_info.dynamic_info.utilization_percent.round(),
                            readings.cpu_info.dynamic_info.current_frequency_mhz as f32 / 1024.
                        ));
                        match readings.cpu_info.dynamic_info.temperature.as_ref() {
                            Some(v) => summary.set_info2(format!("{:.2} °C", *v)),
                            _ => {}
                        }

                        result &= page.update_readings(readings);
                    }
                    Pages::Memory((summary, page)) => {
                        let total_raw = readings.mem_info.mem_total;
                        let total = crate::to_human_readable(total_raw as _, 1024.);
                        let used_raw =
                            readings.mem_info.mem_total - readings.mem_info.mem_available;
                        summary.graph_widget().add_data_point(0, used_raw as _);
                        let used = crate::to_human_readable(used_raw as _, 1024.);

                        summary.set_info1(format!(
                            "{:.2} {}iB/{} {}iB ({}%)",
                            used.0,
                            used.1,
                            total.0.round(),
                            total.1,
                            ((used_raw as f32 / total_raw as f32) * 100.).round()
                        ));

                        result &= page.update_readings(readings);
                    }
                    Pages::Disk(disks_pages) => {
                        for disk in &readings.disks {
                            if let Some((summary, page)) = disks_pages.get(&disk.id) {
                                let graph_widget = summary.graph_widget();
                                graph_widget.add_data_point(0, disk.busy_percent);
                                summary.set_info2(format!("{:.2}%", disk.busy_percent));

                                result &= page.update_readings(disk);
                            } else {
                                // New page? How to detect disk was removed?
                            }
                        }
                    }
                    Pages::Network(pages) => {
                        for network_device in &readings.network_devices {
                            if let Some((summary, page)) =
                                pages.get(&network_device.descriptor.if_name)
                            {
                                let sent = network_device.send_bps as f32;
                                let received = network_device.recv_bps as f32;

                                let graph_widget = summary.graph_widget();
                                graph_widget.add_data_point(0, sent);
                                graph_widget.add_data_point(1, received);

                                let sent = crate::to_human_readable(sent * 8., 1024.);
                                let received = crate::to_human_readable(received * 8., 1024.);
                                summary.set_info2(i18n_f(
                                    "{}: {} {}bps {}: {} {}bps",
                                    &[
                                        "S",
                                        &format!("{}", sent.0.round()),
                                        &format!("{}", sent.1),
                                        "R",
                                        &format!("{}", received.0.round()),
                                        &format!("{}", received.1),
                                    ],
                                ));

                                result &= page.update_readings(network_device);
                            }
                        }
                    }
                    Pages::Gpu(pages) => {
                        for gpu in &readings.gpus {
                            if let Some((summary, page)) = pages.get(&gpu.static_info.device_name) {
                                let graph_widget = summary.graph_widget();
                                graph_widget
                                    .add_data_point(0, gpu.dynamic_info.util_percent as f32);
                                summary.set_info2(format!(
                                    "{}% ({} °C)",
                                    gpu.dynamic_info.util_percent, gpu.dynamic_info.temp_celsius
                                ));

                                result &= page.update_readings(gpu);
                            }
                        }
                    }
                }
            }

            this.imp().pages.set(pages);

            result
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePage {
        const NAME: &'static str = "PerformancePage";
        type Type = super::PerformancePage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            SummaryGraph::ensure_type();
            GraphWidget::ensure_type();
            CpuPage::ensure_type();
            NetworkPage::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePage {
        fn constructed(&self) {
            self.parent_constructed();

            let this = self.obj().as_ref().clone();

            if let Some(app) = crate::MissionCenterApplication::default_instance() {
                self.settings.set(app.settings());
            }

            Self::configure_actions(&this);

            self.sidebar.connect_row_selected(move |_, selected_row| {
                use glib::{translate::*, *};
                use std::ffi::CStr;

                if let Some(row) = selected_row {
                    let child = row.child();
                    if child.is_none() {
                        g_critical!(
                            "MissionCenter::PerformancePage",
                            "Failed to get child of selected row"
                        );
                    }
                    let child = child.unwrap();

                    let widget_name =
                        unsafe { gtk::ffi::gtk_widget_get_name(child.to_glib_none().0) };
                    if widget_name.is_null() {
                        return;
                    }
                    let page_name = unsafe { CStr::from_ptr(widget_name) }.to_string_lossy();
                    let page_name = page_name.as_ref();

                    let imp = this.imp();

                    let actions = imp.context_menu_view_actions.take();
                    if let Some(new_action) = actions.get(page_name) {
                        let prev_action = imp.current_view_action.replace(new_action.clone());
                        prev_action.set_state(glib::Variant::from(false));
                        new_action.set_state(glib::Variant::from(true));
                    }

                    imp.context_menu_view_actions.set(actions);
                    imp.page_stack.set_visible_child_name(page_name);

                    if let Some(settings) = imp.settings.take() {
                        settings
                            .set_string("performance-selected-page", page_name)
                            .unwrap_or_else(|_| {
                                g_warning!(
                                    "MissionCenter::PerformancePage",
                                    "Failed to set performance-selected-page setting"
                                );
                            });
                        imp.settings.set(Some(settings));
                    }
                }
            });
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

    impl WidgetImpl for PerformancePage {}

    impl BoxImpl for PerformancePage {}
}

glib::wrapper! {
    pub struct PerformancePage(ObjectSubclass<imp::PerformancePage>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePage {
    pub fn set_up_pages(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        imp::PerformancePage::set_up_pages(self, readings)
    }

    pub fn update_readings(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        imp::PerformancePage::update_readings(self, readings)
    }
}
