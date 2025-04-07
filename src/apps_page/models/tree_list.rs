use gtk::gio;
use gtk::prelude::*;

use crate::apps_page::row_model::RowModel;

pub fn model(base_model: impl IsA<gio::ListModel>) -> gtk::TreeListModel {
    gtk::TreeListModel::new(base_model, false, true, move |model_entry| {
        let Some(row_model) = model_entry.downcast_ref::<RowModel>() else {
            return None;
        };
        Some(row_model.children().clone().into())
    })
}
