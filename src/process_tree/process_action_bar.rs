use std::fmt::Write;

use adw::glib::{gobject_ffi, Object, ParamSpec, Properties, Value};
use adw::glib::translate::from_glib_full;
use adw::prelude::*;
use gtk::{gio, glib, subclass::prelude::*};

use crate::process_tree::row_model::{ContentType, RowModel};

macro_rules! setup_action {
    ($actions: expr, $this: expr, $action_obj: expr, $magpie_function: ident) => {
        $action_obj.set_enabled(false);
        $action_obj.connect_activate({
            let this = $this.downgrade();
            move |_action, _| {
                let Some(this) = this.upgrade() else {
                    return;
                };
                let this = this.imp();

                let selected_item = this.selected_item.borrow();
                if selected_item.content_type() == ContentType::SectionHeader {
                    return;
                }

                if let Ok(magpie_client) = app!().sys_info() {
                    magpie_client.$magpie_function(selected_item.pid());
                }
            }
        });
        $actions.add_action(&$action_obj);
    };
}

mod imp {
    use super::*;
    use crate::app;
    use crate::process_tree::process_details_dialog::ProcessDetailsDialog;
    use crate::process_tree::service_details_dialog::ServiceDetailsDialog;
    use adw::glib::{g_critical, VariantTy};
    use crate::process_tree::util::calculate_anchor_point;

    #[derive(Properties, gtk::CompositeTemplate)]
    #[properties(wrapper_type = super::ProcessActionBar)]
    #[template(
        resource = "/io/missioncenter/MissionCenter/ui/process_column_view/process_action_bar.ui"
    )]
    pub struct ProcessActionBar {
        #[template_child]
        pub stop_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub force_stop_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub details_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub context_menu: TemplateChild<gtk::PopoverMenu>,

        pub action_stop: gio::SimpleAction,
        pub action_force_stop: gio::SimpleAction,
        pub action_suspend: gio::SimpleAction,
        pub action_continue: gio::SimpleAction,
        pub action_hangup: gio::SimpleAction,
        pub action_interrupt: gio::SimpleAction,
        pub action_user_one: gio::SimpleAction,
        pub action_user_two: gio::SimpleAction,
        pub action_details: gio::SimpleAction,
    }

    impl Default for ProcessActionBar {
        fn default() -> Self {
            Self {
                stop_label: Default::default(),
                force_stop_label: Default::default(),
                details_label: Default::default(),

                context_menu: Default::default(),
                
                action_stop: gio::SimpleAction::new("stop", None),
                action_force_stop: gio::SimpleAction::new("force-stop", None),
                action_suspend: gio::SimpleAction::new("suspend", None),
                action_continue: gio::SimpleAction::new("continue", None),
                action_hangup: gio::SimpleAction::new("hangup", None),
                action_interrupt: gio::SimpleAction::new("interrupt", None),
                action_user_one: gio::SimpleAction::new("user-one", None),
                action_user_two: gio::SimpleAction::new("user-two", None),
                action_details: gio::SimpleAction::new("details", None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProcessActionBar {
        const NAME: &'static str = "ProcessActionBar";
        type Type = super::ProcessActionBar;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProcessActionBar {
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

    impl WidgetImpl for ProcessActionBar {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl BoxImpl for ProcessActionBar {}

    impl ProcessActionBar {
        pub fn configure(&self, imp: &crate::process_tree::column_view_frame::imp::ColumnViewFrame) {
            let this = imp.obj();

            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("apps-page", Some(&actions));

            let action = gio::SimpleAction::new("show-context-menu", Some(VariantTy::TUPLE));
            action.connect_activate({
                let this = this.downgrade();
                let slef = self.downgrade();
                move |_action, entry| {
                    let Some(slef) = slef.upgrade() else {
                        return;
                    };

                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let imp = this.imp();

                    let Some(model) = imp.column_view.model() else {
                        g_critical!(
                            "MissionCenter::ServicesPage",
                            "Failed to get model for `show-context-menu` action"
                        );
                        return;
                    };

                    let Some((id, anchor_widget, x, y)) =
                        entry.and_then(|s| s.get::<(String, u64, f64, f64)>())
                    else {
                        g_critical!(
                            "MissionCenter::ServicesPage",
                            "Failed to get service name and button from show-context-menu action"
                        );
                        return;
                    };

                    let anchor_widget =
                        upgrade_weak_ptr(anchor_widget as _);
                    let (anchor, show_arrow) = calculate_anchor_point(
                        &this,
                        &anchor_widget,
                        x,
                        y,
                    );

                    if select_item(&model, &id) {
                        match imp.selected_item.borrow().content_type() {
                            // should never trigger
                            ContentType::SectionHeader => {}
                            ContentType::Service => {
                                slef.context_menu.set_pointing_to(Some(&anchor));
                                slef.context_menu.popup();
                            }
                            ContentType::Process | ContentType::App => {}
                        }
                    }
                }
            });
            actions.add_action(&action);

            setup_action!(actions, this, self.action_stop, terminate_process);
            setup_action!(actions, this, self.action_force_stop, kill_process);
            setup_action!(actions, this, self.action_suspend, suspend_process);
            setup_action!(actions, this, self.action_continue, continue_process);
            setup_action!(actions, this, self.action_hangup, hangup_process);
            setup_action!(actions, this, self.action_interrupt, interrupt_process);
            setup_action!(actions, this, self.action_user_one, user_signal_one_process);
            setup_action!(actions, this, self.action_user_two, user_signal_two_process);

            self.action_details.set_enabled(false);
            self.action_details.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    let Some(this) = this.upgrade() else {
                        return;
                    };
                    let imp = this.imp();

                    let selected_item = imp.selected_item.borrow();

                    if selected_item.content_type() == ContentType::Process {
                        ProcessDetailsDialog::new(imp.selected_item.borrow().clone())
                            .present(Some(&this));
                    };
                }
            });
            actions.add_action(&self.action_details);
        }

        pub fn handle_changed_selection(&self, row_model: &RowModel) {
            println!("Updating process bar {:?}", row_model.content_type());
            match row_model.content_type() {
                ContentType::Process | ContentType::App => {
                    self.toggle_actions_enabled(true);
                    self.obj().set_visible(true);
                }
                ContentType::SectionHeader => {
                    self.toggle_actions_enabled(false);
                    self.obj().set_visible(true);
                }
                ContentType::Service => {
                    self.toggle_actions_enabled(false);
                    self.obj().set_visible(false);
                }
            }
        }

        fn toggle_actions_enabled(&self, enabled: bool) {
            println!("Togglear {}", enabled);
            self.action_stop.set_enabled(enabled);
            self.action_force_stop.set_enabled(enabled);
            self.action_suspend.set_enabled(enabled);
            self.action_continue.set_enabled(enabled);
            self.action_hangup.set_enabled(enabled);
            self.action_interrupt.set_enabled(enabled);
            self.action_user_one.set_enabled(enabled);
            self.action_user_two.set_enabled(enabled);
            self.action_details.set_enabled(enabled);
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
    pub struct ProcessActionBar(ObjectSubclass<imp::ProcessActionBar>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

fn select_item(model: &gtk::SelectionModel, id: &str) -> bool {
    for i in 0..model.n_items() {
        if let Some(item) = model
            .item(i)
            .and_then(|i| i.downcast::<gtk::TreeListRow>().ok())
            .and_then(|row| row.item())
            .and_then(|obj| obj.downcast::<RowModel>().ok())
        {
            if item.content_type() != ContentType::SectionHeader && item.id() == id {
                model.select_item(i, false);
                return true;
            }
        }
    }

    false
}