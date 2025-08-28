/* neo_services_page/models/selection.rs
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

use gtk::gio;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::neo_services_page::row_model::{ServicesContentType, ServicesRowModel};
use crate::neo_services_page::ServicesPage;

pub fn model(
    neo_services_page: &ServicesPage,
    sort_list_model: impl IsA<gio::ListModel>,
) -> gtk::SingleSelection {
    let selection_model = gtk::SingleSelection::new(Some(sort_list_model));
    selection_model.set_autoselect(true);

    let this = neo_services_page.downgrade();
    selection_model.connect_selected_item_notify(move |model| {
        let Some(this) = this.upgrade() else {
            return;
        };
        let imp = this.imp();

        let Some(row_model) = model
            .selected_item()
            .and_then(|item| item.downcast::<gtk::TreeListRow>().ok())
            .and_then(|row| row.item())
            .and_then(|obj| obj.downcast::<ServicesRowModel>().ok())
        else {
            return;
        };

        if row_model.icon() == "service-running" {
            imp.service_stop.set_enabled(true);
            imp.service_start.set_enabled(false);
            imp.service_restart.set_enabled(true);
        } else {
            imp.service_stop.set_enabled(false);
            imp.service_start.set_enabled(true);
            imp.service_restart.set_enabled(false);
        }

        if row_model.content_type() == ServicesContentType::Process {
            imp.process_ribbon.set_visible(true);
            imp.services_ribbon.set_visible(false);
        } else {
            imp.process_ribbon.set_visible(false);
            imp.services_ribbon.set_visible(true);
        }

        if row_model.content_type() == ServicesContentType::SectionHeader {
            imp.action_stop.set_enabled(false);
            imp.action_force_stop.set_enabled(false);
            imp.action_suspend.set_enabled(false);
            imp.action_continue.set_enabled(false);
            imp.action_hangup.set_enabled(false);
            imp.action_interrupt.set_enabled(false);
            imp.action_user_one.set_enabled(false);
            imp.action_user_two.set_enabled(false);
            imp.action_details.set_enabled(false);

            return;
        }

        imp.action_stop.set_enabled(true);
        imp.action_force_stop.set_enabled(true);
        imp.action_suspend.set_enabled(true);
        imp.action_continue.set_enabled(true);
        imp.action_hangup.set_enabled(true);
        imp.action_interrupt.set_enabled(true);
        imp.action_user_one.set_enabled(true);
        imp.action_user_two.set_enabled(true);
        imp.action_details.set_enabled(true);

        imp.selected_item.replace(row_model);
    });

    selection_model
}
