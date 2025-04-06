use gtk::glib;

use super::{format_bytes, LabelCell};
use crate::label_cell_factory;

pub fn list_item_factory() -> gtk::SignalListItemFactory {
    label_cell_factory!(
        "gpu_memory_usage",
        |label: &LabelCell, value: glib::Value| {
            let gpu_memory_usage: u64 = value.get().unwrap();
            let formatted = format_bytes(gpu_memory_usage as _);
            label.set_label(formatted.as_str());
        }
    )
}
