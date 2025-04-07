use gtk::glib;

use super::{format_bytes, LabelCell};
use crate::label_cell_factory;

pub fn list_item_factory() -> gtk::SignalListItemFactory {
    label_cell_factory!("disk-usage", |label: &LabelCell, value: glib::Value| {
        let disk_usage: f32 = value.get().unwrap();
        let mut formatted = format_bytes(disk_usage);
        formatted.push_str("/s");
        label.set_label(formatted.as_str());
    })
}
