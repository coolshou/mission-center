/* apps_page/actions.rs
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

use crate::app;

use super::details_dialog::DetailsDialog;
use super::imp::AppsPage as AppsPageImp;
use super::row_model::{ContentType, RowModel};
use super::AppsPage;
use super::{select_item, upgrade_weak_ptr};

pub fn configure(imp: &AppsPageImp) {
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
            let anchor = calculate_anchor_point(&this, &anchor_widget, x, y);

            if select_item(&model, &id) {
                imp.context_menu.set_pointing_to(Some(&anchor));
                imp.context_menu.popup();
            }
        }
    });
    actions.add_action(&action);

    imp.action_stop.connect_activate({
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
                    magpie_client.terminate_processes(app_pids(&*selected_item));
                } else {
                    magpie_client.terminate_process(selected_item.pid());
                }
            }
        }
    });
    actions.add_action(&imp.action_stop);

    imp.action_force_stop.connect_activate({
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
                    magpie_client.kill_processes(app_pids(&*selected_item));
                } else {
                    magpie_client.kill_process(selected_item.pid());
                }
            }
        }
    });
    actions.add_action(&imp.action_force_stop);

    imp.action_details.connect_activate({
        let this = this.downgrade();
        move |_action, _| {
            let Some(this) = this.upgrade() else {
                return;
            };
            let imp = this.imp();

            let selected_item = imp.selected_item.borrow();
            if selected_item.content_type() == ContentType::SectionHeader {
                return;
            }

            let details_dialog = DetailsDialog::new(imp.selected_item.borrow().clone());
            details_dialog.present(Some(&this));
        }
    });
    actions.add_action(&imp.action_details);
}

fn calculate_anchor_point(
    apps_page: &AppsPage,
    widget: &Option<gtk::Widget>,
    x: f64,
    y: f64,
) -> gdk::Rectangle {
    let imp = apps_page.imp();

    let Some(anchor_widget) = widget else {
        g_warning!(
            "MissionCenter::AppsPage",
            "Failed to get anchor widget, popup will display in an arbitrary location"
        );
        return gdk::Rectangle::new(0, 0, 0, 0);
    };

    if x > 0. && y > 0. {
        imp.context_menu.set_has_arrow(false);

        match anchor_widget.compute_point(apps_page, &gtk::graphene::Point::new(x as _, y as _)) {
            Some(p) => gdk::Rectangle::new(p.x().round() as i32, p.y().round() as i32, 1, 1),
            None => {
                g_critical!(
                    "MissionCenter::AppsPage",
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
                "MissionCenter::AppsPage",
                "Failed to get bounds for menu button, popup will display in an arbitrary location"
            );
            gdk::Rectangle::new(0, 0, 0, 0)
        }
    }
}

fn app_pids(row_model: &RowModel) -> Vec<u32> {
    let children = row_model.children();
    let mut result = Vec::with_capacity(children.n_items() as usize);

    for i in 0..children.n_items() {
        let Some(child) = children
            .item(i)
            .and_then(|i| i.downcast::<RowModel>().ok())
            .and_then(|rm| find_stoppable_child(&rm))
        else {
            continue;
        };
        result.push(child.pid());
    }

    result
}

fn find_stoppable_child(row_model: &RowModel) -> Option<RowModel> {
    if row_model.name() != "bwrap" {
        return Some(row_model.clone());
    }

    let children = row_model.children();
    for i in 0..children.n_items() {
        let Some(child) = children.item(i).and_then(|i| i.downcast::<RowModel>().ok()) else {
            continue;
        };
        if let Some(rm) = find_stoppable_child(&child) {
            return Some(rm);
        }
    }

    None
}
