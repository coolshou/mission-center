use adw::prelude::{IsA, ObjectExt};
use gtk::glib;
use std::cmp::Ordering;

use super::{compare_column_entries_by, sort_order, LabelCell};
use crate::label_cell_factory;

pub fn list_item_factory() -> gtk::SignalListItemFactory {
    label_cell_factory!("pid", |label: &LabelCell, value: glib::Value| {
        let pid: u32 = value.get().unwrap();
        label.set_label(&pid.to_string());
    })
}

pub fn sorter(column_view: &gtk::ColumnView) -> impl IsA<gtk::Sorter> {
    let column_view = column_view.downgrade();
    gtk::CustomSorter::new(move |lhs, rhs| {
        let Some(column_view) = column_view.upgrade() else {
            return Ordering::Equal.into();
        };

        compare_column_entries_by(lhs, rhs, sort_order(&column_view), |lhs, rhs| {
            lhs.pid().cmp(&rhs.pid())
        })
        .into()
    })
}
