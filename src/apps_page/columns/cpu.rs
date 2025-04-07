use adw::prelude::{IsA, ObjectExt};
use arrayvec::ArrayString;
use gtk::glib;
use std::cmp::Ordering;
use std::fmt::Write;

use super::{compare_column_entries_by, sort_order, LabelCell};
use crate::label_cell_factory;

pub fn list_item_factory() -> gtk::SignalListItemFactory {
    label_cell_factory!("cpu-usage", |label: &LabelCell, value: glib::Value| {
        let cpu_usage: f32 = value.get().unwrap();
        let mut buffer = ArrayString::<128>::new();
        let _ = write!(&mut buffer, "{}%", cpu_usage.round() as u32);
        label.set_label(buffer.as_str());
    })
}

pub fn sorter(column_view: &gtk::ColumnView) -> impl IsA<gtk::Sorter> {
    let column_view = column_view.downgrade();
    gtk::CustomSorter::new(move |lhs, rhs| {
        let Some(column_view) = column_view.upgrade() else {
            return Ordering::Equal.into();
        };

        compare_column_entries_by(lhs, rhs, sort_order(&column_view), |lhs, rhs| {
            let lhs = if let Some(merged_stats) = lhs.merged_stats() {
                merged_stats.cpu_usage
            } else {
                lhs.cpu_usage()
            };
            let rhs = if let Some(merged_stats) = rhs.merged_stats() {
                merged_stats.cpu_usage
            } else {
                rhs.cpu_usage()
            };

            lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
        })
        .into()
    })
}
