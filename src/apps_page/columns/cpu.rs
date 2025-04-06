use std::fmt::Write;

use arrayvec::ArrayString;
use gtk::glib;

use super::LabelCell;
use crate::label_cell_factory;

pub fn list_item_factory() -> gtk::SignalListItemFactory {
    label_cell_factory!("cpu_usage", |label: &LabelCell, value: glib::Value| {
        let cpu_usage: f32 = value.get().unwrap();
        let mut buffer = ArrayString::<128>::new();
        let _ = write!(&mut buffer, "{}%", cpu_usage.round() as u32);
        label.set_label(buffer.as_str());
    })
}
