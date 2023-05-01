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

use crate::graph_widget::GraphWidget;

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/window.ui")]
    pub struct MissionCenterWindow {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,

        #[template_child]
        pub graph_widget: TemplateChild<GraphWidget>,

        pub system: std::cell::Cell<sysinfo::System>,
    }

    impl Default for MissionCenterWindow {
        fn default() -> Self {
            use sysinfo::{System, SystemExt};

            Self {
                header_bar: Default::default(),
                graph_widget: Default::default(),
                system: std::cell::Cell::new(System::new()),
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
        use gtk::glib::translate::{FromGlibPtrNone, ToGlibPtr};
        use sysinfo::{CpuExt, SystemExt};

        let this: MissionCenterWindow = unsafe {
            glib::Object::new_internal(
                MissionCenterWindow::static_type(),
                &mut [("application", application.into())],
            )
            .downcast()
            .unwrap()
        };

        let system = unsafe { &mut *this.imp().system.as_ptr() };
        system.refresh_cpu();

        // The windows should be destroyed after the main loop has exited so there should not be a
        // need to explicitly remove the timeout source.
        let this_ptr = this.inner.to_glib_none().0;
        Some(glib::source::timeout_add_local(
            std::time::Duration::from_millis(500),
            move || {
                let window =
                    unsafe { gtk::Window::from_glib_none(this_ptr as *mut gtk::ffi::GtkWindow) };
                let this: &MissionCenterWindow = unsafe { window.unsafe_cast_ref() };

                let system = unsafe { &mut *this.imp().system.as_ptr() };
                system.refresh_cpu();

                this.imp()
                    .graph_widget
                    .get()
                    .add_data_point(system.global_cpu_info().cpu_usage());

                Continue(true)
            },
        ));

        this
    }
}
