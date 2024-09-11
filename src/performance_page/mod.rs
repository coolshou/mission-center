/* performance_page/view_models
 *
 * Copyright 2024 Romeo Calota
 * Copyright 2024 jojo2357
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
use glib::{ParamSpec, Properties, Value};
use gtk::{gio, glib};

use widgets::GraphWidget;

use crate::{application::BASE_POINTS, i18n::*, sys_info_v2::DiskType, sys_info_v2::NetworkDevice};

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

    // GNOME color palette: Blue 2
    const MEMORY_BASE_COLOR: [u8; 3] = [0x62, 0xa0, 0xea];
    // GNOME color palette: Green 5
    const DISK_BASE_COLOR: [u8; 3] = [0x26, 0xa2, 0x69];
    // GNOME color palette: Purple 1
    const NETWORK_BASE_COLOR: [u8; 3] = [0xdc, 0x8a, 0xdd];

    const SIDEBAR_CPU_PAGE_DEFAULT_IDX: usize = 1;
    const SIDEBAR_MEM_PAGE_DEFAULT_IDX: usize = 2;
    const SIDEBAR_DISK_PAGE_DEFAULT_IDX: usize = 3;
    const SIDEBAR_NET_PAGE_DEFAULT_IDX: usize = 4;
    const SIDEBAR_GPU_PAGE_DEFAULT_IDX: usize = 5;

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
                        prev_action.set_state(&glib::Variant::from(false));
                        new_action.set_state(&glib::Variant::from(true));
                    }

                    imp.context_menu_view_actions.set(actions);
                    imp.page_stack.set_visible_child_name(page_name);

                    if let Some(settings) = imp.settings.take() {
                        println!("Set page {}", page_name);
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
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };

                    let new_state = !this.summary_mode();
                    action.set_state(&glib::Variant::from(new_state));
                    this.set_summary_mode(new_state);
                    if !this.imp().breakpoint_applied.get() {
                        this.imp().page_content.set_show_sidebar(!new_state);
                    }
                }
            });
            actions.add_action(&action);

            let action = gio::SimpleAction::new_stateful("cpu", None, &glib::Variant::from(true));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let row = this
                        .sidebar()
                        .row_at_index(0)
                        .expect("Failed to select CPU row");
                    this.sidebar().select_row(Some(&row));

                    let prev_action = this.current_view_action.replace(action.clone());
                    prev_action.set_state(&glib::Variant::from(false));
                    action.set_state(&glib::Variant::from(true));
                }
            });
            actions.add_action(&action);
            view_actions.insert("cpu".to_string(), action.clone());
            this.imp().current_view_action.set(action);

            let action =
                gio::SimpleAction::new_stateful("memory", None, &glib::Variant::from(false));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let row = this
                        .sidebar()
                        .row_at_index(1)
                        .expect("Failed to select Memory row");
                    this.sidebar().select_row(Some(&row));

                    let prev_action = this.current_view_action.replace(action.clone());
                    prev_action.set_state(&glib::Variant::from(false));
                    action.set_state(&glib::Variant::from(true));
                }
            });
            actions.add_action(&action);
            view_actions.insert("memory".to_string(), action);

            let action = gio::SimpleAction::new_stateful("disk", None, &glib::Variant::from(false));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };

                    let pages = this.imp().pages.take();
                    for page in &pages {
                        let disk_pages = match page {
                            Pages::Disk(disk_pages) => disk_pages,
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

                        this.imp()
                            .sidebar()
                            .select_row(row.downcast_ref::<gtk::ListBoxRow>());

                        let prev_action = this.imp().current_view_action.replace(action.clone());
                        prev_action.set_state(&glib::Variant::from(false));
                        action.set_state(&glib::Variant::from(true));

                        break;
                    }
                    this.imp().pages.set(pages);
                }
            });
            actions.add_action(&action);
            view_actions.insert("disk".to_string(), action);

            let action =
                gio::SimpleAction::new_stateful("network", None, &glib::Variant::from(false));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };

                    let pages = this.imp().pages.take();
                    for page in &pages {
                        let network_pages = match page {
                            Pages::Network(network_pages) => network_pages,
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

                        this.imp()
                            .sidebar()
                            .select_row(row.downcast_ref::<gtk::ListBoxRow>());

                        let prev_action = this.imp().current_view_action.replace(action.clone());
                        prev_action.set_state(&glib::Variant::from(false));
                        action.set_state(&glib::Variant::from(true));

                        break;
                    }
                    this.imp().pages.set(pages);
                }
            });
            actions.add_action(&action);
            view_actions.insert("network".to_string(), action);

            let action = gio::SimpleAction::new_stateful("gpu", None, &glib::Variant::from(false));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };

                    let pages = this.imp().pages.take();
                    for page in &pages {
                        let gpu_pages = match page {
                            Pages::Gpu(gpu_pages) => gpu_pages,
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

                        this.imp()
                            .sidebar()
                            .select_row(row.downcast_ref::<gtk::ListBoxRow>());

                        let prev_action = this.imp().current_view_action.replace(action.clone());
                        prev_action.set_state(&glib::Variant::from(false));
                        action.set_state(&glib::Variant::from(true));

                        break;
                    }
                    this.imp().pages.set(pages);
                }
            });
            actions.add_action(&action);
            view_actions.insert("gpu".to_string(), action);

            this.imp().context_menu_view_actions.set(view_actions);
        }

        fn set_up_cpu_page(&self, pages: &mut Vec<Pages>, readings: &crate::sys_info_v2::Readings) {
            // GNOME color palette: Blue 4
            const BASE_COLOR: [u8; 3] = [0x1c, 0x71, 0xd8];

            let summary = SummaryGraph::new();
            summary.set_page_indices(SIDEBAR_CPU_PAGE_DEFAULT_IDX, 0);
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
            summary
                .graph_widget()
                .set_data_points(unwrapped_settings.int("perfomance-page-data-points") as u32);
            summary
                .graph_widget()
                .set_smooth_graphs(unwrapped_settings.boolean("performance-smooth-graphs"));

            let page = CpuPage::new(unwrapped_settings);
            page.set_base_color(gtk::gdk::RGBA::new(
                BASE_COLOR[0] as f32 / 255.,
                BASE_COLOR[1] as f32 / 255.,
                BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            self.settings.set(settings);
            page.set_static_information(readings);

            self.page_content.connect_collapsed_notify({
                let page = page.downgrade();
                move |pc| {
                    if let Some(page) = page.upgrade() {
                        if pc.is_collapsed() {
                            page.infobar_collapsed();
                        } else {
                            page.infobar_uncollapsed();
                        }
                    }
                }
            });

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
            summary.set_page_indices(SIDEBAR_MEM_PAGE_DEFAULT_IDX, 0);
            summary.set_widget_name("memory");

            {
                let graph_widget = summary.graph_widget();

                graph_widget.set_value_range_max(readings.mem_info.mem_total as f32);
                graph_widget.set_data_set_count(2);
                graph_widget.set_filled(0, false);
                graph_widget.set_dashed(0, true);
            }

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

            summary.graph_widget().set_data_points(
                settings
                    .as_ref()
                    .unwrap()
                    .int("perfomance-page-data-points") as u32,
            );

            summary.graph_widget().set_smooth_graphs(
                settings
                    .as_ref()
                    .unwrap()
                    .boolean("performance-smooth-graphs"),
            );

            let page = MemoryPage::new(settings.as_ref().unwrap());
            page.set_base_color(gtk::gdk::RGBA::new(
                MEMORY_BASE_COLOR[0] as f32 / 255.,
                MEMORY_BASE_COLOR[1] as f32 / 255.,
                MEMORY_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            page.set_memory_color(gtk::gdk::RGBA::new(
                DISK_BASE_COLOR[0] as f32 / 255.,
                DISK_BASE_COLOR[1] as f32 / 255.,
                DISK_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            self.settings.set(settings);
            page.set_static_information(readings);

            self.page_content.connect_collapsed_notify({
                let page = page.downgrade();
                move |pc| {
                    if let Some(page) = page.upgrade() {
                        if pc.is_collapsed() {
                            page.infobar_collapsed();
                        } else {
                            page.infobar_uncollapsed();
                        }
                    }
                }
            });

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
            let len = readings.disks_info.len();
            let hide_index = len == 1;
            for i in 0..len {
                let mut ret =
                    self.create_disk_page(readings, if hide_index { None } else { Some(i) });
                disks.insert(std::mem::take(&mut ret.0), ret.1);
            }

            pages.push(Pages::Disk(disks));
        }

        pub fn update_disk_page_index(
            &self,
            disk_graph: &SummaryGraph,
            disk_id: &str,
            index: Option<usize>,
        ) {
            disk_graph.set_page_secondary_index(index.unwrap_or(0));

            if index.is_some() {
                disk_graph.set_heading(i18n_f(
                    "Drive {} ({})",
                    &[&format!("{}", index.unwrap()), &format!("{}", disk_id)],
                ));
            } else {
                disk_graph.set_heading(i18n_f("Drive", &[]));
            }
        }

        pub fn create_disk_page(
            &self,
            readings: &crate::sys_info_v2::Readings,
            secondary_index: Option<usize>,
        ) -> (String, (summary_graph::SummaryGraph, DiskPage)) {
            use glib::g_critical;

            let disk_static_info = &readings.disks_info[secondary_index.unwrap_or(0)];

            let summary = SummaryGraph::new();
            summary.set_page_indices(SIDEBAR_DISK_PAGE_DEFAULT_IDX, secondary_index.unwrap_or(0));
            summary.set_widget_name(disk_static_info.id.as_ref());

            self.update_disk_page_index(&summary, disk_static_info.id.as_ref(), secondary_index);
            summary.set_info1(match disk_static_info.r#type {
                DiskType::HDD => i18n("HDD"),
                DiskType::SSD => i18n("SSD"),
                DiskType::NVMe => i18n("NVMe"),
                DiskType::eMMC => i18n("eMMC"),
                DiskType::iSCSI => i18n("iSCSI"),
                DiskType::Optical => i18n("Optical"),
                DiskType::Unknown => i18n("Unknown"),
            });
            summary.set_info2(format!("{:.0}%", disk_static_info.busy_percent));
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

            summary.graph_widget().set_data_points(
                settings
                    .as_ref()
                    .unwrap()
                    .int("perfomance-page-data-points") as u32,
            );

            summary.graph_widget().set_smooth_graphs(
                settings
                    .as_ref()
                    .unwrap()
                    .boolean("performance-smooth-graphs"),
            );

            let page = DiskPage::new(disk_static_info.id.as_ref(), settings.as_ref().unwrap());
            page.set_base_color(gtk::gdk::RGBA::new(
                DISK_BASE_COLOR[0] as f32 / 255.,
                DISK_BASE_COLOR[1] as f32 / 255.,
                DISK_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            self.settings.set(settings);
            page.set_static_information(secondary_index, disk_static_info);

            self.page_content.connect_collapsed_notify({
                let page = page.downgrade();
                move |pc| {
                    if let Some(page) = page.upgrade() {
                        if pc.is_collapsed() {
                            page.infobar_collapsed();
                        } else {
                            page.infobar_uncollapsed();
                        }
                    }
                }
            });

            self.obj()
                .as_ref()
                .bind_property("summary-mode", &page, "summary-mode")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();

            self.sidebar().append(&summary);
            self.page_stack.add_named(&page, Some(&disk_static_info.id));

            let mut actions = self.context_menu_view_actions.take();
            match actions.get("disk") {
                None => {
                    g_critical!(
                        "MissionCenter::PerformancePage",
                        "Failed to wire up disk action for {}, logic bug?",
                        &disk_static_info.id
                    );
                }
                Some(action) => {
                    actions.insert(disk_static_info.id.to_string(), action.clone());
                }
            }
            self.context_menu_view_actions.set(actions);

            return (disk_static_info.id.as_ref().to_owned(), (summary, page));
        }

        fn set_up_network_pages(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            let mut networks = HashMap::new();
            for (i, network_device) in readings.network_devices.iter().enumerate() {
                let mut ret = self.create_network_page(network_device, i);
                networks.insert(std::mem::take(&mut ret.0), ret.1);
            }

            pages.push(Pages::Network(networks));
        }

        fn create_network_page(
            &self,
            network_device: &NetworkDevice,
            secondary_index: usize,
        ) -> (String, (summary_graph::SummaryGraph, NetworkPage)) {
            use glib::g_critical;

            let if_name = network_device.descriptor.if_name.as_str();

            let conn_type = network_device.descriptor.kind.to_string();

            let summary = SummaryGraph::new();
            summary.set_page_indices(SIDEBAR_NET_PAGE_DEFAULT_IDX, secondary_index);
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

            summary.graph_widget().set_data_points(
                settings
                    .as_ref()
                    .unwrap()
                    .int("perfomance-page-data-points") as u32,
            );

            summary.graph_widget().set_smooth_graphs(
                settings
                    .as_ref()
                    .unwrap()
                    .boolean("performance-smooth-graphs"),
            );

            let page = NetworkPage::new(
                if_name,
                network_device.descriptor.kind,
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

            self.page_content.connect_collapsed_notify({
                let page = page.downgrade();
                move |pc| {
                    if let Some(page) = page.upgrade() {
                        if pc.is_collapsed() {
                            page.infobar_collapsed();
                        } else {
                            page.infobar_uncollapsed();
                        }
                    }
                }
            });

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

            let hide_index = readings.gpu_static_info.len() == 1;
            for (index, static_info) in readings.gpu_static_info.iter().enumerate() {
                let dynamic_info = &readings.gpu_dynamic_info[index];

                let summary = SummaryGraph::new();
                summary.set_page_indices(SIDEBAR_GPU_PAGE_DEFAULT_IDX, index);
                summary.set_widget_name(static_info.id.as_ref());

                let settings = self.settings.take();
                if settings.is_none() {
                    panic!("Settings not set");
                }

                summary.graph_widget().set_data_points(
                    settings
                        .as_ref()
                        .unwrap()
                        .int("perfomance-page-data-points") as u32,
                );

                summary.graph_widget().set_smooth_graphs(
                    settings
                        .as_ref()
                        .unwrap()
                        .boolean("performance-smooth-graphs"),
                );

                let page = GpuPage::new(&static_info.device_name, settings.as_ref().unwrap());

                self.settings.set(settings);

                if !hide_index {
                    summary.set_heading(i18n_f("GPU {}", &[&format!("{}", index)]));
                } else {
                    summary.set_heading(i18n_f("GPU", &[]));
                }
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

                page.set_base_color(gtk::gdk::RGBA::new(
                    BASE_COLOR[0] as f32 / 255.,
                    BASE_COLOR[1] as f32 / 255.,
                    BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));
                page.set_static_information(
                    if !hide_index { Some(index) } else { None },
                    static_info,
                );

                self.page_content.connect_collapsed_notify({
                    let page = page.downgrade();
                    move |pc| {
                        if let Some(page) = page.upgrade() {
                            if pc.is_collapsed() {
                                page.infobar_collapsed();
                            } else {
                                page.infobar_uncollapsed();
                            }
                        }
                    }
                });

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

            this.sidebar().set_sort_func(|g1, g2| {
                use gtk::Ordering;

                let g1 = match g1
                    .child()
                    .and_then(|g1| g1.downcast_ref::<SummaryGraph>().cloned())
                {
                    None => return Ordering::Equal,
                    Some(g1) => g1,
                };

                let g2 = match g2
                    .child()
                    .and_then(|g2| g2.downcast_ref::<SummaryGraph>().cloned())
                {
                    None => return Ordering::Equal,
                    Some(g2) => g2,
                };

                g1.cmp(&g2)
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
            use glib::g_warning;

            let mut pages = this.imp().pages.take();

            let mut pages_to_destroy = Vec::new();

            for page in &mut pages {
                match page {
                    Pages::Cpu(_) => {}    // not dynamic
                    Pages::Memory(_) => {} // not dynamic
                    Pages::Disk(ref mut disks_pages) => {
                        for disk_name in disks_pages.keys() {
                            if !readings
                                .disks_info
                                .iter()
                                .any(|device| device.id.as_ref() == disk_name)
                            {
                                pages_to_destroy.push(disk_name.clone());
                            }
                        }

                        for disk_name in &pages_to_destroy {
                            if let Some((graph, page)) =
                                disks_pages.get(disk_name).and_then(|v| Some(v.clone()))
                            {
                                let parent = match graph.parent() {
                                    Some(parent) => parent,
                                    None => {
                                        g_warning!(
                                            "MissionCenter::PerformancePage",
                                            "Failed to get parent of graph widget"
                                        );
                                        continue;
                                    }
                                };
                                this.sidebar().remove(&parent);
                                this.imp().page_stack.remove(&page);
                                disks_pages.remove(disk_name);
                            }
                        }

                        pages_to_destroy.clear();
                    }
                    Pages::Network(net_pages) => {
                        for network_id in net_pages.keys() {
                            if !readings
                                .network_devices
                                .iter()
                                .any(|device| &device.descriptor.if_name == network_id)
                            {
                                pages_to_destroy.push(network_id.clone());
                            }
                        }

                        for net_device_name in &pages_to_destroy {
                            if let Some((graph, page)) =
                                net_pages.get(net_device_name).and_then(|v| Some(v.clone()))
                            {
                                let parent = match graph.parent() {
                                    Some(parent) => parent,
                                    None => {
                                        g_warning!(
                                            "MissionCenter::PerformancePage",
                                            "Failed to get parent of graph widget"
                                        );
                                        continue;
                                    }
                                };
                                this.sidebar().remove(&parent);
                                this.imp().page_stack.remove(&page);
                                net_pages.remove(net_device_name);
                            }
                        }

                        pages_to_destroy.clear();
                    }
                    Pages::Gpu(_) => {}
                }
            }

            let mut result = true;

            let mut data_points = BASE_POINTS;
            let mut smooth = false;
            let settings = this.imp().settings.take();
            if !settings.is_none() {
                data_points = settings.clone().unwrap().int("perfomance-page-data-points") as u32;
                smooth = settings
                    .clone()
                    .unwrap()
                    .boolean("performance-smooth-graphs");
            }
            this.imp().settings.set(settings);

            for page in &mut pages {
                match page {
                    Pages::Cpu((summary, page)) => {
                        let graph_widget = summary.graph_widget();
                        graph_widget.set_data_points(data_points);
                        graph_widget.set_smooth_graphs(smooth);

                        graph_widget.add_data_point(
                            0,
                            readings.cpu_dynamic_info.overall_utilization_percent,
                        );
                        summary.set_info1(format!(
                            "{}% {:.2} GHz",
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
                        let used_raw = readings.mem_info.mem_total
                            - (readings.mem_info.mem_available + readings.mem_info.dirty);
                        let graph_widget = summary.graph_widget();
                        graph_widget.set_data_points(data_points);
                        graph_widget.set_smooth_graphs(smooth);
                        graph_widget.add_data_point(0, readings.mem_info.committed as _);
                        graph_widget.add_data_point(1, used_raw as _);
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
                    Pages::Disk(pages) => {
                        let mut new_devices = Vec::new();
                        let hide_index = readings.disks_info.len() == 1;
                        for (index, disk) in readings.disks_info.iter().enumerate() {
                            if let Some((summary, page)) = pages.get(disk.id.as_ref()) {
                                this.imp().update_disk_page_index(
                                    summary,
                                    disk.id.as_ref(),
                                    if hide_index { None } else { Some(index) },
                                );

                                let graph_widget = summary.graph_widget();
                                graph_widget.set_data_points(data_points);
                                graph_widget.set_smooth_graphs(smooth);
                                graph_widget.add_data_point(0, disk.busy_percent);
                                summary.set_info2(format!("{:.0}%", disk.busy_percent));

                                result &= page.update_readings(
                                    if hide_index { None } else { Some(index) },
                                    disk,
                                );
                            } else {
                                new_devices.push(index);
                            }
                        }

                        for new_device_index in new_devices {
                            let (disk_id, page) = this.imp().create_disk_page(
                                readings,
                                if hide_index {
                                    None
                                } else {
                                    Some(new_device_index)
                                },
                            );
                            pages.insert(disk_id, page);
                        }
                    }
                    Pages::Network(pages) => {
                        let mut new_devices = Vec::new();
                        for (index, network_device) in readings.network_devices.iter().enumerate() {
                            if let Some((summary, page)) =
                                pages.get(&network_device.descriptor.if_name)
                            {
                                summary.set_page_secondary_index(index);

                                let data_per_time = page.unit_per_second_label();
                                let byte_coeff = page.byte_conversion_factor();

                                let send_speed = network_device.send_bps * byte_coeff;
                                let rec_speed = network_device.recv_bps * byte_coeff;

                                let graph_widget = summary.graph_widget();
                                graph_widget.set_data_points(data_points);
                                graph_widget.set_smooth_graphs(smooth);
                                graph_widget.add_data_point(0, network_device.send_bps);
                                graph_widget.add_data_point(1, network_device.recv_bps);

                                let sent_speed = crate::to_human_readable(send_speed, 1024.);
                                let rect_speeed = crate::to_human_readable(rec_speed, 1024.);

                                summary.set_info1(i18n_f(
                                    "{}: {} {}{}",
                                    &[
                                        "S",
                                        &format!("{0:.1$}", sent_speed.0, sent_speed.2),
                                        &format!("{}", sent_speed.1),
                                        &*data_per_time,
                                    ],
                                ));
                                summary.set_info2(i18n_f(
                                    "{}: {} {}{}",
                                    &[
                                        "R",
                                        &format!("{0:.1$}", rect_speeed.0, rect_speeed.2),
                                        &format!("{}", rect_speeed.1),
                                        &*data_per_time,
                                    ],
                                ));

                                result &= page.update_readings(network_device);
                            } else {
                                new_devices.push(index);
                            }
                        }

                        for new_device_index in new_devices {
                            let (net_if_id, page) = this.imp().create_network_page(
                                &readings.network_devices[new_device_index],
                                new_device_index,
                            );
                            pages.insert(net_if_id, page);
                        }
                    }
                    Pages::Gpu(pages) => {
                        for gpu in &readings.gpu_dynamic_info {
                            if let Some((summary, page)) = pages.get(gpu.id.as_ref()) {
                                let graph_widget = summary.graph_widget();
                                graph_widget.set_data_points(data_points);
                                graph_widget.set_smooth_graphs(smooth);
                                graph_widget.add_data_point(0, gpu.util_percent as f32);
                                if gpu.temp_celsius > 20 {
                                    summary.set_info2(format!(
                                        "{}% ({} °C)",
                                        gpu.util_percent, gpu.temp_celsius
                                    ));
                                } else {
                                    summary.set_info2(format!("{}%", gpu.util_percent));
                                }
                                let id = gpu.id.clone();
                                let mut gpu_static = None;
                                for gpu_stat in &readings.gpu_static_info {
                                    if id == gpu_stat.id {
                                        gpu_static = Some(gpu_stat);
                                        break;
                                    }
                                }

                                result &= page.update_readings(gpu, gpu_static.unwrap());
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
            self.breakpoint.connect_apply({
                let this = self.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    this.breakpoint_applied.set(true);
                    this.page_content.set_collapsed(true);
                    this.page_content.set_show_sidebar(false);
                }
            });
            self.breakpoint.connect_unapply({
                let this = self.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    this.breakpoint_applied.set(false);
                    this.page_content.set_collapsed(false);
                    if !this.summary_mode.get() {
                        this.page_content.set_show_sidebar(true);
                    } else {
                        this.page_content.set_show_sidebar(false);
                    }
                }
            });

            self.page_content
                .sidebar()
                .expect("Infobar is not set")
                .parent()
                .and_then(|p| Some(p.remove_css_class("sidebar-pane")));
            self.page_content.connect_collapsed_notify({
                let this = self.obj().downgrade();
                move |pc| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    if !pc.is_collapsed() {
                        this.page_content
                            .sidebar()
                            .expect("Infobar is not set")
                            .parent()
                            .and_then(|p| Some(p.remove_css_class("sidebar-pane")));

                        this.info_bar.set_halign(gtk::Align::Fill);
                    } else {
                        this.info_bar.set_halign(gtk::Align::Center);
                    }
                    this.obj().notify_info_button_visible();
                }
            });

            self.page_content.connect_show_sidebar_notify({
                let this = self.obj().downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        this.notify_infobar_visible();
                    }
                }
            });

            if let Some(child) = self.page_stack.visible_child() {
                let infobar_content = child.property::<Option<gtk::Widget>>("infobar-content");
                self.info_bar.set_child(infobar_content.as_ref());
            }
            self.page_stack.connect_visible_child_notify({
                let this = self.obj().downgrade();
                move |page_stack| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };

                    if let Some(child) = page_stack.visible_child() {
                        let infobar_content =
                            child.property::<Option<gtk::Widget>>("infobar-content");
                        this.imp().info_bar.set_child(infobar_content.as_ref());
                    }
                }
            });
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
