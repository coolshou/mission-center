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
use gtk::{
    glib,
    glib::{subclass::prelude::*, ParamSpec, Properties, Value},
};
use std::cell::Cell;

use gtk::prelude::{ButtonExt, WidgetExt};
use gtk::subclass::prelude::WidgetImpl;
use std::cell::OnceCell;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::EjectFailureRow)]
    pub struct EjectFailureRow {
        pub icon: OnceCell<gtk::Image>,
        pub pid: OnceCell<gtk::Label>,
        pub name: OnceCell<gtk::Label>,
        pub open_files: OnceCell<gtk::Label>,
        pub kill: OnceCell<gtk::Button>,
        pub row_entry: OnceCell<gtk::ListBoxRow>,

        pub raw_pid: Cell<Option<u32>>,
    }

    impl EjectFailureRow {
        pub fn set_icon(&self, icon: &str) {
            let icon_path = std::path::Path::new(icon);
            if icon_path.exists() {
                self.icon
                    .get()
                    .expect("Damn")
                    .set_from_file(Some(&icon_path));
                return;
            }

            let display = gtk::gdk::Display::default().unwrap();
            let icon_theme = gtk::IconTheme::for_display(&display);

            if icon_theme.has_icon(icon) {
                self.icon.get().expect("Damn").set_icon_name(Some(icon));
            } else {
                self.icon
                    .get()
                    .expect("Damn")
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
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let sidebar_content_builder = gtk::Builder::from_resource(
                "/io/missioncenter/MissionCenter/ui/performance_page/disk_eject_failure_entry.ui",
            );

            let _ = self.row_entry.set(
                sidebar_content_builder
                    .object::<gtk::ListBoxRow>("root")
                    .expect("Could not find `root` object in details pane"),
            );
            let _ = self.icon.set(
                sidebar_content_builder
                    .object::<gtk::Image>("icon")
                    .expect("Could not find `icon` object in details pane"),
            );
            let _ = self.pid.set(
                sidebar_content_builder
                    .object::<gtk::Label>("pid")
                    .expect("Could not find `pid` object in details pane"),
            );
            let _ = self.name.set(
                sidebar_content_builder
                    .object::<gtk::Label>("name")
                    .expect("Could not find `name` object in details pane"),
            );
            let _ = self.open_files.set(
                sidebar_content_builder
                    .object::<gtk::Label>("open_files")
                    .expect("Could not find `open_files` object in details pane"),
            );
            let _ = self.kill.set(
                sidebar_content_builder
                    .object::<gtk::Button>("kill")
                    .expect("Could not find `kill` object in details pane"),
            );

            let kill_button = self.kill.get().unwrap();
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

            this.raw_pid.set(Some(self.pid));
            this.pid
                .get()
                .expect("Damn")
                .set_label(format!("{}", self.pid).as_str());
            this.name.get().expect("Damn").set_label(self.name.as_str());
            this.set_icon(self.icon.as_str());

            this.open_files
                .get()
                .expect("Damn")
                .set_label(self.files_open.join("\n").as_str());

            this.kill.get().expect("Damn").connect_clicked({
                move |_| {
                    println!("killering {:?}", self.pid);

                    let back = app!()
                        .sys_info()
                        .expect("Failed to get sys_info")
                        .eject_disk(self.id.as_str(), false, self.pid);

                    let parent = self.parent_page.as_ref().unwrap();
                    let efd = parent.eject_failure_dialog().unwrap();
                    efd.close();
                    // efd.imp().apply_eject_result(back, parent);
                    // todo this feels leaky
                    parent.imp().show_eject_result(parent, back);
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
