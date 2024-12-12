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
    use std::collections::HashMap;
    use std::sync::Arc;
    use adw::gio::ListStore;
    use adw::glib::WeakRef;
    use crate::glib_clone;
    use crate::performance_page::eject_failure_row::{ContentType, EjectFailureRowBuilder, EjectFailureRowModel};
    use crate::sys_info_v2::{App, EjectResult, Process};
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::EjectFailureDialog)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/disk_eject_failure_dialog.ui")]
    pub struct EjectFailureDialog {
        #[template_child]
        pub column_view: TemplateChild<gtk::ColumnView>,
        #[template_child]
        pub name_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub pid_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub files_open_column: TemplateChild<gtk::ColumnViewColumn>,

        pub tree_list_sorter: Cell<Option<gtk::TreeListRowSorter>>,

        pub apps_model: Cell<ListStore>,
    }

    impl EjectFailureDialog {
        pub fn apply_eject_result(&self, result: EjectResult) {
            let app = app!();
            let parsed_results = app.handle_eject_result(result);

            let modelo = glib_clone!(self.apps_model);

            while modelo.n_items() > 0 {
                modelo.remove(0);
            }

            println!("Whamming");

            for parsed_result in parsed_results {
                let appname = parsed_result.0.to_string();
                let (app_obj, processes) = parsed_result.1;

                let app_root = EjectFailureRowBuilder::new();

                let iconname = match app_obj.icon.as_ref() {
                    Some(icon) => icon,
                    None => {&Arc::from("")}
                };

                let app_root = if appname != "" {
                    app_root
                        .icon(iconname)
                        .name(&*appname)
                        .content_type(ContentType::App)
                        .show_expander(true)
                        .build()
                } else {
                    app_root
                        .name("NO APP")
                        .content_type(ContentType::Process)
                        .show_expander(true)
                        .build()
                };

                println!("Creating app {}", appname);

                modelo.append(&app_root);

                let children = app_root.children().clone();

                for process in processes {
                    println!("Creating process {}", process.0);

                    let new_root = EjectFailureRowBuilder::new()
                        .icon(iconname)
                        .files_open(process.1.clone())
                        .pid(process.0)
                        .build();

                    children.append(&new_root);
                }
            }
        }
    }

    impl Default for EjectFailureDialog {
        fn default() -> Self {
            Self {
                column_view: Default::default(),
                name_column: Default::default(),
                pid_column: Default::default(),
                files_open_column: Default::default(),
                tree_list_sorter: Cell::new(None),
                apps_model: Cell::new(ListStore::new::<EjectFailureRowModel>()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EjectFailureDialog {
        const NAME: &'static str = "EjectFailureDialog";
        type Type = super::EjectFailureDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EjectFailureDialog {
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

            // todo init here
        }
    }

    impl WidgetImpl for EjectFailureDialog {
        fn realize(&self) {
            self.parent_realize();

            // todo init here
        }
    }

    impl AdwDialogImpl for EjectFailureDialog {
        fn closed(&self) {
            // let list_item = self.list_item();

            // todo buttons here
        }
    }
}

glib::wrapper! {
    pub struct EjectFailureDialog(ObjectSubclass<imp::EjectFailureDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

fn to_signal_id(id: u64) -> SignalHandlerId {
    unsafe { std::mem::transmute(id) }
}

fn from_signal_id(id: SignalHandlerId) -> u64 {
    unsafe { id.as_raw() }
}
