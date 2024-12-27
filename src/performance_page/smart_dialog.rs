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
    use std::time::{SystemTime, UNIX_EPOCH};
    use adw::gio::ListStore;
    use adw::glib::WeakRef;
    use adw::ResponseAppearance;
    use gtk::gio;
    use crate::{glib_clone, i18n};
    use crate::performance_page::disk::PerformancePageDisk;
    use crate::performance_page::DiskPage;
    use crate::performance_page::eject_failure_row::{ContentType, EjectFailureRowBuilder, EjectFailureRow};
    use crate::performance_page::sata_smart_dialog_row::{SmartDialogRow, SmartDialogRowBuilder};
    use crate::sys_info_v2::{App, CommonSmartResult, EjectResult, NVMeSmartResult, Process, SataSmartResult};
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::SmartDataDialog)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/disk_smart_data_dialog.ui")]
    pub struct SmartDataDialog {
        #[template_child]
        pub column_view: TemplateChild<gtk::ColumnView>,
        #[template_child]
        pub id_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub attribute_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub value_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub normalized_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub threshold_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub worst_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub type_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub updates_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub assessment_column: TemplateChild<gtk::ColumnViewColumn>,

        #[template_child]
        pub powered_on: TemplateChild<gtk::Label>,
        #[template_child]
        pub status: TemplateChild<gtk::Label>,
        #[template_child]
        pub last_updated: TemplateChild<gtk::Label>,
        #[template_child]
        pub sata_data: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub nvme_data: TemplateChild<gtk::ScrolledWindow>,

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

            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            let last_updated_nice = crate::to_human_readable_time((since_the_epoch.as_millis() / 1000) as u64 - result.last_update_time);
            self.last_updated.set_text(&i18n_f("{} ago", &[&last_updated_nice]));

            // self.status.set_text(result.status.to_string().as_str());
            self.status.set_text(format!("{:?}", result.test_result).as_str());
        }

        // todo populate self
        pub fn apply_sata_smart_result(&self, result: SataSmartResult, parent: &PerformancePageDisk) {
            self.apply_common_smart_result(result.common_smart_result);

            self.sata_data.set_visible(true);
            self.nvme_data.set_visible(false);

            self.parent_page.set(Some(parent.downgrade().upgrade().unwrap()));

            let mut roze = Vec::new();

            for parsed_result in result.blocking_processes {
                let new_row = SmartDialogRowBuilder::new()
                    .id(parsed_result.id)
                    .attribute(parsed_result.name)
                    .value(parsed_result.value, parsed_result.pretty_unit)
                    .threshold(parsed_result.threshold)
                    .pretty(parsed_result.pretty)
                    .worst(parsed_result.worst)
                    .flags(parsed_result.flags)
                    .build();

                roze.push(new_row);
            }

            let rows: gio::ListStore = roze.into_iter().collect();

            let column_view: gtk::ColumnView = self.column_view.get();
            let id_col: gtk::ColumnViewColumn = self.id_column.get();
            let att_col: gtk::ColumnViewColumn = self.attribute_column.get();
            let val_col: gtk::ColumnViewColumn = self.value_column.get();
            let nor_col: gtk::ColumnViewColumn = self.normalized_column.get();
            let thr_col: gtk::ColumnViewColumn = self.threshold_column.get();
            let wor_col: gtk::ColumnViewColumn = self.worst_column.get();
            let typ_col: gtk::ColumnViewColumn = self.type_column.get();
            let upd_col: gtk::ColumnViewColumn = self.updates_column.get();
            let ass_col: gtk::ColumnViewColumn = self.assessment_column.get();

            let factory_id_col = gtk::SignalListItemFactory::new();
            factory_id_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(
                    &gtk::Label::builder()
                        .halign(gtk::Align::Start)
                        .build(),
                ));
            });
            factory_id_col.connect_bind(move |_factory, list_item| {
                let cell = list_item
                    .to_owned()
                    .downcast::<gtk::ColumnViewCell>()
                    .unwrap();
                let child = cell.child().unwrap();
                let label = child.downcast_ref::<gtk::Label>().unwrap();
                let model_item = cell.item().to_owned().unwrap().downcast::<SmartDialogRow>().unwrap();
                label.set_label(&model_item.smart_id().to_string());
            });
            id_col.set_factory(Some(&factory_id_col));

            let factory_att_col = gtk::SignalListItemFactory::new();
            factory_att_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(
                    &gtk::Label::builder()
                        .halign(gtk::Align::Start)
                        .build(),
                ));
            });
            factory_att_col.connect_bind(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                let child = cell.child().unwrap();
                let label = child.downcast::<gtk::Label>().unwrap();
                let model_item = cell.item().to_owned().unwrap().downcast::<SmartDialogRow>().unwrap();
                label.set_label(&model_item.attribute());
            });
            att_col.set_factory(Some(&factory_att_col));

            let factory_val_col = gtk::SignalListItemFactory::new();
            factory_val_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(
                    &gtk::Label::builder()
                        .halign(gtk::Align::Start)
                        .build(),
                ));
            });
            factory_val_col.connect_bind(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                let child = cell.child().unwrap();
                let label = child.downcast::<gtk::Label>().unwrap();
                let model_item = cell.item().to_owned().unwrap().downcast::<SmartDialogRow>().unwrap();
                label.set_label(&model_item.value());
            });
            val_col.set_factory(Some(&factory_val_col));

            let factory_nor_col = gtk::SignalListItemFactory::new();
            factory_nor_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(
                    &gtk::Label::builder()
                        .halign(gtk::Align::Start)
                        .build(),
                ));
            });
            factory_nor_col.connect_bind(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                let child = cell.child().unwrap();
                let label = child.downcast::<gtk::Label>().unwrap();
                let model_item = cell.item().to_owned().unwrap().downcast::<SmartDialogRow>().unwrap();
                label.set_label(&model_item.normalized().to_string());
            });
            nor_col.set_factory(Some(&factory_nor_col));

            let factory_thr_col = gtk::SignalListItemFactory::new();
            factory_thr_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(
                    &gtk::Label::builder()
                        .halign(gtk::Align::Start)
                        .build(),
                ));
            });
            factory_thr_col.connect_bind(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                let child = cell.child().unwrap();
                let label = child.downcast::<gtk::Label>().unwrap();
                let model_item = cell.item().to_owned().unwrap().downcast::<SmartDialogRow>().unwrap();
                label.set_label(&model_item.threshold().to_string());
            });
            thr_col.set_factory(Some(&factory_thr_col));

            let factory_wor_col = gtk::SignalListItemFactory::new();
            factory_wor_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(
                    &gtk::Label::builder()
                        .halign(gtk::Align::Start)
                        .build(),
                ));
            });
            factory_wor_col.connect_bind(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                let child = cell.child().unwrap();
                let label = child.downcast::<gtk::Label>().unwrap();
                let model_item = cell.item().to_owned().unwrap().downcast::<SmartDialogRow>().unwrap();
                label.set_label(&model_item.worst().to_string());
            });
            wor_col.set_factory(Some(&factory_wor_col));

            let factory_typ_col = gtk::SignalListItemFactory::new();
            factory_typ_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(
                    &gtk::Label::builder()
                        .halign(gtk::Align::Start)
                        .build(),
                ));
            });
            factory_typ_col.connect_bind(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                let child = cell.child().unwrap();
                let label = child.downcast::<gtk::Label>().unwrap();
                let model_item = cell.item().to_owned().unwrap().downcast::<SmartDialogRow>().unwrap();
                label.set_label(&model_item.typee());
            });
            typ_col.set_factory(Some(&factory_typ_col));

            let factory_upd_col = gtk::SignalListItemFactory::new();
            factory_upd_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(
                    &gtk::Label::builder()
                        .halign(gtk::Align::Start)
                        .build(),
                ));
            });
            factory_upd_col.connect_bind(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                let child = cell.child().unwrap();
                let label = child.downcast::<gtk::Label>().unwrap();
                let model_item = cell.item().to_owned().unwrap().downcast::<SmartDialogRow>().unwrap();
                label.set_label(&model_item.updates());
            });
            upd_col.set_factory(Some(&factory_upd_col));

            let factory_ass_col = gtk::SignalListItemFactory::new();
            factory_ass_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(
                    &gtk::Label::builder()
                        .halign(gtk::Align::Start)
                        .build(),
                ));
            });
            factory_ass_col.connect_bind(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                let child = cell.child().unwrap();
                let label = child.downcast::<gtk::Label>().unwrap();
                let model_item = cell.item().to_owned().unwrap().downcast::<SmartDialogRow>().unwrap();
                label.set_label(&model_item.assessment());
            });
            ass_col.set_factory(Some(&factory_ass_col));

            let sort_model = gtk::SortListModel::builder()
                .model(&rows)
                .sorter(&column_view.sorter().unwrap())
                .build();

            column_view.set_model(Some(&gtk::SingleSelection::new(Some(sort_model))));
        }

        pub fn apply_nvme_smart_result(&self, result: NVMeSmartResult, parent: &PerformancePageDisk) {
            self.apply_common_smart_result(result.common_smart_result);

            self.sata_data.set_visible(false);
            self.nvme_data.set_visible(true);

            self.parent_page.set(Some(parent.downgrade().upgrade().unwrap()));

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
