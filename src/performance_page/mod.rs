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

use adw::{prelude::*, subclass::prelude::*};
use glib::{clone, ParamSpec, Properties, Value};
use gtk::{gio, glib};

use widgets::GraphWidget;

use crate::{
    application::BASE_POINTS,
    i18n::*,
    performance_page::disk::PerformancePageDisk,
    performance_page::summary_graph::compare_to,
    sys_info_v2::{Disk, DiskType},
};

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
    use crate::performance_page::network::PerformancePageNetwork;
    use crate::sys_info_v2::{NetDeviceType, NetworkDevice};
    use super::*;

    // GNOME color palette: Blue 2
    pub const MEMORY_BASE_COLOR: [u8; 3] = [0x62, 0xa0, 0xea];
    // GNOME color palette: Green 5
    pub const DISK_BASE_COLOR: [u8; 3] = [0x26, 0xa2, 0x69];
    // GNOME color palette: Purple 1
    pub const NETWORK_BASE_COLOR: [u8; 3] = [0xdc, 0x8a, 0xdd];

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
        pub breakpoint: TemplateChild<adw::Breakpoint>,
        #[template_child]
        pub page_content: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub page_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub info_bar: TemplateChild<adw::Bin>,

        #[property(get = Self::sidebar, set = Self::set_sidebar)]
        pub sidebar: Cell<gtk::ListBox>,
        #[property(get, set)]
        summary_mode: Cell<bool>,
        #[property(name = "infobar-visible", get = Self::infobar_visible, set = Self::set_infobar_visible, type = bool)]
        _infobar_visible: [u8; 0],
        #[property(name = "info-button-visible", get = Self::info_button_visible, type = bool)]
        _info_button_visible: [u8; 0],

        breakpoint_applied: Cell<bool>,

        pages: Cell<Vec<Pages>>,

        action_group: Cell<gio::SimpleActionGroup>,
        context_menu_view_actions: Cell<HashMap<String, gio::SimpleAction>>,
        current_view_action: Cell<gio::SimpleAction>,

        pub settings: Cell<Option<gio::Settings>>,
    }

    impl Default for PerformancePage {
        fn default() -> Self {
            Self {
                breakpoint: Default::default(),
                page_content: Default::default(),
                page_stack: Default::default(),
                info_bar: Default::default(),

                sidebar: Cell::new(gtk::ListBox::new()),
                summary_mode: Cell::new(false),
                _infobar_visible: [0; 0],
                _info_button_visible: [0; 0],

                breakpoint_applied: Cell::new(false),

                pages: Cell::new(Vec::new()),

                action_group: Cell::new(gio::SimpleActionGroup::new()),
                context_menu_view_actions: Cell::new(HashMap::new()),
                current_view_action: Cell::new(gio::SimpleAction::new("", None)),

                settings: Cell::new(None),
            }
        }
    }

    impl PerformancePage {
        fn sidebar(&self) -> gtk::ListBox {
            unsafe { &*self.sidebar.as_ptr() }.clone()
        }

        fn set_sidebar(&self, lb: &gtk::ListBox) {
            let this = self.obj().as_ref().clone();

            Self::configure_actions(&this);
            lb.connect_row_selected(move |_, selected_row| {
                use glib::{*, translate::*};
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
                        prev_action.set_state(&glib::Variant::from(false));
                        new_action.set_state(&glib::Variant::from(true));
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

            self.sidebar.set(lb.clone())
        }

        fn infobar_visible(&self) -> bool {
            self.page_content.shows_sidebar()
        }

        fn set_infobar_visible(&self, v: bool) {
            self.page_content
                .set_show_sidebar(!self.page_content.is_collapsed() || v);
        }

        fn info_button_visible(&self) -> bool {
            self.page_content.is_collapsed()
        }
    }

    impl PerformancePage {
        fn configure_actions(this: &super::PerformancePage) {
            let actions = unsafe { &*this.imp().action_group.as_ptr() }.clone();

            let mut view_actions = HashMap::new();

            let action = gio::SimpleAction::new_stateful(
                "summary",
                None,
                &glib::Variant::from(this.imp().summary_mode.get()),
            );
            action.connect_activate(clone!(@weak this => move |action, _| {
                let new_state = !this.summary_mode();
                action.set_state(&glib::Variant::from(new_state));
                this.set_summary_mode(new_state);
                if !this.imp().breakpoint_applied.get() {
                    this.imp().page_content.set_show_sidebar(!new_state);
                }
            }));
            actions.add_action(&action);

            let action = gio::SimpleAction::new_stateful("cpu", None, &glib::Variant::from(true));
            action.connect_activate(clone!(@weak this => move |action, _| {
                let row= this.imp()
                    .sidebar()
                    .row_at_index(0)
                    .expect("Failed to select CPU row");
                this.imp().sidebar().select_row(Some(&row));

                let prev_action = this.imp().current_view_action.replace(action.clone());
                prev_action.set_state(&glib::Variant::from(false));
                action.set_state(&glib::Variant::from(true));
            }));
            actions.add_action(&action);
            view_actions.insert("cpu".to_string(), action.clone());
            this.imp().current_view_action.set(action);

            let action =
                gio::SimpleAction::new_stateful("memory", None, &glib::Variant::from(false));
            action.connect_activate(clone!(@weak this => move |action, _| {
                let row= this.imp()
                    .sidebar()
                    .row_at_index(1)
                    .expect("Failed to select Memory row");
                this.imp().sidebar().select_row(Some(&row));

                let prev_action = this.imp().current_view_action.replace(action.clone());
                prev_action.set_state(&glib::Variant::from(false));
                action.set_state(&glib::Variant::from(true));
            }));
            actions.add_action(&action);
            view_actions.insert("memory".to_string(), action);

            let action = gio::SimpleAction::new_stateful("disk", None, &glib::Variant::from(false));
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

                    this.imp().sidebar().select_row(row.downcast_ref::<gtk::ListBoxRow>());

                    let prev_action = this.imp().current_view_action.replace(action.clone());
                    prev_action.set_state(&glib::Variant::from(false));
                    action.set_state(&glib::Variant::from(true));

                    break;
                }
                this.imp().pages.set(pages);
            }));
            actions.add_action(&action);
            view_actions.insert("disk".to_string(), action);

            let action =
                gio::SimpleAction::new_stateful("network", None, &glib::Variant::from(false));
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

                    this.imp().sidebar().select_row(row.downcast_ref::<gtk::ListBoxRow>());

                    let prev_action = this.imp().current_view_action.replace(action.clone());
                    prev_action.set_state(&glib::Variant::from(false));
                    action.set_state(&glib::Variant::from(true));

                    break;
                }
                this.imp().pages.set(pages);
            }));
            actions.add_action(&action);
            view_actions.insert("network".to_string(), action);

            let action = gio::SimpleAction::new_stateful("gpu", None, &glib::Variant::from(false));
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

                    this.imp().sidebar().select_row(row.downcast_ref::<gtk::ListBoxRow>());

                    let prev_action = this.imp().current_view_action.replace(action.clone());
                    prev_action.set_state(&glib::Variant::from(false));
                    action.set_state(&glib::Variant::from(true));

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
            summary.set_page_indicies(&1, &0);
            summary.set_widget_name("cpu");

            summary.set_heading(i18n("CPU"));
            summary.set_info1("0% 0.00 GHz");
            match readings.cpu_dynamic_info.temperature.as_ref() {
                Some(v) => summary.set_info2(format!("{:.0} °C", *v)),
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

            let unwrapped_settings = settings.as_ref().unwrap();
            summary.graph_widget().set_data_points(unwrapped_settings.int("perfomance-page-data-points") as u32);

            let page = CpuPage::new(unwrapped_settings);
            page.set_base_color(gtk::gdk::RGBA::new(
                BASE_COLOR[0] as f32 / 255.,
                BASE_COLOR[1] as f32 / 255.,
                BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            self.settings.set(settings);
            page.set_static_information(readings);

            self.page_content
                .connect_collapsed_notify(clone!(@weak page => move |pc| {
                    if pc.is_collapsed() {
                        page.infobar_collapsed();
                    } else {
                        page.infobar_uncollapsed();
                    }
                }));

            self.obj()
                .as_ref()
                .bind_property("summary-mode", &page, "summary-mode")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            self.sidebar().append(&summary);
            self.page_stack.add_named(&page, Some("cpu"));

            pages.push(Pages::Cpu((summary, page)));
        }

        fn set_up_memory_page(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            let summary = SummaryGraph::new();
            summary.set_page_indicies(&2, &0);
            summary.set_widget_name("memory");

            summary
                .graph_widget()
                .set_value_range_max(readings.mem_info.mem_total as f32);

            summary.set_heading(i18n("Memory"));
            summary.set_info1("0/0 GiB (100%)");

            summary.set_base_color(gtk::gdk::RGBA::new(
                MEMORY_BASE_COLOR[0] as f32 / 255.,
                MEMORY_BASE_COLOR[1] as f32 / 255.,
                MEMORY_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));

            let settings = self.settings.take();
            if settings.is_none() {
                panic!("Settings not set");
            }

            summary.graph_widget().set_data_points(settings.as_ref().unwrap().int("perfomance-page-data-points") as u32);

            let page = MemoryPage::new(settings.as_ref().unwrap());
            page.set_base_color(gtk::gdk::RGBA::new(
                MEMORY_BASE_COLOR[0] as f32 / 255.,
                MEMORY_BASE_COLOR[1] as f32 / 255.,
                MEMORY_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            self.settings.set(settings);
            page.set_static_information(readings);

            self.page_content
                .connect_collapsed_notify(clone!(@weak page => move |pc| {
                    if pc.is_collapsed() {
                        page.infobar_collapsed();
                    } else {
                        page.infobar_uncollapsed();
                    }
                }));

            self.obj()
                .as_ref()
                .bind_property("summary-mode", &page, "summary-mode")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            self.sidebar().append(&summary);
            self.page_stack.add_named(&page, Some("memory"));

            pages.push(Pages::Memory((summary, page)));
        }

        fn set_up_disk_pages(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            let mut disks = HashMap::new();
            for i in 0..readings.disks.len() {
                let ret = self.create_disk_page(&readings.disks[i], i);
                disks.insert(ret.clone().0, ret.1);
            }

            pages.push(Pages::Disk(disks));
        }

        pub fn update_disk_page_index(&self, disk_graph: &SummaryGraph, disk_id: &String, index: &usize) {
            disk_graph.clone().set_page_secondary_index(index);

            disk_graph.set_heading(i18n_f(
                "Disk {} ({})",
                &[&format!("{}", index), &format!("{}", disk_id)],
            ));
        }

        pub fn create_disk_page(&self, disk: &Disk, i: usize) -> (String, (summary_graph::SummaryGraph, PerformancePageDisk)) {
            use glib::g_critical;

            let summary = SummaryGraph::new();
            summary.set_page_indicies(&3, &i);
            summary.set_widget_name(&disk.id);

            self.update_disk_page_index(&summary, &disk.id, &i);
            summary.set_info1(match disk.r#type {
                DiskType::HDD => i18n("HDD"),
                DiskType::SSD => i18n("SSD"),
                DiskType::NVMe => i18n("NVMe"),
                DiskType::eMMC => i18n("eMMC"),
                DiskType::iSCSI => i18n("iSCSI"),
                DiskType::OPTIC => i18n("Optical"),
                DiskType::Unknown => i18n("Unknown"),
            });
            summary.set_info2(format!("{:.0}%", disk.busy_percent));
            summary.set_base_color(gtk::gdk::RGBA::new(
                DISK_BASE_COLOR[0] as f32 / 255.,
                DISK_BASE_COLOR[1] as f32 / 255.,
                DISK_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));

            let settings = self.settings.take();
            if settings.is_none() {
                panic!("Settings not set");
            }

            summary.graph_widget().set_data_points(settings.as_ref().unwrap().int("perfomance-page-data-points") as u32);

            let page = DiskPage::new(&disk.id, settings.as_ref().unwrap());
            page.set_base_color(gtk::gdk::RGBA::new(
                DISK_BASE_COLOR[0] as f32 / 255.,
                DISK_BASE_COLOR[1] as f32 / 255.,
                DISK_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            self.settings.set(settings);
            page.set_static_information(i, disk);

            self.page_content
                .connect_collapsed_notify(clone!(@weak page => move |pc| {
                        if pc.is_collapsed() {
                            page.infobar_collapsed();
                        } else {
                            page.infobar_uncollapsed();
                        }
                    }));

            self.obj()
                .as_ref()
                .bind_property("summary-mode", &page, "summary-mode")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            self.sidebar().append(&summary);
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

            return (disk.id.clone(), (summary, page));
        }

        fn set_up_network_pages(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            // GNOME color palette: Purple 1

            let mut networks = HashMap::new();
            for (i, network_device) in readings.network_devices.iter().enumerate() {
                let ret = self.create_network_page(network_device, &i);

                networks.insert(ret.clone().0, ret.1);
            }

            pages.push(Pages::Network(networks));
        }

        fn create_network_page(&self, network_device: &NetworkDevice, i: &usize) -> (String, (summary_graph::SummaryGraph, PerformancePageNetwork)) {
            use glib::g_critical;

            let if_name = network_device.descriptor.if_name.as_str();

            let conn_type = match network_device.descriptor.r#type {
                NetDeviceType::Wired => i18n("Ethernet"),
                NetDeviceType::Wireless => i18n("Wi-Fi"),
                NetDeviceType::Other => i18n("Other"),
            };

            let summary = SummaryGraph::new();
            summary.set_page_indicies(&4, &i);
            summary.set_widget_name(if_name);

            summary.set_heading(format!("{} ({})", conn_type.clone(), if_name.to_string()));

            {
                let graph_widget = summary.graph_widget();

                graph_widget.set_data_set_count(2);
                graph_widget.set_auto_scale(true);
                graph_widget.set_auto_scale_pow2(true);
                graph_widget.set_filled(0, false);
                graph_widget.set_dashed(0, true);
                graph_widget.set_base_color(gtk::gdk::RGBA::new(
                    NETWORK_BASE_COLOR[0] as f32 / 255.,
                    NETWORK_BASE_COLOR[1] as f32 / 255.,
                    NETWORK_BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));
            }

            let settings = self.settings.take();
            if settings.is_none() {
                panic!("Settings not set");
            }

            summary.graph_widget().set_data_points(settings.as_ref().unwrap().int("perfomance-page-data-points") as u32);

            let page = NetworkPage::new(
                if_name,
                network_device.descriptor.r#type,
                settings.as_ref().unwrap(),
            );
            page.set_base_color(gtk::gdk::RGBA::new(
                NETWORK_BASE_COLOR[0] as f32 / 255.,
                NETWORK_BASE_COLOR[1] as f32 / 255.,
                NETWORK_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            self.settings.set(settings);
            page.set_static_information(network_device);

            self.page_content
                .connect_collapsed_notify(clone!(@weak page => move |pc| {
                        if pc.is_collapsed() {
                            page.infobar_collapsed();
                        } else {
                            page.infobar_uncollapsed();
                        }
                    }));

            self.obj()
                .as_ref()
                .bind_property("summary-mode", &page, "summary-mode")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            self.sidebar().append(&summary);
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
            (if_name.to_string(), (summary, page))
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

            for (i, static_info) in readings.gpu_static_info.iter().enumerate() {
                let dynamic_info = &readings.gpu_dynamic_info[i];

                let summary = SummaryGraph::new();
                summary.set_page_indicies(&5, &i);
                summary.set_widget_name(static_info.id.as_ref());

                let settings = self.settings.take();
                if settings.is_none() {
                    panic!("Settings not set");
                }

                summary.graph_widget().set_data_points(settings.as_ref().unwrap().int("perfomance-page-data-points") as u32);

                self.settings.set(settings);

                summary.set_heading(i18n_f("GPU {}", &[&format!("{}", i)]));
                summary.set_info1(static_info.device_name.as_ref());
                summary.set_info2(format!(
                    "{}% ({} °C)",
                    dynamic_info.util_percent, dynamic_info.temp_celsius
                ));
                summary.set_base_color(gtk::gdk::RGBA::new(
                    BASE_COLOR[0] as f32 / 255.,
                    BASE_COLOR[1] as f32 / 255.,
                    BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));

                let page = GpuPage::new(&static_info.device_name);
                page.set_base_color(gtk::gdk::RGBA::new(
                    BASE_COLOR[0] as f32 / 255.,
                    BASE_COLOR[1] as f32 / 255.,
                    BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));
                page.set_static_information(i, static_info);


                self.page_content
                    .connect_collapsed_notify(clone!(@weak page => move |pc| {
                        if pc.is_collapsed() {
                            page.infobar_collapsed();
                        } else {
                            page.infobar_uncollapsed();
                        }
                    }));

                self.obj()
                    .as_ref()
                    .bind_property("summary-mode", &page, "summary-mode")
                    .flags(BindingFlags::SYNC_CREATE)
                    .build();

                self.sidebar().append(&summary);
                self.page_stack
                    .add_named(&page, Some(static_info.id.as_ref()));

                let mut actions = self.context_menu_view_actions.take();
                match actions.get("gpu") {
                    None => {
                        g_critical!(
                            "MissionCenter::PerformancePage",
                            "Failed to wire up GPU action for {}, logic bug?",
                            &static_info.device_name
                        );
                    }
                    Some(action) => {
                        actions.insert(static_info.id.as_ref().into(), action.clone());
                    }
                }
                self.context_menu_view_actions.set(actions);

                gpus.insert(static_info.id.as_ref().into(), (summary, page));
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

            this.sidebar().set_sort_func(|g1, g2| unsafe {
                let g1 = g1.child().unwrap().unsafe_cast::<SummaryGraph>();
                let g2 = g2.child().unwrap().unsafe_cast::<SummaryGraph>();

                compare_to(g1, g2)
            });

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
            let mut pages = this.imp().pages.take();

            let mut disks_to_destroy = Vec::new();

            for page in &mut pages {
                match page {
                    Pages::Cpu(_) => {} // not dynamic
                    Pages::Memory(_) => {} // not dynamic
                    Pages::Disk(ref mut disks_pages) => {
                        for disk_name in disks_pages.keys() {
                            if !readings.disks.iter().any(|device| &device.id == disk_name) {
                                disks_to_destroy.push(disk_name.clone());
                            }
                        }

                        for disk_name in &disks_to_destroy {
                            let datum = disks_pages.get(disk_name).unwrap();
                            let page = &datum.clone().1;
                            let graf = &datum.0;

                            let parent = &graf.parent().unwrap();
                            this.sidebar().remove(parent);
                            this.imp().page_stack.remove(page);
                            disks_pages.remove(disk_name);
                        }
                    }
                    Pages::Network(net_pages) => {
                        for network_id in net_pages.keys() {
                            if !readings.network_devices.iter().any(|device| &device.descriptor.if_name == network_id) {
                                disks_to_destroy.push(network_id.clone());
                            }
                        }

                        for disk_name in &disks_to_destroy {
                            let datum = net_pages.get(disk_name).unwrap();
                            let page = &datum.clone().1;
                            let graf = &datum.0;

                            let parent = &graf.parent().unwrap();
                            this.sidebar().remove(parent);
                            this.imp().page_stack.remove(page);
                            net_pages.remove(disk_name);
                        }
                    }
                    Pages::Gpu(_) => {}
                }
            }

            let mut result = true;

            let mut data_points = BASE_POINTS;
            let settings = this.imp().settings.take();
            if !settings.is_none() {
                data_points = settings.clone().unwrap().int("perfomance-page-data-points") as u32;
            }
            this.imp().settings.set(settings);

            for page in &mut pages {
                match page {
                    Pages::Cpu((summary, page)) => {
                        let graph_widget = summary.graph_widget();
                        if graph_widget.data_points() != data_points {
                            graph_widget.set_data_points(data_points);
                        }
                        graph_widget.add_data_point(
                            0,
                            readings.cpu_dynamic_info.overall_utilization_percent,
                        );
                        summary.set_info1(format!(
                            "{}% {:.2} Ghz",
                            readings
                                .cpu_dynamic_info
                                .overall_utilization_percent
                                .round(),
                            readings.cpu_dynamic_info.current_frequency_mhz as f32 / 1024.
                        ));
                        match readings.cpu_dynamic_info.temperature.as_ref() {
                            Some(v) => summary.set_info2(format!("{:.0} °C", *v)),
                            _ => {}
                        }

                        result &= page.update_readings(readings);
                    }
                    Pages::Memory((summary, page)) => {
                        let total_raw = readings.mem_info.mem_total;
                        let total = crate::to_human_readable(total_raw as _, 1024.);
                        let used_raw =
                            readings.mem_info.mem_total - readings.mem_info.mem_available;
                        let graph_widget = summary.graph_widget();
                        if graph_widget.data_points() != data_points {
                            graph_widget.set_data_points(data_points);
                        }
                        graph_widget.add_data_point(0, used_raw as _);
                        let used = crate::to_human_readable(used_raw as _, 1024.);

                        summary.set_info1(format!(
                            "{}%",
                            ((used_raw as f32 / total_raw as f32) * 100.).round()
                        ));

                        summary.set_info2(format!(
                            "{0:.2$} {1}iB/{3:.5$} {4}iB",
                            used.0, used.1, used.2, total.0, total.1, total.2
                        ));

                        result &= page.update_readings(readings);
                    }
                    Pages::Disk(ref mut disks_pages) => {
                        let mut new_pages = Vec::new();
                        for (index, disk) in readings.disks.iter().enumerate() {
                            if let Some((summary, page)) = disks_pages.get(&disk.id) {
                                let graph_widget = summary.graph_widget();
                                if graph_widget.data_points() != data_points {
                                    graph_widget.set_data_points(data_points);
                                }
                                this.imp().update_disk_page_index(summary, &disk.id, &index);
                                graph_widget.add_data_point(0, disk.busy_percent);
                                summary.set_info2(format!("{:.0}%", disk.busy_percent));

                                result &= page.update_readings(disk);
                            } else {
                                new_pages.push(this.imp().create_disk_page(disk, index));
                                // New page? How to detect disk was removed?
                            }
                        }

                        for new_page in new_pages {
                            disks_pages.insert(new_page.clone().0, new_page.1);
                        }
                    }
                    Pages::Network(pages) => {
                        let mut new_pages = Vec::new();
                        for (index, network_device) in readings.network_devices.iter().enumerate() {
                            if let Some((summary, page)) =
                                pages.get(&network_device.descriptor.if_name)
                            {
                                let sent = network_device.send_bps;
                                let received = network_device.recv_bps;

                                let graph_widget = summary.graph_widget();
                                if graph_widget.data_points() != data_points {
                                    graph_widget.set_data_points(data_points);
                                }
                                graph_widget.add_data_point(0, sent);
                                graph_widget.add_data_point(1, received);

                                let sent = crate::to_human_readable(sent * 8., 1024.);
                                let received = crate::to_human_readable(received * 8., 1024.);

                                summary.set_info1(i18n_f(
                                    "{}: {} {}bps",
                                    &[
                                        "S",
                                        &format!("{0:.1$}", sent.0, sent.2),
                                        &format!("{}", sent.1),
                                    ],
                                ));
                                summary.set_info2(i18n_f(
                                    "{}: {} {}bps",
                                    &[
                                        "R",
                                        &format!("{0:.1$}", received.0, received.2),
                                        &format!("{}", received.1),
                                    ],
                                ));

                                result &= page.update_readings(network_device);
                            } else {
                                new_pages.push(this.imp().create_network_page(network_device, &index));
                            }
                        }

                        for new_page in new_pages {
                            pages.insert(new_page.clone().0, new_page.1);
                        }
                    }
                    Pages::Gpu(pages) => {
                        for gpu in &readings.gpu_dynamic_info {
                            if let Some((summary, page)) = pages.get(gpu.id.as_ref()) {
                                let graph_widget = summary.graph_widget();
                                if graph_widget.data_points() != data_points {
                                    graph_widget.set_data_points(data_points);
                                }
                                graph_widget.add_data_point(0, gpu.util_percent as f32);
                                summary.set_info2(format!(
                                    "{}% ({} °C)",
                                    gpu.util_percent, gpu.temp_celsius
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
        type ParentType = adw::BreakpointBin;

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

            let this = self.obj().as_ref().clone();

            if let Some(app) = crate::MissionCenterApplication::default_instance() {
                self.settings.set(app.settings());
            }

            this.insert_action_group("graph", Some(unsafe { &*self.action_group.as_ptr() }));

            self.breakpoint.set_condition(Some(
                &adw::BreakpointCondition::parse("max-width: 440sp").unwrap(),
            ));
            self.breakpoint
                .connect_apply(clone!(@weak self as this => move |_| {
                    this.breakpoint_applied.set(true);
                    this.page_content.set_collapsed(true);
                    this.page_content.set_show_sidebar(false);
                }));
            self.breakpoint
                .connect_unapply(clone!(@weak self as this => move |_| {
                    this.breakpoint_applied.set(false);
                    this.page_content.set_collapsed(false);
                    if !this.summary_mode.get() {
                        this.page_content.set_show_sidebar(true);
                    } else {
                        this.page_content.set_show_sidebar(false);
                    }
                }));

            self.page_content
                .sidebar()
                .expect("Infobar is not set")
                .parent()
                .and_then(|p| Some(p.remove_css_class("sidebar-pane")));
            self.page_content.connect_collapsed_notify(
                glib::clone!(@weak self as this => move |pc| {
                    if !pc.is_collapsed() {
                        this.page_content
                            .sidebar()
                            .expect("Infobar is not set")
                            .parent()
                            .and_then(|p| Some(p.remove_css_class("sidebar-pane")));
                    }
                    this.obj().notify_info_button_visible();
                }),
            );

            self.page_content.connect_show_sidebar_notify(
                glib::clone!(@weak self as this => move |_| {
                    this.obj().notify_infobar_visible();
                }),
            );

            if let Some(child) = self.page_stack.visible_child() {
                let infobar_content = child.property::<Option<gtk::Widget>>("infobar-content");
                self.info_bar.set_child(infobar_content.as_ref());
            }
            self.page_stack.connect_visible_child_notify(
                glib::clone!(@weak self as this => move |page_stack| {
                    if let Some(child) = page_stack.visible_child() {
                        let infobar_content = child.property::<Option<gtk::Widget>>("infobar-content");
                        this.info_bar.set_child(infobar_content.as_ref());
                    }
            }));
        }
    }

    impl WidgetImpl for PerformancePage {}

    impl BreakpointBinImpl for PerformancePage {}
}

glib::wrapper! {
    pub struct PerformancePage(ObjectSubclass<imp::PerformancePage>)
        @extends adw::BreakpointBin, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePage {
    pub fn set_initial_readings(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        let ok = imp::PerformancePage::set_up_pages(self, readings);
        imp::PerformancePage::update_readings(self, readings) && ok
    }

    pub fn update_readings(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        imp::PerformancePage::update_readings(self, readings)
    }
}
