/* services_page/details_dialog.rs
 *
 * Copyright 2024 Romeo Calota
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

use std::{cell::Cell, num::NonZeroU32};

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, g_warning, ParamSpec, Properties, SignalHandlerId, Value};

use crate::{app, i18n::*};

mod imp {
    use std::cell::OnceCell;
    use std::collections::HashMap;
    use std::sync::Arc;
    use adw::gio::ListStore;
    use adw::glib::WeakRef;
    use adw::ResponseAppearance;
    use crate::{glib_clone, i18n};
    use crate::performance_page::disk::PerformancePageDisk;
    use crate::performance_page::DiskPage;
    use crate::performance_page::eject_failure_row::{ContentType, EjectFailureRowBuilder, EjectFailureRow};
    use crate::performance_page::sata_smart_dialog_row::SmartDialogRowBuilder;
    use crate::sys_info_v2::{App, CommonSmartResult, EjectResult, NVMeSmartResult, Process, SataSmartResult};
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::SmartDataDialog)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/disk_smart_data_dialog.ui")]
    pub struct SmartDataDialog {
        #[template_child]
        pub column_view: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub powered_on: TemplateChild<gtk::Label>,
        #[template_child]
        pub status: TemplateChild<gtk::Label>,
        #[template_child]
        pub sata_data: TemplateChild<gtk::ScrolledWindow>,

        #[template_child]
        pub avail_spare: TemplateChild<gtk::Label>,
        #[template_child]
        pub spare_thresh: TemplateChild<gtk::Label>,
        #[template_child]
        pub percent_used: TemplateChild<gtk::Label>,
        #[template_child]
        pub data_read: TemplateChild<gtk::Label>,
        #[template_child]
        pub data_written: TemplateChild<gtk::Label>,
        #[template_child]
        pub ctrl_busy_minutes: TemplateChild<gtk::Label>,
        #[template_child]
        pub power_cycles: TemplateChild<gtk::Label>,
        #[template_child]
        pub unsafe_shutdowns: TemplateChild<gtk::Label>,
        #[template_child]
        pub media_errors: TemplateChild<gtk::Label>,
        #[template_child]
        pub num_err_log_entries: TemplateChild<gtk::Label>,
        #[template_child]
        pub temp_sensors: TemplateChild<gtk::Label>,
        #[template_child]
        pub wctemp: TemplateChild<gtk::Label>,
        #[template_child]
        pub cctemp: TemplateChild<gtk::Label>,
        #[template_child]
        pub warning_temp_time: TemplateChild<gtk::Label>,
        #[template_child]
        pub critical_temp_time: TemplateChild<gtk::Label>,


        pub parent_page: Cell<Option<PerformancePageDisk>>,
    }

    impl SmartDataDialog {
        fn apply_common_smart_result(&self, result: CommonSmartResult) {
            let powered_on_nice = crate::to_human_readable_time(result.powered_on_seconds);
            self.powered_on.set_text(powered_on_nice.as_str());
            // self.status.set_text(result.status.to_string().as_str());
            self.status.set_text(format!("{:?}", result.test_result).as_str());
            self.
        }

        // todo populate self
        pub fn apply_sata_smart_result(&self, result: SataSmartResult, parent: &PerformancePageDisk) {
            self.apply_common_smart_result(result.common_smart_result);

            self.sata_data.set_visible(true);

            let modelo = self.column_view.get();

            self.parent_page.set(Some(parent.downgrade().upgrade().unwrap()));

            modelo.remove_all();

            for parsed_result in result.blocking_processes {
                let new_row = SmartDialogRowBuilder::new()
                    .id(parsed_result.id)
                    .attribute(parsed_result.name.as_str())
                    .value(parsed_result.value, parsed_result.pretty_unit)
                    .threshold(parsed_result.threshold)
                    .pretty(parsed_result.pretty)
                    .worst(parsed_result.worst)
                    .build();

                modelo.append(
                    new_row.imp().row_entry.get().expect("Missing row entry")
                )
            }
        }

        pub fn apply_nvme_smart_result(&self, result: NVMeSmartResult, parent: &PerformancePageDisk) {
            self.apply_common_smart_result(result.common_smart_result);

            self.sata_data.set_visible(false);

            let modelo = self.column_view.get();

            self.parent_page.set(Some(parent.downgrade().upgrade().unwrap()));

            modelo.remove_all();

            self.percent_used.set_text(&format!("{}%", result.percent_used));

            self.avail_spare.set_text(result.avail_spare.to_string().as_str());
            self.spare_thresh.set_text(result.spare_thresh.to_string().as_str());

            let used_memory = crate::to_human_readable(result.total_data_read as f32, 1024.);
            self.data_read.set_text(&format!(
                "{0:.2$} {1}{3}B",
                used_memory.0,
                used_memory.1,
                used_memory.2,
                if used_memory.1.is_empty() { "" } else { "i" },
            ));
            let used_memory = crate::to_human_readable(result.total_data_written as f32, 1024.);
            self.data_written.set_text(&format!(
                "{0:.2$} {1}{3}B",
                used_memory.0,
                used_memory.1,
                used_memory.2,
                if used_memory.1.is_empty() { "" } else { "i" },
            ));

            self.warning_temp_time.set_text(crate::to_human_readable_time(result.warning_temp_time as u64).as_str());
            self.critical_temp_time.set_text(crate::to_human_readable_time(result.critical_temp_time as u64).as_str());
            self.ctrl_busy_minutes.set_text(crate::to_human_readable_time(result.ctrl_busy_minutes * 60).as_str());

            self.wctemp.set_text(&i18n_f("{} °C", &[&format!("{}", result.wctemp - 273)]));
            self.cctemp.set_text(&i18n_f("{} °C", &[&format!("{}", result.cctemp - 273)]));

            self.temp_sensors.set_text(&format!("{:?}", result.temp_sensors));
            self.unsafe_shutdowns.set_text(result.unsafe_shutdowns.to_string().as_str());
            self.media_errors.set_text(result.media_errors.to_string().as_str());
            self.num_err_log_entries.set_text(result.num_err_log_entries.to_string().as_str());
            self.power_cycles.set_text(result.power_cycles.to_string().as_str());

            self.ctrl_busy_minutes.set_text(result.ctrl_busy_minutes.to_string().as_str());
        }
    }

    impl Default for SmartDataDialog {
        fn default() -> Self {
            Self {
                column_view: Default::default(),
                powered_on: Default::default(),
                status: Default::default(),
                sata_data: Default::default(),
                avail_spare: Default::default(),
                spare_thresh: Default::default(),
                percent_used: Default::default(),
                data_read: Default::default(),
                data_written: Default::default(),
                ctrl_busy_minutes: Default::default(),
                power_cycles: Default::default(),
                unsafe_shutdowns: Default::default(),
                media_errors: Default::default(),
                num_err_log_entries: Default::default(),
                temp_sensors: Default::default(),
                wctemp: Default::default(),
                cctemp: Default::default(),
                warning_temp_time: Default::default(),
                critical_temp_time: Default::default(),
                parent_page: Cell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SmartDataDialog {
        const NAME: &'static str = "SmartDataDialog";
        type Type = super::SmartDataDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SmartDataDialog {
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for SmartDataDialog {
        fn realize(&self) {
            self.parent_realize();

            // todo init here
        }
    }

    impl AdwDialogImpl for SmartDataDialog {
        fn closed(&self) {

        }
    }
}

glib::wrapper! {
    pub struct SmartDataDialog(ObjectSubclass<imp::SmartDataDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

fn to_signal_id(id: u64) -> SignalHandlerId {
    unsafe { std::mem::transmute(id) }
}

fn from_signal_id(id: SignalHandlerId) -> u64 {
    unsafe { id.as_raw() }
}
