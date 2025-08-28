/* neo_services_page/actions.rs
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

use adw::gdk;
use adw::glib::g_warning;
use adw::prelude::AdwDialogExt;
use gtk::gio;
use gtk::glib::{g_critical, VariantTy};
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use super::imp::ServicesPage as ServicesPageImp;
use super::process_details_dialog::ProcessDetailsDialog;
use super::row_model::{ServicesContentType, ServicesRowModel};
use super::ServicesPage;
use super::{select_item, upgrade_weak_ptr};
use crate::app;
use crate::neo_services_page::service_details_dialog::ServiceDetailsDialog;

pub fn configure(imp: &ServicesPageImp) {
    let this = imp.obj();

    let actions = gio::SimpleActionGroup::new();
    this.insert_action_group("apps-page", Some(&actions));

    let action = gio::SimpleAction::new("show-context-menu", Some(VariantTy::TUPLE));
    action.connect_activate({
        let this = this.downgrade();
        move |_action, entry| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let imp = this.imp();

            let Some(model) = imp.column_view.model().as_ref().cloned() else {
                g_critical!(
                    "MissionCenter::ServicesPage",
                    "Failed to get model for `show-context-menu` action"
                );
                return;
            };

            let Some((id, anchor_widget, x, y)) =
                entry.and_then(|s| s.get::<(String, u64, f64, f64)>())
            else {
                g_critical!(
                    "MissionCenter::ServicesPage",
                    "Failed to get service name and button from show-context-menu action"
                );
                return;
            };

            let anchor_widget = upgrade_weak_ptr(anchor_widget as _);
            let anchor = calculate_anchor_point(&this, &anchor_widget, x, y);

            if select_item(&model, &id) {
                match imp.selected_item.borrow().content_type() {
                    // should never trigger
                    ServicesContentType::SectionHeader => {}
                    ServicesContentType::Service => {
                        imp.service_context_menu.set_pointing_to(Some(&anchor));
                        imp.service_context_menu.popup();
                    }
                    ServicesContentType::Process => {
                        imp.context_menu.set_pointing_to(Some(&anchor));
                        imp.context_menu.popup();
                    }
                }
            }
        }
    });
    actions.add_action(&action);

    imp.action_stop.set_enabled(false);
    imp.action_stop.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let this = this.imp();

            let selected_item = this.selected_item.borrow();
            if selected_item.content_type() == ServicesContentType::SectionHeader {
                return;
            }

            if let Ok(magpie_client) = app!().sys_info() {
                magpie_client.terminate_process(selected_item.pid());
            }
        }
    });
    actions.add_action(&imp.action_stop);

    imp.action_force_stop.set_enabled(false);
    imp.action_force_stop.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let this = this.imp();

            let selected_item = this.selected_item.borrow();
            if selected_item.content_type() == ServicesContentType::SectionHeader {
                return;
            }

            if let Ok(magpie_client) = app!().sys_info() {
                magpie_client.kill_process(selected_item.pid());
            }
        }
    });
    actions.add_action(&imp.action_force_stop);

    imp.action_suspend.set_enabled(false);
    imp.action_suspend.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let this = this.imp();

            let selected_item = this.selected_item.borrow();
            if selected_item.content_type() == ServicesContentType::SectionHeader {
                return;
            }

            if let Ok(magpie_client) = app!().sys_info() {
                magpie_client.suspend_process(selected_item.pid());
            }
        }
    });
    actions.add_action(&imp.action_suspend);

    imp.action_continue.set_enabled(false);
    imp.action_continue.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let this = this.imp();

            let selected_item = this.selected_item.borrow();
            if selected_item.content_type() == ServicesContentType::SectionHeader {
                return;
            }

            if let Ok(magpie_client) = app!().sys_info() {
                magpie_client.continue_process(selected_item.pid());
            }
        }
    });
    actions.add_action(&imp.action_continue);

    imp.action_hangup.set_enabled(false);
    imp.action_hangup.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let this = this.imp();

            let selected_item = this.selected_item.borrow();
            if selected_item.content_type() == ServicesContentType::SectionHeader {
                return;
            }

            if let Ok(magpie_client) = app!().sys_info() {
                magpie_client.hangup_process(selected_item.pid());
            }
        }
    });
    actions.add_action(&imp.action_hangup);

    imp.action_interrupt.set_enabled(false);
    imp.action_interrupt.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let this = this.imp();

            let selected_item = this.selected_item.borrow();
            if selected_item.content_type() == ServicesContentType::SectionHeader {
                return;
            }

            if let Ok(magpie_client) = app!().sys_info() {
                magpie_client.interrupt_process(selected_item.pid());
            }
        }
    });
    actions.add_action(&imp.action_interrupt);

    imp.action_user_one.set_enabled(false);
    imp.action_user_one.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let this = this.imp();

            let selected_item = this.selected_item.borrow();
            if selected_item.content_type() == ServicesContentType::SectionHeader {
                return;
            }

            if let Ok(magpie_client) = app!().sys_info() {
                magpie_client.user_signal_one_process(selected_item.pid());
            }
        }
    });
    actions.add_action(&imp.action_user_one);

    imp.action_user_two.set_enabled(false);
    imp.action_user_two.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let this = this.imp();

            let selected_item = this.selected_item.borrow();
            if selected_item.content_type() == ServicesContentType::SectionHeader {
                return;
            }

            if let Ok(magpie_client) = app!().sys_info() {
                magpie_client.user_signal_two_process(selected_item.pid());
            }
        }
    });
    actions.add_action(&imp.action_user_two);

    imp.action_details.set_enabled(false);
    imp.action_details.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let imp = this.imp();

            let selected_item = imp.selected_item.borrow();
            if selected_item.content_type() == ServicesContentType::SectionHeader {
                return;
            }

            if selected_item.content_type() == ServicesContentType::Process {
                ProcessDetailsDialog::new(imp.selected_item.borrow().clone()).present(Some(&this));
            } else {
                ServiceDetailsDialog::new(imp.selected_item.borrow().clone()).present(Some(&this));
            };
        }
    });
    actions.add_action(&imp.action_details);

    let action = gio::SimpleAction::new("collapse-all", None);
    action.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let imp = this.imp();

            let Some(selection_model) = imp
                .column_view
                .model()
                .and_then(|model| model.downcast::<gtk::SingleSelection>().ok())
            else {
                g_critical!(
                    "MissionCenter::ServicesPage",
                    "Failed to get model for `collapse-all` action"
                );
                return;
            };

            let mut count = 0;
            for i in 0..selection_model.n_items() {
                let Some(row) = selection_model
                    .item(i)
                    .and_then(|item| item.downcast::<gtk::TreeListRow>().ok())
                else {
                    return;
                };

                let Some(row_model) = row
                    .item()
                    .and_then(|item| item.downcast::<ServicesRowModel>().ok())
                else {
                    continue;
                };

                if row_model.content_type() != ServicesContentType::SectionHeader {
                    continue;
                }

                row.set_expanded(false);
                count += 1;

                if count >= 2 {
                    break;
                }
            }
        }
    });
    actions.add_action(&action);
}

fn calculate_anchor_point(
    neo_services_page: &ServicesPage,
    widget: &Option<gtk::Widget>,
    x: f64,
    y: f64,
) -> gdk::Rectangle {
    let imp = neo_services_page.imp();

    let Some(anchor_widget) = widget else {
        g_warning!(
            "MissionCenter::ServicesPage",
            "Failed to get anchor widget, popup will display in an arbitrary location"
        );
        return gdk::Rectangle::new(0, 0, 0, 0);
    };

    if x > 0. && y > 0. {
        imp.context_menu.set_has_arrow(false);

        match anchor_widget.compute_point(
            neo_services_page,
            &gtk::graphene::Point::new(x as _, y as _),
        ) {
            Some(p) => gdk::Rectangle::new(p.x().round() as i32, p.y().round() as i32, 1, 1),
            None => {
                g_critical!(
                    "MissionCenter::ServicesPage",
                    "Failed to compute_point, context menu will not be anchored to mouse position"
                );
                gdk::Rectangle::new(x.round() as i32, y.round() as i32, 1, 1)
            }
        }
    } else {
        imp.context_menu.set_has_arrow(true);

        if let Some(bounds) = anchor_widget.compute_bounds(&*imp.obj()) {
            gdk::Rectangle::new(
                bounds.x() as i32,
                bounds.y() as i32,
                bounds.width() as i32,
                bounds.height() as i32,
            )
        } else {
            g_warning!(
                "MissionCenter::ServicesPage",
                "Failed to get bounds for menu button, popup will display in an arbitrary location"
            );
            gdk::Rectangle::new(0, 0, 0, 0)
        }
    }
}

fn app_pids(row_model: &ServicesRowModel) -> Vec<u32> {
    let children = row_model.children();
    let mut result = Vec::with_capacity(children.n_items() as usize);

    for i in 0..children.n_items() {
        let Some(child) = children
            .item(i)
            .and_then(|i| i.downcast::<ServicesRowModel>().ok())
            .and_then(|rm| find_stoppable_child(&rm))
        else {
            continue;
        };
        result.push(child.pid());
    }

    result
}

fn find_stoppable_child(row_model: &ServicesRowModel) -> Option<ServicesRowModel> {
    if row_model.name() != "bwrap" {
        return Some(row_model.clone());
    }

    let children = row_model.children();
    for i in 0..children.n_items() {
        let Some(child) = children
            .item(i)
            .and_then(|i| i.downcast::<ServicesRowModel>().ok())
        else {
            continue;
        };
        if let Some(rm) = find_stoppable_child(&child) {
            return Some(rm);
        }
    }

    None
}
