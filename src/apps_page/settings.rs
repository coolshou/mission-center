use glib::g_critical;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::apps_page::imp::AppsPage as AppsPageImpl;
use crate::settings;

pub fn configure(imp: &AppsPageImpl) {
    let apps_page = imp.obj();

    let settings = settings!();

    settings
        .bind(
            "apps-page-show-column-separators",
            &*apps_page,
            "show-column-separators",
        )
        .build();

    imp.use_merged_stats
        .set(settings.boolean("apps-page-merged-process-stats"));
    settings.connect_changed(Some("apps-page-merged-process-stats"), {
        let this = apps_page.downgrade();
        move |settings, _| {
            if let Some(this) = this.upgrade() {
                this.imp()
                    .use_merged_stats
                    .set(settings.boolean("apps-page-merged-process-stats"));
            }
        }
    });

    configure_sorting(imp, &settings);
}

fn configure_sorting(imp: &AppsPageImpl, settings: &gio::Settings) {
    if !settings.boolean("apps-page-remember-sorting") {
        let _ = settings.set_string("apps-page-sorting-column-name", "");
        let _ = settings.set_enum("apps-page-sorting-order", gtk::ffi::GTK_SORT_ASCENDING);
        return;
    }

    let column = settings.string("apps-page-sorting-column-name");
    let order = settings.enum_("apps-page-sorting-order");
    let column = match column.as_str() {
        "name" => &imp.name_column,
        "pid" => &imp.pid_column,
        "cpu" => &imp.cpu_column,
        "memory" => &imp.memory_column,
        "shared_memory" => &imp.shared_memory_column,
        "drive" => &imp.drive_column,
        "gpu" => &imp.gpu_usage_column,
        "gpu_memory" => &imp.gpu_memory_column,
        _ => {
            return;
        }
    };
    let order = match order {
        gtk::ffi::GTK_SORT_ASCENDING => gtk::SortType::Ascending,
        gtk::ffi::GTK_SORT_DESCENDING => gtk::SortType::Descending,
        255 => return,
        _ => {
            g_critical!(
                "MissionCenter::AppsPage",
                "Unknown column sorting order retrieved from settings, sorting in ascending order as a fallback"
            );
            gtk::SortType::Ascending
        }
    };
    imp.column_view.sort_by_column(Some(column), order);
}
