/* window.rs
 *
 * Copyright 2023 Romeo Calota
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

use adw::subclass::prelude::*;
use gtk::{gio, glib, prelude::*};

mod imp {
    use crate::{
        graph_widget::GraphWidget, performance_cpu::PerformanceCpu,
        performance_page::PerformancePage,
    };

    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/window.ui")]
    pub struct MissionCenterWindow {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
    }

    impl Default for MissionCenterWindow {
        fn default() -> Self {
            Self {
                header_bar: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MissionCenterWindow {
        const NAME: &'static str = "MissionCenterWindow";
        type Type = super::MissionCenterWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            GraphWidget::ensure_type();
            PerformanceCpu::ensure_type();
            PerformancePage::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MissionCenterWindow {}

    impl WidgetImpl for MissionCenterWindow {}

    impl WindowImpl for MissionCenterWindow {}

    impl ApplicationWindowImpl for MissionCenterWindow {}

    impl AdwApplicationWindowImpl for MissionCenterWindow {}
}

glib::wrapper! {
    pub struct MissionCenterWindow(ObjectSubclass<imp::MissionCenterWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl MissionCenterWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        let this: MissionCenterWindow = unsafe {
            glib::Object::new_internal(
                MissionCenterWindow::static_type(),
                &mut [("application", application.into())],
            )
            .downcast()
            .unwrap()
        };
        this
    }
}
