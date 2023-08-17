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

use std::cell::Cell;

use adw::subclass::prelude::*;
use gtk::{gio, glib, prelude::*};

use crate::sys_info_v2::Readings;

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
        #[template_child]
        pub loading_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub loading_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub stack: TemplateChild<adw::ViewStack>,

        pub settings: Cell<Option<gio::Settings>>,
    }

    impl Default for MissionCenterWindow {
        fn default() -> Self {
            Self {
                performance_page: TemplateChild::default(),
                apps_page: TemplateChild::default(),
                header_stack: TemplateChild::default(),
                search_entry: TemplateChild::default(),
                search_button: TemplateChild::default(),
                loading_box: TemplateChild::default(),
                loading_spinner: TemplateChild::default(),
                stack: TemplateChild::default(),

                settings: Cell::new(None),
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

            if let Some(app) = crate::MissionCenterApplication::default_instance() {
                self.settings.set(app.settings());
            }

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
                    this.search_entry.select_region(-1, -1);
                } else {
                    this.search_entry.set_text("");
                    this.header_stack.set_visible_child_name("view-switcher");
                }
            }));

            let visible_child_name = self.stack.visible_child_name().unwrap_or("".into());
            if visible_child_name == "performance-page" {
                self.search_button.set_visible(false);
                self.search_entry
                    .set_state_flags(gtk::StateFlags::INSENSITIVE, true);
            }

            let this = self.obj().downgrade();
            self.stack.connect_visible_child_notify(move |stack| {
                let this = this.upgrade();
                if this.is_none() {
                    return;
                }
                let this = this.unwrap();
                let this = this.imp();

                let visible_child_name = stack.visible_child_name().unwrap_or("".into());
                if visible_child_name == "performance-page" {
                    this.search_button.set_visible(false);
                    this.search_entry
                        .set_state_flags(gtk::StateFlags::INSENSITIVE, true);
                } else {
                    this.search_button.set_visible(true);
                    this.search_entry
                        .set_state_flags(gtk::StateFlags::NORMAL, true);
                }

                if let Some(settings) = this.settings.take() {
                    settings
                        .set_string("window-selected-page", &visible_child_name)
                        .unwrap_or_else(|_| {
                            g_critical!(
                                "MissionCenter",
                                "Failed to set window-selected-page setting"
                            );
                        });
                    this.settings.set(Some(settings));
                }
            });

            self.search_entry
                .set_key_capture_widget(Some(self.obj().upcast_ref::<gtk::Widget>()));

            let this = self.obj().downgrade();
            self.search_entry.connect_search_started(move |_| {
                let this = this.upgrade();
                if let Some(this) = this {
                    let _ = WidgetExt::activate_action(&this, "win.toggle-search", None);
                }
            });

            let this = self.obj().downgrade();
            self.search_entry.connect_stop_search(move |_| {
                let this = this.upgrade();
                if let Some(this) = this {
                    let _ = WidgetExt::activate_action(&this, "win.toggle-search", None);
                }
            });

            self.obj().add_action(&toggle_search);
        }
    }

    impl WidgetImpl for MissionCenterWindow {
        fn realize(&self) {
            self.parent_realize();

            if let Some(settings) = self.settings.take() {
                self.stack
                    .set_visible_child_name(settings.string("window-selected-page").as_str());
                self.settings.set(Some(settings));
            }
        }
    }

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
        sys_info: &crate::sys_info_v2::SysInfoV2,
    ) -> Self {
        use gtk::glib::*;

        let this: Self = Object::builder()
            .property("application", application)
            .build();

        if let Some(settings) = settings {
            sys_info.set_update_speed(settings.int("update-speed").into());
            sys_info.set_merged_process_stats(settings.boolean("apps-page-merged-process-stats"));

            settings.connect_changed(
                Some("update-speed"),
                clone!(@weak this => move |settings, _| {
                    use crate::{application::MissionCenterApplication, sys_info_v2::UpdateSpeed};

                    let update_speed: UpdateSpeed = settings.int("update-speed").into();
                    let app = match MissionCenterApplication::default_instance() {
                        Some(app) => app,
                        None => {
                            g_critical!("MissionCenter", "Failed to get default instance of MissionCenterApplication");
                            return;
                        }
                    };
                    match app.sys_info() {
                        Ok(sys_info) => {
                            sys_info.set_update_speed(update_speed);
                        }
                        Err(e) => {
                            g_critical!("MissionCenter", "Failed to get sys_info from MissionCenterApplication: {}", e);
                        }
                    };
                }),
            );

            settings.connect_changed(Some("apps-page-merged-process-stats"), clone!(@weak this => move |settings, _| {
                use crate::{application::MissionCenterApplication};
                let merged_process_stats = settings.boolean("apps-page-merged-process-stats");

                let app = match MissionCenterApplication::default_instance() {
                    Some(app) => app,
                    None => {
                        g_critical!("MissionCenter", "Failed to get default instance of MissionCenterApplication");
                        return;
                    }
                };

                match app.sys_info() {
                    Ok(sys_info) => {
                        sys_info.set_merged_process_stats(merged_process_stats);
                    }
                    Err(e) => {
                        g_critical!("MissionCenter", "Failed to get sys_info from MissionCenterApplication: {}", e);
                    }
                };
            }));
        }

        this
    }

    pub fn set_initial_readings(&self, mut readings: Readings) {
        use gtk::glib::*;

        let ok = self.imp().performance_page.set_up_pages(&readings);
        if !ok {
            g_critical!(
                "MissionCenter",
                "Failed to set initial readings for performance page"
            );
        }

        let ok = self.imp().apps_page.set_initial_readings(&mut readings);
        if !ok {
            g_critical!(
                "MissionCenter",
                "Failed to set initial readings for apps page"
            );
        }

        self.imp().loading_spinner.set_spinning(false);
        self.imp().loading_box.set_visible(false);
        self.imp().stack.set_visible(true);
    }

    pub fn update_readings(&self, readings: &mut Readings) -> bool {
        let mut result = true;

        result &= self.imp().performance_page.update_readings(readings);
        result &= self.imp().apps_page.update_readings(readings);

        result
    }
}
