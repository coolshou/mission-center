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

use std::time::{SystemTime, UNIX_EPOCH};

use adw::{prelude::*, subclass::prelude::*};
use gtk::gio;
use gtk::glib::{self, g_critical};
use gtk::{Align, ColumnViewColumn};

use magpie_types::disks::smart_data::{Ata, Nvme};
use magpie_types::disks::{smart_data, SmartData};

use crate::i18n::*;

use super::SmartDialogRow;

mod imp {
    use crate::DataType;
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
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
    }

    impl SmartDataDialog {
        pub fn update_model(&self, data: SmartData) {
            let powered_on_nice = crate::to_human_readable_time(data.powered_on_seconds);
            self.powered_on.set_text(&powered_on_nice);

            if let Ok(since_the_epoch) = SystemTime::now().duration_since(UNIX_EPOCH) {
                let last_updated_nice = crate::to_human_readable_time(
                    since_the_epoch.as_secs_f32() as u64 - data.last_update_time,
                );
                self.last_updated
                    .set_text(&i18n_f("{} ago", &[&last_updated_nice]));
            } else {
                g_critical!("MissionCenter::SMARTDialog", "Time somehow went backwards");
                self.last_updated.set_text(&i18n("Unknown"));
            }

            self.status
                .set_text(format!("{:?}", data.test_result()).as_str());

            match data.data {
                Some(smart_data::Data::Ata(ata)) => self.apply_ata_smart_data(ata),
                Some(smart_data::Data::Nvme(nvme)) => self.apply_nvme_smart_data(nvme),
                None => {
                    self.sata_data.set_visible(false);
                    self.nvme_data.set_visible(false);
                }
            }
        }

        fn apply_ata_smart_data(&self, ata_smart_data: Ata) {
            self.sata_data.set_visible(true);
            self.nvme_data.set_visible(false);

            let mut rows = Vec::new();

            for parsed_result in ata_smart_data.attributes {
                let new_row = SmartDialogRow::new(
                    parsed_result.id as u8,
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
                    &i18n("Unknown"),
                );

                rows.push(new_row);
            }

            let rows: gio::ListStore = rows.into_iter().collect();

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

        fn apply_nvme_smart_data(&self, result: Nvme) {
            self.sata_data.set_visible(false);
            self.nvme_data.set_visible(true);

            if let Some(percent_used) = result.percent_used {
                self.percent_used.set_text(&format!("{}%", percent_used));
            } else {
                self.percent_used.set_text(&i18n("N/A"));
            }

            if let Some(avail_spare) = result.avail_spare {
                self.avail_spare.set_text(&avail_spare.to_string());
            } else {
                self.avail_spare.set_text(&i18n("N/A"));
            }

            if let Some(spare_thresh) = result.spare_thresh {
                self.spare_thresh.set_text(&spare_thresh.to_string());
            } else {
                self.spare_thresh.set_text(&i18n("N/A"));
            }

            if let Some(total_data_read) = result.total_data_read {
                let data_read = crate::to_human_readable_nice(total_data_read as f32, &DataType::DriveBytes);
                self.data_read.set_text(&data_read);
            } else {
                self.data_read.set_text(&i18n("N/A"));
            }

            if let Some(total_data_written) = result.total_data_written {
                let data_written = crate::to_human_readable_nice(total_data_written as f32, &DataType::DriveBytes);
                self.data_written.set_text(&data_written);
            } else {
                self.data_written.set_text(&i18n("N/A"));
            }

            if let Some(warning_temp_time) = result.warn_composite_temp_time {
                self.warning_temp_time
                    .set_text(crate::to_human_readable_time(warning_temp_time as u64).as_str());
            } else {
                self.warning_temp_time.set_text(&i18n("N/A"));
            }

            if let Some(critical_temp_time) = result.crit_composite_temp_time {
                self.critical_temp_time
                    .set_text(crate::to_human_readable_time(critical_temp_time as u64).as_str());
            } else {
                self.critical_temp_time.set_text(&i18n("N/A"));
            }

            if let Some(ctrl_busy_minutes) = result.ctrl_busy_minutes {
                self.ctrl_busy_minutes
                    .set_text(crate::to_human_readable_time(ctrl_busy_minutes).as_str());
            } else {
                self.ctrl_busy_minutes.set_text(&i18n("N/A"));
            }

            if let Some(wctemp) = result.warn_composite_temp_thresh {
                self.wctemp
                    .set_text(&i18n_f("{} °C", &[&format!("{}", wctemp - 273)]));
            } else {
                self.wctemp.set_text(&i18n("N/A"));
            }

            if let Some(cctemp) = result.crit_composite_temp_thresh {
                self.cctemp
                    .set_text(&i18n_f("{} °C", &[&format!("{}", cctemp - 273)]));
            } else {
                self.cctemp.set_text(&i18n("N/A"));
            }

            self.temp_sensors
                .set_text(&format!("{:?}", result.temp_sensors));

            if let Some(unsafe_shutdowns) = result.unsafe_shutdowns {
                self.unsafe_shutdowns
                    .set_text(&unsafe_shutdowns.to_string());
            } else {
                self.unsafe_shutdowns.set_text(&i18n("N/A"));
            }

            if let Some(media_errors) = result.media_errors {
                self.media_errors.set_text(&media_errors.to_string());
            } else {
                self.media_errors.set_text(&i18n("N/A"));
            }

            if let Some(num_err_log_entries) = result.num_err_log_entries {
                self.num_err_log_entries
                    .set_text(&num_err_log_entries.to_string());
            } else {
                self.num_err_log_entries.set_text(&i18n("N/A"));
            }

            if let Some(power_cycles) = result.power_cycles {
                self.power_cycles.set_text(&power_cycles.to_string());
            } else {
                self.power_cycles.set_text(&i18n("N/A"));
            }

            if let Some(ctrl_busy_minutes) = result.ctrl_busy_minutes {
                self.ctrl_busy_minutes
                    .set_text(&ctrl_busy_minutes.to_string());
            } else {
                self.ctrl_busy_minutes.set_text(&i18n("N/A"));
            }
        }

        fn setup_column_factory<'a, E>(id_col: ColumnViewColumn, alignment: Align, extract: E)
        where
            E: Fn(SmartDialogRow) -> String + 'static,
        {
            let factory_id_col = gtk::SignalListItemFactory::new();
            factory_id_col.connect_setup(move |_factory, list_item| {
                let cell = list_item.downcast_ref::<gtk::ColumnViewCell>().unwrap();
                cell.set_child(Some(&gtk::Label::builder().halign(alignment).build()));
            });
            factory_id_col.connect_bind(move |_factory, list_item| {
                let cell = match list_item.downcast_ref::<gtk::ColumnViewCell>() {
                    Some(cell) => cell,
                    None => {
                        g_critical!(
                            "MissionCenter::SMARTDialog",
                            "Failed to obtain GtkColumnViewCell from list item"
                        );
                        return;
                    }
                };

                let model_item = match cell
                    .item()
                    .and_then(|i| i.downcast::<SmartDialogRow>().ok())
                {
                    Some(model_item) => model_item,
                    None => {
                        g_critical!(
                            "MissionCenter::SMARTDialog",
                            "Failed to obtain SmartDialogRow item from GtkColumnViewCell"
                        );
                        return;
                    }
                };

                let label_object = match cell.child().and_then(|c| c.downcast::<gtk::Label>().ok())
                {
                    Some(label) => label,
                    None => {
                        g_critical!(
                            "MissionCenter::SMARTDialog",
                            "Failed to obtain child GtkLabel from GtkColumnViewCell"
                        );
                        return;
                    }
                };

                label_object.set_label(&extract(model_item));
            });

            id_col.set_factory(Some(&factory_id_col));
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

impl SmartDataDialog {
    pub fn new(smart_data: SmartData) -> Self {
        let this: Self = glib::Object::builder()
            .property("follows-content-size", true)
            .build();
        {
            let this = this.imp();
            this.update_model(smart_data);
        }

        this
    }
}
