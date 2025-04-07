use adw::prelude::{IsA, ObjectExt};
use gtk::glib;
use std::cmp::Ordering;

use super::{compare_column_entries_by, format_bytes, sort_order, LabelCell};
use crate::label_cell_factory;

pub fn list_item_factory() -> gtk::SignalListItemFactory {
    label_cell_factory!(
        "shared-memory-usage",
        |label: &LabelCell, value: glib::Value| {
            let shared_memory_usage: u64 = value.get().unwrap();
            let formatted = format_bytes(shared_memory_usage as _);
            label.set_label(formatted.as_str());
        }
    )
}

pub fn sorter(column_view: &gtk::ColumnView) -> impl IsA<gtk::Sorter> {
    let column_view = column_view.downgrade();
    gtk::CustomSorter::new(move |lhs, rhs| {
        let Some(column_view) = column_view.upgrade() else {
            return Ordering::Equal.into();
        };

        compare_column_entries_by(lhs, rhs, sort_order(&column_view), |lhs, rhs| {
            let lhs = if let Some(merged_stats) = lhs.merged_stats() {
                merged_stats.shared_memory_usage
            } else {
                lhs.shared_memory_usage()
            };
            let rhs = if let Some(merged_stats) = rhs.merged_stats() {
                merged_stats.shared_memory_usage
            } else {
                rhs.shared_memory_usage()
            };

            lhs.cmp(&rhs)
        })
        .into()
    })
}
