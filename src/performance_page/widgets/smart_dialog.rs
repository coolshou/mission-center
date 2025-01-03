/* services_page/details_dialog.rs
 *
 * Copyright 2024 Mission Center Devs
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

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, ParamSpec, Properties, Value};

use crate::i18n::*;
use crate::performance_page::disk::PerformancePageDisk;
use crate::performance_page::widgets::sata_smart_dialog_row::SmartDialogRow;
use crate::sys_info_v2::{CommonSmartResult, NVMeSmartResult, SataSmartResult};
use gtk::gio;
use std::time::{SystemTime, UNIX_EPOCH};

mod imp {
    use dbus::arg::RefArg;
    use gtk::{Align, ColumnViewColumn};
    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::SmartDataDialog)]
    #[derive(gtk::CompositeTemplate)]
    #[template(
        resource = "/io/missioncenter/MissionCenter/ui/performance_page/disk_smart_data_dialog.ui"
    )]
    pub struct SmartDataDialog {
        #[template_child]
        pub column_view: TemplateChild<gtk::ColumnView>,
        #[template_child]
        pub id_column: TemplateChild<ColumnViewColumn>,
        #[template_child]
        pub attribute_column: TemplateChild<ColumnViewColumn>,
        #[template_child]
        pub value_column: TemplateChild<ColumnViewColumn>,
        #[template_child]
        pub normalized_column: TemplateChild<ColumnViewColumn>,
        #[template_child]
        pub threshold_column: TemplateChild<ColumnViewColumn>,
        #[template_child]
        pub worst_column: TemplateChild<ColumnViewColumn>,
        #[template_child]
        pub type_column: TemplateChild<ColumnViewColumn>,
        #[template_child]
        pub updates_column: TemplateChild<ColumnViewColumn>,
        #[template_child]
        pub assessment_column: TemplateChild<ColumnViewColumn>,

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
            let last_updated_nice = crate::to_human_readable_time(
                (since_the_epoch.as_millis() / 1000) as u64 - result.last_update_time,
            );
            self.last_updated
                .set_text(&i18n_f("{} ago", &[&last_updated_nice]));

            self.status
                .set_text(format!("{:?}", result.test_result).as_str());
        }

        pub fn apply_sata_smart_result(
            &self,
            result: SataSmartResult,
            parent: &PerformancePageDisk,
        ) {
            self.apply_common_smart_result(result.common_smart_result);

            self.sata_data.set_visible(true);
            self.nvme_data.set_visible(false);

            self.parent_page
                .set(Some(parent.downgrade().upgrade().unwrap()));

            let mut roze = Vec::new();

            for parsed_result in result.blocking_processes {
                let new_row = SmartDialogRow::new(
                    parsed_result.id,
                    parsed_result.name,
                    parsed_result.value,
                    parsed_result.pretty,
                    parsed_result.pretty_unit,
                    parsed_result.threshold,
                    parsed_result.worst,
                    &match parsed_result.flags & 0b1 {
                        1 => i18n("Pre-Fail"),
                        _ => i18n("Old-Age"),
                    },
                    &match parsed_result.flags & 0b10 >> 1 {
                        0 => i18n("Online"),
                        _ => i18n("Offline"),
                    },
                    "UNKNOWN",
                );

                roze.push(new_row);
            }

            let rows: gio::ListStore = roze.into_iter().collect();

            let column_view: gtk::ColumnView = self.column_view.get();
            let id_col: ColumnViewColumn = self.id_column.get();
            let att_col: ColumnViewColumn = self.attribute_column.get();
            let val_col: ColumnViewColumn = self.value_column.get();
            let nor_col: ColumnViewColumn = self.normalized_column.get();
            let thr_col: ColumnViewColumn = self.threshold_column.get();
            let wor_col: ColumnViewColumn = self.worst_column.get();
            let typ_col: ColumnViewColumn = self.type_column.get();
            let upd_col: ColumnViewColumn = self.updates_column.get();
            let ass_col: ColumnViewColumn = self.assessment_column.get();

            Self::setup_column_factory(id_col, Align::Start, |mi| mi.smart_id().to_string());
            Self::setup_column_factory(att_col, Align::Start, |mi| mi.attribute().to_string());
            Self::setup_column_factory(val_col, Align::Start, |mi| mi.value().to_string());
            Self::setup_column_factory(nor_col, Align::Start, |mi| mi.normalized().to_string());
            Self::setup_column_factory(thr_col, Align::Start, |mi| mi.threshold().to_string());
            Self::setup_column_factory(wor_col, Align::Start, |mi| mi.worst().to_string());
            Self::setup_column_factory(typ_col, Align::Start, |mi| mi.typee().to_string());
            Self::setup_column_factory(upd_col, Align::Start, |mi| mi.updates().to_string());
            Self::setup_column_factory(ass_col, Align::Start, |mi| mi.assessment().to_string());

            let sort_model = gtk::SortListModel::builder()
                .model(&rows)
                .sorter(&column_view.sorter().unwrap())
                .build();

            column_view.set_model(Some(&gtk::SingleSelection::new(Some(sort_model))));
        }

        fn setup_column_factory<'a, E>(id_col: ColumnViewColumn, alignment: Align, extract: E) where
            E: Fn(SmartDialogRow) -> String + 'static,
        {
            let factory_id_col = gtk::SignalListItemFactory::new();
            factory_id_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(
                    &gtk::Label::builder().halign(alignment).build(),
                ));
            });
            factory_id_col.connect_bind(move |_factory, list_item| {
                let cell = list_item
                    .to_owned()
                    .downcast::<gtk::ColumnViewCell>()
                    .unwrap();
                let child = cell.child().unwrap();
                let label_object = child.downcast_ref::<gtk::Label>().unwrap();
                let model_item = cell
                    .item()
                    .to_owned()
                    .unwrap()
                    .downcast::<SmartDialogRow>()
                    .unwrap();
                label_object.set_label(extract(model_item).as_str());
            });
            id_col.set_factory(Some(&factory_id_col));
        }

        pub fn apply_nvme_smart_result(
            &self,
            result: NVMeSmartResult,
            parent: &PerformancePageDisk,
        ) {
            self.apply_common_smart_result(result.common_smart_result);

            self.sata_data.set_visible(false);
            self.nvme_data.set_visible(true);

            self.parent_page
                .set(Some(parent.downgrade().upgrade().unwrap()));

            self.percent_used
                .set_text(&format!("{}%", result.percent_used));

            self.avail_spare
                .set_text(result.avail_spare.to_string().as_str());
            self.spare_thresh
                .set_text(result.spare_thresh.to_string().as_str());

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

            self.warning_temp_time
                .set_text(crate::to_human_readable_time(result.warning_temp_time as u64).as_str());
            self.critical_temp_time
                .set_text(crate::to_human_readable_time(result.critical_temp_time as u64).as_str());
            self.ctrl_busy_minutes
                .set_text(crate::to_human_readable_time(result.ctrl_busy_minutes * 60).as_str());

            self.wctemp
                .set_text(&i18n_f("{} °C", &[&format!("{}", result.wctemp - 273)]));
            self.cctemp
                .set_text(&i18n_f("{} °C", &[&format!("{}", result.cctemp - 273)]));

            self.temp_sensors
                .set_text(&format!("{:?}", result.temp_sensors));
            self.unsafe_shutdowns
                .set_text(result.unsafe_shutdowns.to_string().as_str());
            self.media_errors
                .set_text(result.media_errors.to_string().as_str());
            self.num_err_log_entries
                .set_text(result.num_err_log_entries.to_string().as_str());
            self.power_cycles
                .set_text(result.power_cycles.to_string().as_str());

            self.ctrl_busy_minutes
                .set_text(result.ctrl_busy_minutes.to_string().as_str());
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
        }
    }

    impl AdwDialogImpl for SmartDataDialog {
        fn closed(&self) {}
    }
}

glib::wrapper! {
    pub struct SmartDataDialog(ObjectSubclass<imp::SmartDataDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
