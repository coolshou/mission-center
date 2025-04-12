/* apps_page/columns/mod.rs
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

use std::cmp::Ordering;
use std::fmt::Write;

use arrayvec::ArrayString;
use gtk::glib;
use gtk::prelude::*;

use crate::apps_page::row_model::{ContentType, RowModel, SectionType};
use crate::i18n::i18n;

pub use cpu::label_formatter as cpu_label_formatter;
pub use cpu::list_item_factory as cpu_list_item_factory;
pub use cpu::sorter as cpu_sorter;
pub use drive::label_formatter as drive_label_formatter;
pub use drive::list_item_factory as drive_list_item_factory;
pub use drive::sorter as drive_sorter;
pub use gpu::label_formatter as gpu_label_formatter;
pub use gpu::list_item_factory as gpu_list_item_factory;
pub use gpu::sorter as gpu_sorter;
pub use gpu_memory::label_formatter as gpu_memory_label_formatter;
pub use gpu_memory::list_item_factory as gpu_memory_list_item_factory;
pub use gpu_memory::sorter as gpu_memory_sorter;
pub use label_cell::LabelCell;
pub use memory::label_formatter as memory_label_formatter;
pub use memory::list_item_factory as memory_list_item_factory;
pub use memory::sorter as memory_sorter;
pub use name::list_item_factory as name_list_item_factory;
pub use name::sorter as name_sorter;
pub use name_cell::NameCell;
pub use pid::list_item_factory as pid_list_item_factory;
pub use pid::sorter as pid_sorter;
pub use shared_memory::label_formatter as shared_memory_label_formatter;
pub use shared_memory::list_item_factory as shared_memory_list_item_factory;
pub use shared_memory::sorter as shared_memory_sorter;

mod cpu;
mod drive;
mod gpu;
mod gpu_memory;
mod label_cell;
mod memory;
mod name;
mod name_cell;
mod pid;
mod shared_memory;

#[macro_export]
macro_rules! label_cell_factory {
    ($property: literal, $setter: expr) => {{
        label_cell_factory!($property, ContentType::SectionHeader, $setter)
    }};

    ($property: literal, $skip_content: expr, $setter: expr) => {{
        use gtk::prelude::*;

        use crate::apps_page::row_model::{ContentType, RowModel};

        let factory = gtk::SignalListItemFactory::new();

        factory.connect_setup(|_, list_item| {
            let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                return;
            };

            let label = LabelCell::new();
            let expander = gtk::TreeExpander::new();
            expander.set_child(Some(&label));

            expander.set_hide_expander(true);
            expander.set_indent_for_icon(false);
            expander.set_indent_for_depth(false);
            expander.set_halign(gtk::Align::End);

            list_item.set_child(Some(&expander));

            unsafe {
                list_item.set_data("expander", expander);
                list_item.set_data("label", label);
            }
        });

        factory.connect_bind(move |_, list_item| {
            let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                return;
            };

            let Some(row) = list_item
                .item()
                .and_then(|item| item.downcast::<gtk::TreeListRow>().ok())
            else {
                return;
            };

            let expander = unsafe {
                list_item
                    .data::<gtk::TreeExpander>("expander")
                    .unwrap_unchecked()
                    .as_ref()
            };
            expander.set_list_row(Some(&row));

            let Some(model) = expander
                .item()
                .and_then(|item| item.downcast::<RowModel>().ok())
            else {
                return;
            };

            let label = unsafe {
                list_item
                    .data::<LabelCell>("label")
                    .unwrap_unchecked()
                    .as_ref()
            };

            if model.content_type() == ContentType::SectionHeader
                || model.content_type() == $skip_content
            {
                label.set_label("");
                return;
            }

            let value = model.property_value($property);
            ($setter)(&label, value);

            label.bind(&model, $property, $setter);
        });

        factory.connect_unbind(|_, list_item| {
            let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                return;
            };

            let expander = unsafe {
                list_item
                    .data::<gtk::TreeExpander>("expander")
                    .unwrap_unchecked()
                    .as_ref()
            };
            expander.set_list_row(None);

            let label = unsafe {
                list_item
                    .data::<LabelCell>("label")
                    .unwrap_unchecked()
                    .as_ref()
            };
            label.unbind();
        });

        factory.connect_teardown(|_, list_item| {
            let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
                return;
            };

            unsafe {
                let _ = list_item.steal_data::<gtk::TreeExpander>("expander");
                let _ = list_item.steal_data::<gtk::Label>("label");
            }
        });

        factory
    }};
}

pub fn adjust_view_header_alignment(column_view_titlebar: Option<gtk::Widget>) {
    let mut column_view_title = column_view_titlebar.and_then(|w| w.first_child());
    loop {
        let Some(view_title) = column_view_title.take() else {
            break;
        };
        column_view_title = view_title.next_sibling();

        let Some(container) = view_title.first_child() else {
            continue;
        };

        let Some(label) = container
            .first_child()
            .and_then(|l| l.downcast::<gtk::Label>().ok())
        else {
            continue;
        };

        // The `Name` column should be default aligned
        // The column that contains the context menu button should be default aligned
        if label.label().starts_with(&i18n("Name")) {
            label.set_margin_start(10);
            continue;
        }

        container.set_hexpand(true);
        container.set_width_request(70);
        label.set_halign(gtk::Align::End);
        label.set_justify(gtk::Justification::Right);

        if let Some(arrow) = label.next_sibling() {
            if let Some(container) = container.downcast_ref::<gtk::Box>() {
                container.reorder_child_after(&label, Some(&arrow));
                arrow.set_halign(gtk::Align::Start);
                arrow.set_hexpand(true);
            }
        }
    }
}

pub fn update_column_titles(
    cpu_column: &gtk::ColumnViewColumn,
    memory_column: &gtk::ColumnViewColumn,
    drive_column: &gtk::ColumnViewColumn,
    gpu_usage_column: &gtk::ColumnViewColumn,
    gpu_memory_column: &gtk::ColumnViewColumn,
    readings: &crate::magpie_client::Readings,
) {
    let mut buffer = ArrayString::<128>::new();

    let cpu_usage = readings.cpu.total_usage_percent.round() as u32;
    let _ = write!(&mut buffer, "{}\n{}%", i18n("CPU"), cpu_usage);
    cpu_column.set_title(Some(buffer.as_str()));

    buffer.clear();
    let mut memory_used = readings
        .mem_info
        .mem_total
        .saturating_sub(readings.mem_info.mem_available);
    if memory_used == 0 {
        memory_used = readings.mem_info.mem_total - readings.mem_info.mem_free;
    }
    let memory_usage = memory_used as f32 * 100. / readings.mem_info.mem_total as f32;
    let memory_usage = memory_usage.round() as u32;
    let _ = write!(&mut buffer, "{}\n{}%", i18n("Memory"), memory_usage);
    memory_column.set_title(Some(buffer.as_str()));

    buffer.clear();
    if readings.disks_info.is_empty() {
        let _ = write!(&mut buffer, "{}\n0%", i18n("Drive"));
    } else {
        let mut sum = 0.;
        for disk in &readings.disks_info {
            sum += disk.busy_percent
        }
        let drive_usage = sum / readings.disks_info.len() as f32;
        let drive_usage = drive_usage.round() as u32;
        let _ = write!(&mut buffer, "{}\n{}%", i18n("Drive"), drive_usage);
    }
    drive_column.set_title(Some(buffer.as_str()));

    buffer.clear();
    if readings.gpus.is_empty() {
        let _ = write!(&mut buffer, "{}\n0%", i18n("GPU"));
        gpu_usage_column.set_title(Some(buffer.as_str()));

        buffer.clear();
        let _ = write!(&mut buffer, "{}\n0%", i18n("GPU Memory"));
        gpu_memory_column.set_title(Some(buffer.as_str()));
    } else {
        let mut sum_util = 0.;
        let mut sum_mem_used = 0.;
        let mut sum_mem_total = 0.;
        for gpu in readings.gpus.values() {
            sum_util += gpu.utilization_percent.unwrap_or(0.);
            sum_mem_used += gpu.used_memory.unwrap_or(0) as f32;
            sum_mem_total += gpu.total_memory.unwrap_or(0) as f32;
        }
        let gpu_usage = sum_util / readings.gpus.len() as f32;
        let gpu_usage = gpu_usage.round() as u32;
        let _ = write!(&mut buffer, "{}\n{}%", i18n("GPU"), gpu_usage);
        gpu_usage_column.set_title(Some(buffer.as_str()));

        buffer.clear();
        let gpu_mem_usage = sum_mem_used * 100. / sum_mem_total;
        let gpu_mem_usage = gpu_mem_usage.round() as u32;
        let _ = write!(&mut buffer, "{}\n{}%", i18n("GPU Memory"), gpu_mem_usage);
        gpu_memory_column.set_title(Some(buffer.as_str()));
    }
}

fn format_bytes(bytes: f32) -> ArrayString<128> {
    let mut buffer = ArrayString::<128>::new();

    let (v, unit, _) = crate::to_human_readable(bytes, 1024.);
    if unit.is_empty() {
        let _ = write!(&mut buffer, "{} B", v.round() as u32);
    } else {
        let _ = write!(&mut buffer, "{} {}iB", v.round() as u32, unit);
    }

    buffer
}

fn sort_order(column_view: &gtk::ColumnView) -> gtk::SortType {
    column_view
        .sorter()
        .and_downcast_ref::<gtk::ColumnViewSorter>()
        .and_then(|sorter| Some(sorter.primary_sort_order()))
        .unwrap_or(gtk::SortType::Ascending)
}

fn compare_column_entries_by(
    lhs: &glib::Object,
    rhs: &glib::Object,
    sort_order: gtk::SortType,
    compare_fn: fn(&RowModel, &RowModel) -> Ordering,
) -> Ordering {
    let Some(lhs) = lhs.downcast_ref::<RowModel>() else {
        return Ordering::Equal.into();
    };

    let Some(rhs) = rhs.downcast_ref::<RowModel>() else {
        return Ordering::Equal.into();
    };

    let (ord_less, ord_greater) = if sort_order == gtk::SortType::Ascending {
        (Ordering::Less, Ordering::Greater)
    } else {
        (Ordering::Greater, Ordering::Less)
    };

    if lhs.content_type() == ContentType::SectionHeader {
        if lhs.section_type() == SectionType::Apps {
            return ord_less;
        }

        if lhs.section_type() == SectionType::Processes {
            return if rhs.content_type() == ContentType::Process {
                ord_less
            } else {
                ord_greater
            };
        }
    }

    if rhs.content_type() == ContentType::SectionHeader {
        if rhs.section_type() == SectionType::Apps {
            return ord_greater;
        }

        if rhs.section_type() == SectionType::Processes {
            return if lhs.content_type() == ContentType::Process {
                ord_greater
            } else {
                ord_less
            };
        }
    }

    if lhs.content_type() == ContentType::App {
        if rhs.content_type() == ContentType::App {
            return compare_fn(lhs, rhs);
        }
        if rhs.content_type() == ContentType::Process {
            return ord_less;
        }
    }

    if lhs.content_type() == ContentType::Process {
        if rhs.content_type() == ContentType::Process {
            return compare_fn(lhs, rhs);
        }
        if rhs.content_type() == ContentType::App {
            return ord_greater;
        }
    }

    Ordering::Equal
}
