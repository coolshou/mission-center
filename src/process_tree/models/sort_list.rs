/* neo_services_page/models/sort_list.rs
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

use crate::settings;

pub fn model(
    filter_list_model: impl IsA<gio::ListModel>,
    column_view: &gtk::ColumnView,
) -> (gtk::SortListModel, gtk::TreeListRowSorter) {
    let column_view_sorter = column_view.sorter();

    if let Some(column_view_sorter) = column_view_sorter.as_ref() {
        column_view_sorter.connect_changed({
            |sorter, _| {
                let settings = settings!();

                let Some(sorter) = sorter.downcast_ref::<gtk::ColumnViewSorter>() else {
                    return;
                };

                let Some(sorted_column) = sorter.primary_sort_column() else {
                    return;
                };

                let Some(sorted_column_id) = sorted_column.id() else {
                    return;
                };
                let _ =
                    settings.set_string("apps-page-sorting-column-name", sorted_column_id.as_str());

                let sort_order = sorter.primary_sort_order();
                let _ = settings.set_enum(
                    "apps-page-sorting-order",
                    match sort_order {
                        gtk::SortType::Ascending => gtk::ffi::GTK_SORT_ASCENDING,
                        gtk::SortType::Descending => gtk::ffi::GTK_SORT_DESCENDING,
                        _ => gtk::ffi::GTK_SORT_ASCENDING,
                    },
                );
            }
        });
    }

    let tree_list_sorter = gtk::TreeListRowSorter::new(column_view_sorter);
    (
        gtk::SortListModel::new(Some(filter_list_model), Some(tree_list_sorter.clone())),
        tree_list_sorter,
    )
}
