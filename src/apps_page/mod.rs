/* apps_page/mod.rs
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
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::Write;

use adw::prelude::*;
use arrayvec::ArrayString;
use glib::translate::from_glib_full;
use glib::{g_critical, g_warning, gobject_ffi, Object, VariantTy};
use gtk::{gdk, gio, glib, subclass::prelude::*, TreeListRow};

use crate::app;
use crate::magpie_client::App;
use crate::settings;

use crate::i18n::ni18n_f;
use columns::*;
use row_model::{ContentType, RowModel, RowModelBuilder, SectionType};

mod columns;
mod models;
mod row_model;

pub const CSS_CELL_USAGE_LOW: &[u8] = b"cell { background-color: rgba(246, 211, 45, 0.3); }";
pub const CSS_CELL_USAGE_MEDIUM: &[u8] = b"cell { background-color: rgba(230, 97, 0, 0.3); }";
pub const CSS_CELL_USAGE_HIGH: &[u8] = b"cell { background-color: rgba(165, 29, 45, 0.3); }";

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/page.ui")]
    pub struct AppsPage {
        #[template_child]
        pub content: TemplateChild<adw::Clamp>,
        #[template_child]
        pub h1: TemplateChild<gtk::Label>,
        #[template_child]
        pub h2: TemplateChild<gtk::Label>,
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
        pub gpu_usage_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_memory_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        context_menu: TemplateChild<gtk::PopoverMenu>,

        pub apps_section: RowModel,
        pub processes_section: RowModel,

        pub root_process: Cell<u32>,
        pub app_icons: RefCell<HashMap<u32, String>>,
        pub selected_item: RefCell<RowModel>,

        pub action_stop: gio::SimpleAction,
        pub action_force_stop: gio::SimpleAction,
        pub action_details: gio::SimpleAction,

        pub use_merged_stats: Cell<bool>,
    }

    impl Default for AppsPage {
        fn default() -> Self {
            Self {
                content: TemplateChild::default(),
                h1: TemplateChild::default(),
                h2: TemplateChild::default(),
                column_view: TemplateChild::default(),
                name_column: TemplateChild::default(),
                pid_column: TemplateChild::default(),
                cpu_column: TemplateChild::default(),
                memory_column: TemplateChild::default(),
                shared_memory_column: TemplateChild::default(),
                drive_column: TemplateChild::default(),
                gpu_usage_column: TemplateChild::default(),
                gpu_memory_column: TemplateChild::default(),
                context_menu: TemplateChild::default(),

                apps_section: RowModelBuilder::new()
                    .name("Apps")
                    .content_type(ContentType::SectionHeader)
                    .section_type(SectionType::Apps)
                    .build(),
                processes_section: RowModelBuilder::new()
                    .name("Processes")
                    .content_type(ContentType::SectionHeader)
                    .section_type(SectionType::Processes)
                    .build(),

                root_process: Cell::new(1),
                app_icons: RefCell::new(HashMap::new()),
                selected_item: RefCell::new(RowModelBuilder::new().build()),

                action_stop: gio::SimpleAction::new("stop", None),
                action_force_stop: gio::SimpleAction::new("force-stop", None),
                action_details: gio::SimpleAction::new("details", None),

                use_merged_stats: Cell::new(false),
            }
        }
    }

    impl AppsPage {
        fn configure_actions(&self) {
            let this = self.obj();
            let this = this.as_ref();

            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("apps-page", Some(&actions));

            let action = gio::SimpleAction::new("show-context-menu", Some(VariantTy::TUPLE));
            action.connect_activate({
                let this = this.downgrade();
                move |_action, entry| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let this = this.imp();

                    let Some(model) = this.column_view.model().as_ref().cloned() else {
                        g_critical!(
                            "MissionCenter::AppsPage",
                            "Failed to get model for `show-context-menu` action"
                        );
                        return;
                    };

                    let Some((id, anchor_widget, x, y)) =
                        entry.and_then(|s| s.get::<(String, u64, f64, f64)>())
                    else {
                        g_critical!(
                            "MissionCenter::AppsPage",
                            "Failed to get service name and button from show-context-menu action"
                        );
                        return;
                    };

                    let anchor_widget = upgrade_weak_ptr(anchor_widget as _);
                    let anchor = this.calculate_anchor_point(&anchor_widget, x, y);

                    if select_item(&model, &id) {
                        this.context_menu.set_pointing_to(Some(&anchor));
                        this.context_menu.popup();
                    }
                }
            });
            actions.add_action(&action);

            self.action_stop.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let this = this.imp();

                    let selected_item = this.selected_item.borrow();
                    if selected_item.content_type() == ContentType::SectionHeader {
                        return;
                    }

                    if let Ok(magpie_client) = app!().sys_info() {
                        if selected_item.content_type() == ContentType::App {
                            let mut pids_to_terminate = vec![];
                            let primary_process = selected_item.children();
                            for i in 0..primary_process.n_items() {
                                let Some(child) = selected_item
                                    .children()
                                    .item(i)
                                    .and_then(|i| i.downcast::<RowModel>().ok())
                                else {
                                    continue;
                                };
                                pids_to_terminate.push(child.pid());
                            }
                            magpie_client.terminate_processes(pids_to_terminate);
                        } else {
                            magpie_client.terminate_process(selected_item.pid());
                        }
                    }
                }
            });
            actions.add_action(&self.action_stop);

            self.action_force_stop.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let this = this.imp();

                    let selected_item = this.selected_item.borrow();
                    if selected_item.content_type() == ContentType::SectionHeader {
                        return;
                    }

                    if let Ok(magpie_client) = app!().sys_info() {
                        if selected_item.content_type() == ContentType::App {
                            let mut pids_to_terminate = vec![];
                            let primary_process = selected_item.children();
                            for i in 0..primary_process.n_items() {
                                let Some(child) = selected_item
                                    .children()
                                    .item(i)
                                    .and_then(|i| i.downcast::<RowModel>().ok())
                                else {
                                    continue;
                                };
                                pids_to_terminate.push(child.pid());
                            }
                            magpie_client.kill_processes(pids_to_terminate);
                        } else {
                            magpie_client.kill_process(selected_item.pid());
                        }
                    }
                }
            });
            actions.add_action(&self.action_force_stop);

            self.action_details.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let this = this.imp();

                    let selected_item = this.selected_item.borrow();
                    if selected_item.content_type() == ContentType::SectionHeader {
                        return;
                    }
                }
            });
            actions.add_action(&self.action_details);
        }

        fn calculate_anchor_point(
            &self,
            widget: &Option<gtk::Widget>,
            x: f64,
            y: f64,
        ) -> gdk::Rectangle {
            let Some(anchor_widget) = widget else {
                g_warning!(
                    "MissionCenter::AppsPage",
                    "Failed to get anchor widget, popup will display in an arbitrary location"
                );
                return gdk::Rectangle::new(0, 0, 0, 0);
            };

            if x > 0. && y > 0. {
                self.context_menu.set_has_arrow(false);

                match anchor_widget
                    .compute_point(&*self.obj(), &gtk::graphene::Point::new(x as _, y as _))
                {
                    Some(p) => {
                        gdk::Rectangle::new(p.x().round() as i32, p.y().round() as i32, 1, 1)
                    }
                    None => {
                        g_critical!(
                            "MissionCenter::AppsPage",
                            "Failed to compute_point, context menu will not be anchored to mouse position"
                        );
                        gdk::Rectangle::new(x.round() as i32, y.round() as i32, 1, 1)
                    }
                }
            } else {
                self.context_menu.set_has_arrow(true);

                if let Some(bounds) = anchor_widget.compute_bounds(&*self.obj()) {
                    gdk::Rectangle::new(
                        bounds.x() as i32,
                        bounds.y() as i32,
                        bounds.width() as i32,
                        bounds.height() as i32,
                    )
                } else {
                    g_warning!(
                        "MissionCenter::AppsPage",
                        "Failed to get bounds for menu button, popup will display in an arbitrary location"
                    );
                    gdk::Rectangle::new(0, 0, 0, 0)
                }
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppsPage {
        const NAME: &'static str = "AppsPage";
        type Type = super::AppsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            RowModel::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppsPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.configure_actions();

            let settings = settings!();

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

            self.gpu_usage_column
                .set_factory(Some(&gpu_list_item_factory()));
            self.gpu_usage_column
                .set_sorter(Some(&gpu_sorter(&self.column_view)));

            self.gpu_memory_column
                .set_factory(Some(&gpu_memory_list_item_factory()));
            self.gpu_memory_column
                .set_sorter(Some(&gpu_memory_sorter(&self.column_view)));

            let column_view_title = self.column_view.first_child();
            adjust_view_header_alignment(column_view_title);

            self.use_merged_stats
                .set(settings.boolean("apps-page-merged-process-stats"));
            settings.connect_changed(Some("apps-page-merged-process-stats"), {
                let this = self.obj().downgrade();
                move |settings, _| {
                    if let Some(this) = this.upgrade() {
                        this.imp()
                            .use_merged_stats
                            .set(settings.boolean("apps-page-merged-process-stats"));
                    }
                }
            });
        }
    }

    impl WidgetImpl for AppsPage {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl BoxImpl for AppsPage {}
}

glib::wrapper! {
    pub struct AppsPage(ObjectSubclass<imp::AppsPage>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl AppsPage {
    pub fn set_initial_readings(&self, readings: &mut crate::magpie_client::Readings) -> bool {
        let imp = self.imp();

        // Set up the models here since we need access to the main application window
        // which is not yet available in the constructor.
        let base_model = models::base_model(&imp.apps_section, &imp.processes_section);
        let tree_list_model = models::tree_list_model(base_model);
        let filter_list_model = models::filter_list_model(tree_list_model);
        let sort_list_model = models::sort_list_model(filter_list_model, &imp.column_view);
        let selection_model = gtk::SingleSelection::new(Some(sort_list_model));
        imp.column_view.set_model(Some(&selection_model));

        selection_model.connect_selection_changed({
            let this = self.downgrade();
            move |model, index, n_items| {
                let Some(this) = this.upgrade() else {
                    return;
                };
                let imp = this.imp();

                // GtkSingleSelection is used as the selection model for our tree view which  means
                // we could use `model.selected_item()` to get the selected item.
                // However, we might want to support multiple selection in the future.
                let changed = model.selection_in_range(index, n_items);
                for i in 0..n_items {
                    let changed = changed.nth(i);
                    if model.is_selected(changed) {
                        let Some(row_model) = model
                            .item(changed)
                            .and_then(|i| i.downcast::<TreeListRow>().ok())
                            .and_then(|row| row.item())
                            .and_then(|obj| obj.downcast::<RowModel>().ok())
                        else {
                            return;
                        };

                        if row_model.content_type() == ContentType::SectionHeader {
                            imp.action_stop.set_enabled(false);
                            imp.action_force_stop.set_enabled(false);
                            imp.action_details.set_enabled(false);

                            return;
                        }

                        imp.action_stop.set_enabled(true);
                        imp.action_force_stop.set_enabled(true);
                        imp.action_details.set_enabled(true);

                        imp.selected_item.replace(row_model);

                        break;
                    }
                }
            }
        });

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
            &imp.gpu_usage_column,
            &imp.gpu_memory_column,
            readings,
        );

        let mut process_model_map = HashMap::new();
        let root_process = readings.running_processes.keys().min().unwrap_or(&1);
        if let Some(init) = readings.running_processes.get(root_process) {
            for child in &init.children {
                models::update_processes(
                    &readings.running_processes,
                    child,
                    &imp.processes_section.children(),
                    &imp.app_icons.borrow(),
                    "application-x-executable-symbolic",
                    imp.use_merged_stats.get(),
                    &mut process_model_map,
                );
            }
        }
        imp.root_process.set(*root_process);

        models::update_apps(
            &readings.running_apps,
            &readings.running_processes,
            &process_model_map,
            &mut imp.app_icons.borrow_mut(),
            &imp.apps_section.children(),
            imp.use_merged_stats.get(),
        );

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
            &imp.gpu_usage_column,
            &imp.gpu_memory_column,
            readings,
        );

        let mut process_model_map = HashMap::new();
        let root_process = imp.root_process.get();
        if let Some(init) = readings.running_processes.get(&root_process) {
            for child in &init.children {
                models::update_processes(
                    &readings.running_processes,
                    child,
                    &imp.processes_section.children(),
                    &imp.app_icons.borrow(),
                    "application-x-executable-symbolic",
                    imp.use_merged_stats.get(),
                    &mut process_model_map,
                );
            }
        }

        models::update_apps(
            &readings.running_apps,
            &readings.running_processes,
            &process_model_map,
            &mut imp.app_icons.borrow_mut(),
            &imp.apps_section.children(),
            imp.use_merged_stats.get(),
        );

        true
    }

    pub fn get_running_apps(&self) -> HashMap<String, App> {
        HashMap::new()
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
            .and_then(|i| i.downcast::<TreeListRow>().ok())
            .and_then(|row| row.item())
            .and_then(|obj| obj.downcast::<RowModel>().ok())
        {
            if item.content_type() != ContentType::SectionHeader && item.id() == id {
                model.select_item(i, false);
                return true;
            }
        }
    }

    false
}
