use gtk::glib;

use super::LabelCell;
use crate::label_cell_factory;

pub fn list_item_factory() -> gtk::SignalListItemFactory {
    label_cell_factory!("pid", |label: &LabelCell, value: glib::Value| {
        let pid: u32 = value.get().unwrap();
        label.set_label(&pid.to_string());
    })
}
