use std::fmt::Write;

use arrayvec::ArrayString;
use gtk::glib;

use super::LabelCell;
use crate::label_cell_factory;

pub fn list_item_factory() -> gtk::SignalListItemFactory {
    label_cell_factory!("gpu_usage", |label: &LabelCell, value: glib::Value| {
        let gpu_usage: f32 = value.get().unwrap();
        let mut buffer = ArrayString::<128>::new();
        let _ = write!(&mut buffer, "{}%", gpu_usage.round() as u32);
        label.set_label(&mut buffer.as_str());
    })
}
