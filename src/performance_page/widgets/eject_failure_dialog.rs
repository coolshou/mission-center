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

use adw::ResponseAppearance;
use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::g_critical;
use gtk::glib::{self, g_warning};
use std::cell::Cell;
use std::sync::Arc;

use crate::app;
use crate::i18n;
use crate::performance_page::disk::PerformancePageDisk;
use crate::performance_page::widgets::eject_failure_row::EjectFailureRowBuilder;
use crate::sys_info_v2::EjectResult;

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(
        resource = "/io/missioncenter/MissionCenter/ui/performance_page/disk_eject_failure_dialog.ui"
    )]
    pub struct EjectFailureDialog {
        #[template_child]
        pub column_view: TemplateChild<gtk::ListBox>,

        pub parent_page: Cell<Option<PerformancePageDisk>>,
    }

    impl EjectFailureDialog {
        pub fn apply_eject_result(&self, result: EjectResult, parent: &PerformancePageDisk) {
            let app = app!();
            let parsed_results = app.handle_eject_result(result);

            let modelo = self.column_view.get();

            self.parent_page
                .set(Some(parent.downgrade().upgrade().unwrap()));

            modelo.remove_all();

            for parsed_result in parsed_results {
                let appname = parsed_result.0.to_string();
                let (app_obj, processes) = parsed_result.1;

                let iconname = match app_obj.icon.as_ref() {
                    Some(icon) => icon,
                    None => &Arc::from(""),
                };

                let parent_id = &parent
                    .imp()
                    .raw_disk_id
                    .get()
                    .expect("Expected a raw disk id, got none");

                for process in processes {
                    if !process.1.is_empty() {
                        let new_root = EjectFailureRowBuilder::new()
                            .id(parent_id)
                            .icon(iconname)
                            .files_open(process.1.clone())
                            .pid(process.0)
                            .name(&appname)
                            .parent_page(parent.downgrade().upgrade().unwrap())
                            .build();

                        modelo.append(&new_root.imp().row_entry.get());
                    }

                    if !process.2.is_empty() {
                        let new_root = EjectFailureRowBuilder::new()
                            .id(parent_id)
                            .icon(iconname)
                            .files_open(process.2.clone())
                            .pid(process.0)
                            .name(&appname)
                            .parent_page(parent.downgrade().upgrade().unwrap())
                            .build();

                        modelo.append(&new_root.imp().row_entry.get());
                    }
                }
            }
        }
    }

    impl Default for EjectFailureDialog {
        fn default() -> Self {
            Self {
                column_view: Default::default(),
                parent_page: Cell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EjectFailureDialog {
        const NAME: &'static str = "EjectFailureDialog";
        type Type = super::EjectFailureDialog;
        type ParentType = adw::AlertDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EjectFailureDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let close = "close";
            let retry = "retry";
            let kill = "kill";
            self.obj().add_response(close, &i18n::i18n("Close"));
            self.obj().add_response(retry, &i18n::i18n("Retry"));
            self.obj().add_response(kill, &i18n::i18n("Kill All"));

            self.obj()
                .set_response_appearance(close, ResponseAppearance::Default);
            self.obj()
                .set_response_appearance(retry, ResponseAppearance::Default);
            self.obj()
                .set_response_appearance(kill, ResponseAppearance::Destructive);
        }
    }

    impl AdwAlertDialogImpl for EjectFailureDialog {
        fn response(&self, response: &str) {
            match response {
                "retry" => {
                    match app!().sys_info().and_then(move |sys_info| {
                        let parent = match self.parent_page.take() {
                            Some(parent) => parent,
                            None => {
                                g_critical!(
                                    "MissionCenter::DetailsDialog",
                                    "`parent_page` was unexpectedly empty",
                                );
                                return Ok(());
                            }
                        };

                        let disk_id = match parent.imp().raw_disk_id.get() {
                            Some(id) => id,
                            None => {
                                g_critical!(
                                    "MissionCenter::DetailsDialog",
                                    "`disk_id` was unexpectedly empty",
                                );
                                return Ok(());
                            }
                        };
                        let eject_result = sys_info.eject_disk(disk_id, false, 0);

                        parent.imp().show_eject_result(&parent, eject_result);

                        Ok(())
                    }) {
                        Err(e) => {
                            g_warning!(
                                "MissionCenter::DetailsDialog",
                                "Failed to get `sys_info`: {}",
                                e
                            );
                        }
                        _ => {}
                    }
                }
                "kill" => {
                    match app!().sys_info().and_then(move |sys_info| {
                        let parent = match self.parent_page.take() {
                            Some(parent) => parent,
                            None => {
                                g_critical!(
                                    "MissionCenter::DetailsDialog",
                                    "`parent_page` was unexpectedly empty",
                                );
                                return Ok(());
                            }
                        };

                        let disk_id = match parent.imp().raw_disk_id.get() {
                            Some(id) => id,
                            None => {
                                g_critical!(
                                    "MissionCenter::DetailsDialog",
                                    "`disk_id` was unexpectedly empty",
                                );
                                return Ok(());
                            }
                        };
                        let eject_result = sys_info.eject_disk(disk_id, true, 0);

                        parent.imp().show_eject_result(&parent, eject_result);

                        Ok(())
                    }) {
                        Err(e) => {
                            g_warning!(
                                "MissionCenter::DetailsDialog",
                                "Failed to get `sys_info`: {}",
                                e
                            );
                        }
                        _ => {}
                    }
                }
                "close" => {}
                e => {
                    g_warning!("MissionCenter::DetailsDialog", "Unexpected response: {}", e);
                }
            }
        }
    }

    impl WidgetImpl for EjectFailureDialog {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl AdwDialogImpl for EjectFailureDialog {
        fn closed(&self) {}
    }
}

glib::wrapper! {
    pub struct EjectFailureDialog(ObjectSubclass<imp::EjectFailureDialog>)
        @extends adw::AlertDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
