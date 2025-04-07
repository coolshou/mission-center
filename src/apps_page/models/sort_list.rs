use gtk::gio;
use gtk::prelude::*;

pub fn model(
    filter_list_model: impl IsA<gio::ListModel>,
    column_view: &gtk::ColumnView,
) -> gtk::SortListModel {
    let tree_list_sorter = gtk::TreeListRowSorter::new(column_view.sorter());
    gtk::SortListModel::new(Some(filter_list_model), Some(tree_list_sorter))
}
