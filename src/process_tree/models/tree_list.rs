/* neo_services_page/models/tree_list.rs
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

use crate::process_tree::row_model::RowModel;

pub fn model(base_model: impl IsA<gio::ListModel>) -> gtk::TreeListModel {
    gtk::TreeListModel::new(base_model, false, true, move |model_entry| {
        let Some(row_model) = model_entry.downcast_ref::<RowModel>() else {
            return None;
        };
        Some(row_model.children().clone().into())
    })
}
