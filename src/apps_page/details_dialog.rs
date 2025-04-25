/* apps_page/details_dialog.rs
 *
 * Copyright 2025 Mission Center Developers
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

use std::cell::RefCell;

use adw::subclass::prelude::*;
use gtk::glib::{self};
use gtk::prelude::StaticTypeExt;

use super::columns::*;
use super::row_model::{ContentType, RowModel};

mod imp {
    use adw::PreferencesRow;
    use gtk::prelude::WidgetExt;
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/details_dialog.ui")]
    pub struct DetailsDialog {
        #[template_child]
        icon: TemplateChild<gtk::Image>,
        #[template_child]
        title: TemplateChild<gtk::Label>,

        #[template_child]
        id: TemplateChild<gtk::Label>,
        #[template_child]
        kind: TemplateChild<gtk::Label>,
        #[template_child]
        command_line_row: TemplateChild<PreferencesRow>,
        #[template_child]
        command_line: TemplateChild<gtk::Label>,

        #[template_child]
        cpu: TemplateChild<LabelCell>,
        #[template_child]
        memory: TemplateChild<LabelCell>,
        #[template_child]
        shared_memory: TemplateChild<LabelCell>,
        #[template_child]
        drives: TemplateChild<LabelCell>,
        #[template_child]
        gpu: TemplateChild<LabelCell>,
        #[template_child]
        gpu_memory: TemplateChild<LabelCell>,

        pub model: RefCell<RowModel>,
    }

    impl Default for DetailsDialog {
        fn default() -> Self {
            Self {
                icon: TemplateChild::default(),
                title: TemplateChild::default(),

                id: TemplateChild::default(),
                kind: TemplateChild::default(),
                command_line_row: Default::default(),
                command_line: Default::default(),

                cpu: TemplateChild::default(),
                memory: TemplateChild::default(),
                shared_memory: TemplateChild::default(),
                drives: TemplateChild::default(),
                gpu: TemplateChild::default(),
                gpu_memory: TemplateChild::default(),

                model: RefCell::new(RowModel::new(ContentType::SectionHeader)),
            }
        }
    }

    impl DetailsDialog {
        pub fn bind(&self) {
            let model = self.model.borrow();

            if model.content_type() == ContentType::App {
                self.icon.set_pixel_size(24);
            } else {
                self.icon.set_pixel_size(16);
            }

            let icon = model.icon();
            let icon_path = std::path::Path::new(icon.as_str());
            if icon_path.exists() {
                self.icon.set_from_file(Some(&icon_path));
                return;
            }

            let display = gtk::gdk::Display::default().unwrap();
            let icon_theme = gtk::IconTheme::for_display(&display);
            if icon_theme.has_icon(&icon) {
                self.icon.set_icon_name(Some(&icon));
            } else {
                self.icon.set_icon_name(None);
            }

            self.title.set_label(&model.name());

            self.id.set_label(&model.id());

            let content_type: String = model.content_type().into();
            self.kind.set_label(&content_type);

            let cli: String = model.command_line().into();
            self.command_line.set_label(&cli);
            
            self.command_line_row.set_visible(!cli.is_empty());

            cpu_label_formatter(&*self.cpu, model.cpu_usage().into());
            self.cpu.bind(&*model, "cpu-usage", cpu_label_formatter);

            memory_label_formatter(&*self.memory, model.memory_usage().into());
            self.memory
                .bind(&*model, "memory-usage", memory_label_formatter);

            shared_memory_label_formatter(&*self.shared_memory, model.shared_memory_usage().into());
            self.shared_memory.bind(
                &*model,
                "shared-memory-usage",
                shared_memory_label_formatter,
            );

            drive_label_formatter(&*self.drives, model.disk_usage().into());
            self.drives
                .bind(&*model, "disk-usage", drive_label_formatter);

            gpu_label_formatter(&*self.gpu, model.gpu_usage().into());
            self.gpu.bind(&*model, "gpu-usage", gpu_label_formatter);

            gpu_memory_label_formatter(&*self.gpu_memory, model.gpu_memory_usage().into());
            self.gpu_memory
                .bind(&*model, "gpu-memory-usage", gpu_memory_label_formatter);
        }

        fn unbind(&self) {
            self.cpu.unbind();
            self.memory.unbind();
            self.shared_memory.unbind();
            self.drives.unbind();
            self.gpu.unbind();
            self.gpu_memory.unbind();
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DetailsDialog {
        const NAME: &'static str = "AppsPageDetailsDialog";
        type Type = super::DetailsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            LabelCell::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DetailsDialog {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for DetailsDialog {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl AdwDialogImpl for DetailsDialog {
        fn closed(&self) {
            self.unbind();
        }
    }
}

glib::wrapper! {
    pub struct DetailsDialog(ObjectSubclass<imp::DetailsDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DetailsDialog {
    pub fn new(model: RowModel) -> Self {
        let this: Self = glib::Object::builder()
            .property("follows-content-size", true)
            .build();

        let imp = this.imp();

        imp.model.replace(model);
        imp.bind();

        this
    }
}
