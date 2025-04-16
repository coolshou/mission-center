/* apps_page/models/filter_list.rs
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
use gtk::glib::g_critical;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use textdistance::{Algorithm, Levenshtein};

use crate::app;
use crate::apps_page::row_model::{ContentType, RowModel};

pub fn model(tree_list_model: impl IsA<gio::ListModel>) -> gtk::FilterListModel {
    let Some(window) = app!().window() else {
        g_critical!(
            "MissionCenter::AppsPage",
            "Failed to get MissionCenterWindow instance; searching and filtering will not function"
        );
        return gtk::FilterListModel::new(Some(tree_list_model), None::<gtk::CustomFilter>);
    };

    let filter = gtk::CustomFilter::new({
        let window = window.downgrade();
        move |obj| {
            let Some(window) = window.upgrade() else {
                return true;
            };
            let window = window.imp();

            if !window.search_button.is_active() {
                return true;
            }

            if window.header_search_entry.text().is_empty() {
                return true;
            }

            let Some(row_model) = obj
                .downcast_ref::<gtk::TreeListRow>()
                .and_then(|row| row.item())
                .and_then(|item| item.downcast::<RowModel>().ok())
            else {
                return false;
            };

            if row_model.content_type() == ContentType::SectionHeader {
                return true;
            }

            let entry_name = row_model.name().to_lowercase();
            let pid = row_model.pid().to_string();
            let search_query = window.header_search_entry.text().to_lowercase();

            if entry_name.contains(&search_query) || pid.contains(&search_query) {
                return true;
            }

            if search_query.contains(&entry_name) || search_query.contains(&pid) {
                return true;
            }

            let str_distance = Levenshtein::default()
                .for_str(&entry_name, &search_query)
                .ndist();
            if str_distance <= 0.6 {
                return true;
            }

            false
        }
    });

    window.imp().header_search_entry.connect_search_changed({
        let filter = filter.downgrade();
        let window = window.downgrade();
        move |_| {
            if let Some(window) = window.upgrade() {
                if !window.apps_page_active() {
                    return;
                }
                if let Some(filter) = filter.upgrade() {
                    filter.changed(gtk::FilterChange::Different);
                }
            }
        }
    });

    gtk::FilterListModel::new(Some(tree_list_model), Some(filter))
}
