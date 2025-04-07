use gtk::gio;

use crate::apps_page::row_model::RowModel;

pub fn model(apps_section: &RowModel, processes_section: &RowModel) -> gio::ListStore {
    let model = gio::ListStore::new::<RowModel>();
    model.append(apps_section);
    model.append(processes_section);

    model
}
