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
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/window.ui")]
    pub struct MissionCenterWindow {
        #[template_child]
        pub performance_page: TemplateChild<crate::performance_page::PerformancePage>,
        #[template_child]
        pub apps_page: TemplateChild<crate::apps_page::AppsPage>,
        #[template_child]
        pub header_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_button: TemplateChild<gtk::ToggleButton>,

        pub sys_info: std::cell::Cell<Option<crate::sys_info_v2::SysInfoV2>>,
    }

    impl Default for MissionCenterWindow {
        fn default() -> Self {
            Self {
                performance_page: TemplateChild::default(),
                apps_page: TemplateChild::default(),
                header_stack: TemplateChild::default(),
                search_entry: TemplateChild::default(),
                search_button: TemplateChild::default(),

                sys_info: std::cell::Cell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MissionCenterWindow {
        const NAME: &'static str = "MissionCenterWindow";
        type Type = super::MissionCenterWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            use crate::{apps_page::AppsPage, performance_page::PerformancePage};

            PerformancePage::ensure_type();
            AppsPage::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MissionCenterWindow {
        fn constructed(&self) {
            use glib::*;

            self.parent_constructed();

            let toggle_search =
                gio::SimpleAction::new_stateful("toggle-search", None, false.to_variant());
            toggle_search.connect_activate(clone!(@weak self as this => move |action, _| {
                let current_state = action.state();
                if current_state.is_none() {
                    g_critical!("MissionCenter", "Failed to get search action state");
                    action.set_state(false.to_variant());
                    return;
                }
                let current_state = current_state.unwrap();

                let new_state = !current_state.get::<bool>().unwrap_or(false);
                action.set_state(new_state.to_variant());
                this.search_button.set_active(new_state);

                if new_state {
                    this.header_stack.set_visible_child_name("search-entry");
                    this.search_entry.grab_focus();
                } else {
                    this.header_stack.set_visible_child_name("view-switcher");
                }
            }));

            let escape_controller = gtk::EventControllerKey::new();
            let this = self.obj().clone();
            escape_controller.connect_key_pressed(move |_, key, _, _| {
                if key == gtk::gdk::Key::Escape && this.imp().search_button.is_active() {
                    // TODO: Fix
                    unsafe {
                        gtk::ffi::gtk_widget_activate_action(
                            this.as_ptr() as *mut _,
                            b"win.toggle-search\0".as_ptr() as *const _,
                            std::ptr::null_mut(),
                        );
                    }

                    return signal::Inhibit(true);
                }
                signal::Inhibit(false)
            });
            self.search_entry.add_controller(escape_controller.clone());
            self.obj().add_action(&toggle_search);
        }
    }

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
    pub fn new<P: IsA<gtk::Application>>(
        application: &P,
        settings: Option<&gio::Settings>,
    ) -> Self {
        use gtk::glib::*;

        let this: Self = Object::builder()
            .property("application", application)
            .build();

        let (sys_info, mut initial_readings) = crate::sys_info_v2::SysInfoV2::new();

        let ok = this.imp().performance_page.set_up_pages(&initial_readings);
        if !ok {
            g_critical!(
                "MissionCenter",
                "Failed to set initial readings for performance page"
            );
        }

        let ok = this
            .imp()
            .apps_page
            .set_initial_readings(&mut initial_readings);
        if !ok {
            g_critical!(
                "MissionCenter",
                "Failed to set initial readings for apps page"
            );
        }

        if let Some(settings) = settings {
            sys_info.set_update_speed(settings.int("update-speed").into());

            settings.connect_changed(
                Some("update-speed"),
                clone!(@weak this => move |settings, _| {
                    use crate::sys_info_v2::UpdateSpeed;

                    let update_speed: UpdateSpeed = settings.int("update-speed").into();
                    let sys_info = this.imp().sys_info.take();
                    if sys_info.is_none() {
                        g_critical!("MissionCenter", "SysInfo is not initialized, how is this application still running?");
                    }
                    let sys_info = sys_info.unwrap();

                    sys_info.set_update_speed(update_speed);

                    this.imp().sys_info.set(Some(sys_info));
                }),
            );
        }

        this.imp().sys_info.set(Some(sys_info));

        this
    }

    pub fn update_readings(&self, readings: &mut crate::sys_info_v2::Readings) -> bool {
        let mut result = true;

        result &= self.imp().performance_page.update_readings(readings);
        result &= self.imp().apps_page.update_readings(readings);

        result
    }
}
