use std::fmt::Write;

use adw::glib::translate::from_glib_full;
use adw::glib::{gobject_ffi, Object, ParamSpec, Properties, Value};
use adw::prelude::*;
use gtk::{gio, glib, subclass::prelude::*};

use crate::process_tree::row_model::{ContentType, RowModel};

mod imp {
    use super::*;
    use crate::app;
    use crate::process_tree::service_details_dialog::ServiceDetailsDialog;
    use adw::glib::{g_critical, VariantTy};
    use crate::process_tree::process_details_dialog::ProcessDetailsDialog;

    #[derive(Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::ServiceActionBar)]
    #[template(
        resource = "/io/missioncenter/MissionCenter/ui/process_column_view/service_action_bar.ui"
    )]
    pub struct ServiceActionBar {
        #[template_child]
        pub service_start_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub service_stop_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub service_restart_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub service_details_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub service_context_menu: TemplateChild<gtk::PopoverMenu>,

        pub service_start: gio::SimpleAction,
        pub service_stop: gio::SimpleAction,
        pub service_restart: gio::SimpleAction,
        pub service_details: gio::SimpleAction,
    }

    impl Default for ServiceActionBar {
        fn default() -> Self {
            Self {
                service_start_label: Default::default(),
                service_stop_label: Default::default(),
                service_restart_label: Default::default(),
                service_details_label: Default::default(),
                service_context_menu: Default::default(),

                service_start: gio::SimpleAction::new("selected-svc-start", None),
                service_stop: gio::SimpleAction::new("selected-svc-stop", None),
                service_restart: gio::SimpleAction::new("selected-svc-restart", None),
                service_details: gio::SimpleAction::new("details", None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServiceActionBar {
        const NAME: &'static str = "ServiceActionBar";
        type Type = super::ServiceActionBar;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ServiceActionBar {
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
        }
    }

    impl WidgetImpl for ServiceActionBar {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl BoxImpl for ServiceActionBar {}

    impl ServiceActionBar {
        pub fn configure(
            &self,
            imp: &crate::process_tree::column_view_frame::imp::ColumnViewFrame,
        ) {
            let this = imp.obj();

            let actions = gio::SimpleActionGroup::new();
            self.obj()
                .insert_action_group("services-page", Some(&actions));

            self.service_start.set_enabled(false);
            self.service_start.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let imp = this.imp();

                    let selected_item = imp.selected_item.borrow();
                    if selected_item.content_type() != ContentType::Service {
                        return;
                    }

                    if let Ok(magpie_client) = app!().sys_info() {
                        magpie_client.start_service(selected_item.name().to_string());
                    }
                }
            });
            actions.add_action(&self.service_start);

            self.service_stop.set_enabled(false);
            self.service_stop.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let imp = this.imp();

                    let selected_item = imp.selected_item.borrow();
                    if selected_item.content_type() != ContentType::Service {
                        return;
                    }

                    if let Ok(magpie_client) = app!().sys_info() {
                        magpie_client.stop_service(selected_item.name().to_string());
                    }
                }
            });
            actions.add_action(&self.service_stop);

            self.service_restart.set_enabled(false);
            self.service_restart.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let imp = this.imp();

                    let selected_item = imp.selected_item.borrow();
                    if selected_item.content_type() != ContentType::Service {
                        return;
                    }

                    if let Ok(magpie_client) = app!().sys_info() {
                        magpie_client.restart_service(selected_item.name().to_string());
                    }
                }
            });
            actions.add_action(&self.service_restart);

            self.service_details.set_enabled(false);
            self.service_details.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let imp = this.imp();

                    let selected_item = imp.selected_item.borrow();

                    if selected_item.content_type() == ContentType::Service
                    {
                        ServiceDetailsDialog::new(imp.selected_item.borrow().clone())
                            .present(Some(&this));
                    };
                }
            });
            actions.add_action(&self.service_details);
        }

        pub fn handle_changed_selection(&self, row_model: &RowModel) {
            match row_model.content_type() {
                ContentType::Service => {
                    self.obj().set_visible(true);
                    if row_model.service_running() {
                        self.service_stop.set_enabled(true);
                        self.service_start.set_enabled(false);
                        self.service_restart.set_enabled(true);
                    } else {
                        self.service_stop.set_enabled(false);
                        self.service_start.set_enabled(true);
                        self.service_restart.set_enabled(false);
                    }

                    self.service_details.set_enabled(true);
                }
                _ => {
                    self.obj().set_visible(false);
                    self.service_details.set_enabled(false);
                }
            }
        }
    }
}

fn upgrade_weak_ptr(ptr: usize) -> Option<gtk::Widget> {
    let ptr = unsafe { gobject_ffi::g_weak_ref_get(ptr as *mut _) };
    if ptr.is_null() {
        return None;
    }
    let obj: Object = unsafe { from_glib_full(ptr) };
    obj.downcast::<gtk::Widget>().ok()
}

glib::wrapper! {
    pub struct ServiceActionBar(ObjectSubclass<imp::ServiceActionBar>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}
