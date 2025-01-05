/* apps_page/view_model.rs
 *
 * Copyright 2024 Mission Center Devs
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::app;
use crate::performance_page::disk::PerformancePageDisk;
use adw::prelude::AdwDialogExt;
use gtk::glib::g_critical;
use gtk::prelude::{ButtonExt, WidgetExt};
use gtk::subclass::prelude::WidgetImpl;
use gtk::subclass::widget::WidgetClassExt;
use gtk::TemplateChild;
use gtk::{glib, glib::subclass::prelude::*};
use std::cell::Cell;

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(
        resource = "/io/missioncenter/MissionCenter/ui/performance_page/disk_eject_failure_dialog.ui"
    )]
    pub struct EjectFailureRow {
        #[template_child]
        pub icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub pid: TemplateChild<gtk::Label>,
        #[template_child]
        pub name: TemplateChild<gtk::Label>,
        #[template_child]
        pub open_files: TemplateChild<gtk::Label>,
        #[template_child]
        pub kill: TemplateChild<gtk::Button>,
        #[template_child]
        pub row_entry: TemplateChild<gtk::ListBoxRow>,

        pub raw_pid: Cell<Option<u32>>,
    }

    impl EjectFailureRow {
        pub fn set_icon(&self, icon: &str) {
            let icon_path = std::path::Path::new(icon);
            if icon_path.exists() {
                self.icon.get().set_from_file(Some(&icon_path));
                return;
            }

            let display = gtk::gdk::Display::default().unwrap();
            let icon_theme = gtk::IconTheme::for_display(&display);

            if icon_theme.has_icon(icon) {
                self.icon.get().set_icon_name(Some(icon));
            } else {
                self.icon
                    .get()
                    .set_icon_name(Some("application-x-executable"));
            }
        }
    }

    impl Default for EjectFailureRow {
        fn default() -> Self {
            Self {
                icon: Default::default(),
                name: Default::default(),
                pid: Default::default(),
                open_files: Default::default(),
                kill: Default::default(),
                row_entry: Default::default(),
                raw_pid: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EjectFailureRow {
        const NAME: &'static str = "EjectFailureRow";
        type Type = super::EjectFailureRow;
    }

    impl ObjectImpl for EjectFailureRow {
        fn constructed(&self) {
            self.parent_constructed();

            let kill_button = self.kill.get();
            kill_button.add_css_class("destructive-action");
        }
    }

    impl WidgetImpl for EjectFailureRow {}
}

pub struct EjectFailureRowBuilder {
    pid: u32,
    icon: glib::GString,
    name: glib::GString,
    id: String,

    parent_page: Option<PerformancePageDisk>,

    files_open: Vec<String>,
}

impl EjectFailureRowBuilder {
    pub fn new() -> Self {
        Self {
            pid: 0,
            icon: "application-x-executable-symbolic".into(),
            name: glib::GString::default(),
            id: String::from(""),

            parent_page: None,
            files_open: vec![],
        }
    }

    pub fn pid(mut self, pid: u32) -> Self {
        self.pid = pid;
        self
    }

    pub fn icon(mut self, icon: &str) -> Self {
        self.icon = icon.into();
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = id.into();
        self
    }

    pub fn files_open(mut self, files_open: Vec<String>) -> Self {
        self.files_open = files_open;
        self
    }

    pub fn parent_page(mut self, parent_page: PerformancePageDisk) -> Self {
        self.parent_page = Some(parent_page);
        self
    }

    pub fn build(self) -> EjectFailureRow {
        let this = EjectFailureRow::new();
        {
            let this = this.imp();

            this.pid.set_label(format!("{}", self.pid).as_str());

            this.name.set_label(self.name.as_str());

            this.set_icon(self.icon.as_str());

            this.open_files
                .set_label(self.files_open.join("\n").as_str());

            this.kill.connect_clicked({
                move |_| {
                    let eject_result = match app!().sys_info() {
                        Ok(sys_info) => sys_info.eject_disk(self.id.as_str(), false, self.pid),
                        Err(err) => {
                            g_critical!(
                                "MissionCenter::EjectFailureRow",
                                "Failed to get sys_info: {}",
                                err
                            );
                            return;
                        }
                    };

                    let Some(parent) = self.parent_page.as_ref() else {
                        g_critical!(
                            "MissionCenter::EjectFailureRow",
                            "Failed to get parent page",
                        );
                        return;
                    };

                    let Some(efd) = parent.eject_failure_dialog() else {
                        g_critical!(
                            "MissionCenter::EjectFailureRow",
                            "Failed to get eject failure dialog",
                        );
                        return;
                    };
                    efd.close();
                    // efd.imp().apply_eject_result(back, parent);
                    // todo this feels leaky
                    parent.imp().show_eject_result(parent, eject_result);
                }
            });
        }

        this
    }
}

glib::wrapper! {
    pub struct EjectFailureRow(ObjectSubclass<imp::EjectFailureRow>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl EjectFailureRow {
    pub fn new() -> Self {
        let this: Self = glib::Object::builder().build();

        this
    }
}
