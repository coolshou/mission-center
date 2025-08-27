/* services_page/mod.rs
 *
 * Copyright 2025 Mission Center Developers
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

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::fmt::Write;

use adw::glib::{ParamSpec, Properties, Value};
use adw::prelude::*;
use arrayvec::ArrayString;
use glib::translate::from_glib_full;
use glib::{gobject_ffi, Object};
use gtk::{gio, glib, subclass::prelude::*};

use crate::magpie_client::App;

use crate::i18n::{i18n, ni18n_f};
use columns::*;
use row_model::{ServicesContentType, ServicesRowModel, ServicesRowModelBuilder, ServicesSectionType};
use crate::neo_services_page::row_model::ServicesSectionType::{SystemServices, UserServices};

mod actions;
mod columns;
mod details_dialog;
mod models;
mod row_model;
mod settings;

mod imp {
    use super::*;

    #[derive(Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::ServicesPage)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/services_page/page.ui")]
    pub struct ServicesPage {
        #[template_child]
        pub h1: TemplateChild<gtk::Label>,
        #[template_child]
        pub h2: TemplateChild<gtk::Label>,
        #[template_child]
        pub collapse_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub stop_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub force_stop_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub details_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub column_view: TemplateChild<gtk::ColumnView>,
        #[template_child]
        pub name_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub pid_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub cpu_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub memory_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub shared_memory_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub drive_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub network_usage_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_usage_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_memory_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::PopoverMenu>,

        #[property(get, set)]
        pub show_column_separators: Cell<bool>,

        pub user_section: ServicesRowModel,
        pub system_section: ServicesRowModel,

        pub running_apps: RefCell<HashMap<String, App>>,

        pub row_sorter: OnceCell<gtk::TreeListRowSorter>,

        pub app_icons: RefCell<HashMap<u32, String>>,
        pub selected_item: RefCell<ServicesRowModel>,

        pub use_merged_stats: Cell<bool>,
    }

    impl Default for ServicesPage {
        fn default() -> Self {
            Self {
                h1: TemplateChild::default(),
                h2: TemplateChild::default(),
                collapse_label: TemplateChild::default(),
                stop_label: TemplateChild::default(),
                force_stop_label: TemplateChild::default(),
                details_label: TemplateChild::default(),
                column_view: TemplateChild::default(),
                name_column: TemplateChild::default(),
                pid_column: TemplateChild::default(),
                cpu_column: TemplateChild::default(),
                memory_column: TemplateChild::default(),
                shared_memory_column: TemplateChild::default(),
                drive_column: TemplateChild::default(),
                network_usage_column: TemplateChild::default(),
                gpu_usage_column: TemplateChild::default(),
                gpu_memory_column: TemplateChild::default(),
                context_menu: TemplateChild::default(),

                show_column_separators: Cell::new(false),

                user_section: ServicesRowModelBuilder::new()
                    .name(&i18n("Apps"))
                    .content_type(ServicesContentType::SectionHeader)
                    .section_type(ServicesSectionType::UserServices)
                    .build(),
                system_section: ServicesRowModelBuilder::new()
                    .name(&i18n("Processes"))
                    .content_type(ServicesContentType::SectionHeader)
                    .section_type(ServicesSectionType::SystemServices)
                    .build(),

                running_apps: RefCell::new(HashMap::new()),

                row_sorter: OnceCell::new(),

                app_icons: RefCell::new(HashMap::new()),
                selected_item: RefCell::new(ServicesRowModelBuilder::new().build()),

                use_merged_stats: Cell::new(false),
            }
        }
    }

    impl ServicesPage {
        pub fn collapse(&self) {
            self.collapse_label.set_visible(false);
            self.stop_label.set_visible(false);
            self.force_stop_label.set_visible(false);
            self.details_label.set_visible(false);

            self.h2.set_visible(false);
        }

        pub fn expand(&self) {
            self.collapse_label.set_visible(true);
            self.stop_label.set_visible(true);
            self.force_stop_label.set_visible(true);
            self.details_label.set_visible(true);

            self.h2.set_visible(true);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServicesPage {
        const NAME: &'static str = "ServicesPage";
        type Type = super::ServicesPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            ServicesRowModel::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ServicesPage {
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

            actions::configure(self);

            update_column_order(&self.column_view);

            self.name_column
                .set_factory(Some(&name_list_item_factory()));
            self.name_column
                .set_sorter(Some(&name_sorter(&self.column_view)));

            self.pid_column.set_factory(Some(&pid_list_item_factory()));
            self.pid_column
                .set_sorter(Some(&pid_sorter(&self.column_view)));

            self.cpu_column.set_factory(Some(&cpu_list_item_factory()));
            self.cpu_column
                .set_sorter(Some(&cpu_sorter(&self.column_view)));

            self.memory_column
                .set_factory(Some(&memory_list_item_factory()));
            self.memory_column
                .set_sorter(Some(&memory_sorter(&self.column_view)));

            self.shared_memory_column
                .set_factory(Some(&shared_memory_list_item_factory()));
            self.shared_memory_column
                .set_sorter(Some(&shared_memory_sorter(&self.column_view)));

            self.drive_column
                .set_factory(Some(&drive_list_item_factory()));
            self.drive_column
                .set_sorter(Some(&drive_sorter(&self.column_view)));

            self.network_usage_column
                .set_factory(Some(&network_list_item_factory()));
            self.network_usage_column
                .set_sorter(Some(&network_sorter(&self.column_view)));

            self.gpu_usage_column
                .set_factory(Some(&gpu_list_item_factory()));
            self.gpu_usage_column
                .set_sorter(Some(&gpu_sorter(&self.column_view)));

            self.gpu_memory_column
                .set_factory(Some(&gpu_memory_list_item_factory()));
            self.gpu_memory_column
                .set_sorter(Some(&gpu_memory_sorter(&self.column_view)));

            // Make sure to do this after the columns are set up otherwise restoring sorting
            // won't work
            settings::configure(self);

            let column_view_title = self.column_view.first_child();
            adjust_view_header_alignment(column_view_title);
        }
    }

    impl WidgetImpl for ServicesPage {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl BoxImpl for ServicesPage {}
}

glib::wrapper! {
    pub struct ServicesPage(ObjectSubclass<imp::ServicesPage>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl ServicesPage {
    pub fn set_initial_readings(&self, readings: &mut crate::magpie_client::Readings) -> bool {
        let imp = self.imp();

        // Set up the models here since we need access to the main application window
        // which is not yet available in the constructor.
        let base_model = models::base_model(&imp.user_section, &imp.system_section);
        let tree_list_model = models::tree_list_model(base_model);
        let filter_list_model = models::filter_list_model(tree_list_model);
        let (sort_list_model, row_sorter) =
            models::sort_list_model(filter_list_model, &imp.column_view);
        let selection_model = models::selection_model(&self, sort_list_model);
        imp.column_view.set_model(Some(&selection_model));

        let _ = imp.row_sorter.set(row_sorter);

        let mut buffer = ArrayString::<64>::new();
        let running_apps_len = readings.running_apps.len() as u32;
        let _ = write!(&mut buffer, "{}", running_apps_len);
        imp.h1.set_label(&ni18n_f(
            "{} Running App",
            "{} Running Apps",
            running_apps_len,
            &[buffer.as_str()],
        ));

        buffer.clear();
        let running_processes_len = readings.running_processes.len() as u32;
        let _ = write!(&mut buffer, "{}", running_processes_len);
        imp.h2.set_label(&ni18n_f(
            "{} Running Process",
            "{} Running Processes",
            running_processes_len,
            &[buffer.as_str()],
        ));

        update_column_titles(
            &imp.cpu_column,
            &imp.memory_column,
            &imp.drive_column,
            &imp.network_usage_column,
            &imp.gpu_usage_column,
            &imp.gpu_memory_column,
            readings,
        );

        models::update_services(
            &readings.running_processes,
            &readings.services,
            &imp.system_section.children(),
            &imp.app_icons.borrow(),
            "application-x-executable-symbolic",
            imp.use_merged_stats.get(),
            SystemServices
        );

        models::update_services(
            &readings.running_processes,
            &readings.services,
            &imp.user_section.children(),
            &imp.app_icons.borrow(),
            "application-x-executable-symbolic",
            imp.use_merged_stats.get(),
            UserServices
        );

        // Select the first item in the list
        selection_model.set_selected(0);

        true
    }

    pub fn update_readings(&self, readings: &mut crate::magpie_client::Readings) -> bool {
        let imp = self.imp();

        let mut buffer = ArrayString::<64>::new();
        let running_apps_len = readings.running_apps.len() as u32;
        let _ = write!(&mut buffer, "{}", running_apps_len);
        imp.h1.set_label(&ni18n_f(
            "{} Running App",
            "{} Running Apps",
            running_apps_len,
            &[buffer.as_str()],
        ));

        buffer.clear();
        let running_processes_len = readings.running_processes.len() as u32;
        let _ = write!(&mut buffer, "{}", running_processes_len);
        imp.h2.set_label(&ni18n_f(
            "{} Running Process",
            "{} Running Processes",
            running_processes_len,
            &[buffer.as_str()],
        ));

        update_column_titles(
            &imp.cpu_column,
            &imp.memory_column,
            &imp.drive_column,
            &imp.network_usage_column,
            &imp.gpu_usage_column,
            &imp.gpu_memory_column,
            readings,
        );

        models::update_services(
            &readings.running_processes,
            &readings.services,
            &imp.system_section.children(),
            &imp.app_icons.borrow(),
            "application-x-executable-symbolic",
            imp.use_merged_stats.get(),
            SystemServices
        );

        models::update_services(
            &readings.running_processes,
            &readings.services,
            &imp.user_section.children(),
            &imp.app_icons.borrow(),
            "application-x-executable-symbolic",
            imp.use_merged_stats.get(),
            UserServices
        );

        let _ = std::mem::replace(
            &mut *imp.running_apps.borrow_mut(),
            std::mem::take(&mut readings.running_apps),
        );

        if let Some(row_sorter) = imp.row_sorter.get() {
            row_sorter.changed(gtk::SorterChange::Different)
        }

        if readings.network_stats_error.is_some() {
            imp.network_usage_column.set_visible(false);
        }

        true
    }

    #[inline]
    pub fn collapse(&self) {
        self.imp().collapse();
    }

    #[inline]
    pub fn expand(&self) {
        self.imp().expand();
    }

    pub fn running_apps(&self) -> HashMap<String, App> {
        self.imp().running_apps.borrow().clone()
    }
}

fn upgrade_weak_ptr(ptr: usize) -> Option<gtk::Widget> {
    let ptr = unsafe { gobject_ffi::g_weak_ref_get(ptr as *mut _) };
    if ptr.is_null() {
        return None;
    }
    let obj: Object = unsafe { from_glib_full(ptr) };
    obj.downcast::<gtk::Widget>().ok()
}

fn select_item(model: &gtk::SelectionModel, id: &str) -> bool {
    for i in 0..model.n_items() {
        if let Some(item) = model
            .item(i)
            .and_then(|i| i.downcast::<gtk::TreeListRow>().ok())
            .and_then(|row| row.item())
            .and_then(|obj| obj.downcast::<ServicesRowModel>().ok())
        {
            if item.content_type() != ServicesContentType::SectionHeader && item.id() == id {
                model.select_item(i, false);
                return true;
            }
        }
    }

    false
}
