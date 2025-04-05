use adw::prelude::{Cast, ObjectExt};
use gtk::glib;
use gtk::prelude::{ListItemExt, WidgetExt};

use crate::apps_page::row_model::{ContentType, RowModel};

use super::format_bytes;

pub fn list_item_factory() -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(|_, list_item| {
        let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
            return;
        };

        let label = gtk::Label::new(None);
        let expander = gtk::TreeExpander::new();
        expander.set_child(Some(&label));

        expander.set_hide_expander(true);
        expander.set_indent_for_icon(false);
        expander.set_indent_for_depth(false);
        expander.set_halign(gtk::Align::End);

        list_item.set_child(Some(&expander));

        unsafe {
            list_item.set_data("expander", expander);
            list_item.set_data("label", label);
        }
    });

    factory.connect_bind(|_, list_item| {
        let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
            return;
        };

        let Some(row) = list_item
            .item()
            .and_then(|item| item.downcast::<gtk::TreeListRow>().ok())
        else {
            return;
        };

        let expander = unsafe {
            list_item
                .data::<gtk::TreeExpander>("expander")
                .unwrap_unchecked()
                .as_ref()
        };
        expander.set_list_row(Some(&row));

        let label = unsafe {
            list_item
                .data::<gtk::Label>("label")
                .unwrap_unchecked()
                .as_ref()
        };

        let Some(model) = expander
            .item()
            .and_then(|item| item.downcast::<RowModel>().ok())
        else {
            return;
        };

        if model.content_type() == ContentType::SectionHeader {
            return;
        }

        let sig_handler = model.connect_gpu_memory_usage_notify({
            let label = label.clone();
            move |model| {
                let formatted = format_bytes(model.gpu_memory_usage() as _);
                label.set_label(formatted.as_str());
            }
        });
        let formatted = format_bytes(model.gpu_memory_usage() as _);
        label.set_label(formatted.as_str());

        unsafe {
            label.set_data("sig_handler", sig_handler);
        }
    });

    factory.connect_unbind(|_, list_item| {
        let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
            return;
        };

        let expander = unsafe {
            list_item
                .data::<gtk::TreeExpander>("expander")
                .unwrap_unchecked()
                .as_ref()
        };
        expander.set_list_row(None);

        let label = unsafe {
            list_item
                .data::<gtk::Label>("label")
                .unwrap_unchecked()
                .as_ref()
        };
        label.set_label("");

        if let Some(sig_handler) =
            unsafe { list_item.steal_data::<glib::SignalHandlerId>("sig_handler") }
        {
            let Some(model) = expander
                .item()
                .and_then(|item| item.downcast::<RowModel>().ok())
            else {
                return;
            };
            model.disconnect(sig_handler);
        }
    });

    factory.connect_teardown(|_, list_item| {
        let Some(list_item) = list_item.downcast_ref::<gtk::ListItem>() else {
            return;
        };

        unsafe {
            let _ = list_item.steal_data::<gtk::TreeExpander>("expander");
            let _ = list_item.steal_data::<gtk::Label>("label");
        }
    });

    factory
}
