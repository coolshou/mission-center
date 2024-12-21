/* services_page/details_dialog.rs
 *
 * Copyright 2024 Romeo Calota
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

use std::{cell::Cell, num::NonZeroU32};

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, g_warning, ParamSpec, Properties, SignalHandlerId, Value};

use crate::{app, i18n::*};

mod imp {
    use std::cell::OnceCell;
    use std::collections::HashMap;
    use std::sync::Arc;
    use adw::gio::ListStore;
    use adw::glib::WeakRef;
    use adw::ResponseAppearance;
    use crate::{glib_clone, i18n};
    use crate::performance_page::disk::PerformancePageDisk;
    use crate::performance_page::DiskPage;
    use crate::performance_page::eject_failure_row::{ContentType, EjectFailureRowBuilder, EjectFailureRow};
    use crate::sys_info_v2::{App, EjectResult, Process};
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::SmartDataDialog)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/disk_smart_data_dialog.ui")]
    pub struct SmartDataDialog {
        #[template_child]
        pub power_on: TemplateChild<gtk::Label>,
    }

    impl SmartDataDialog {
        // todo populate self
    }

    impl Default for SmartDataDialog {
        fn default() -> Self {
            Self {
                power_on: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SmartDataDialog {
        const NAME: &'static str = "SmartDataDialog";
        type Type = super::SmartDataDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SmartDataDialog {
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

    impl WidgetImpl for SmartDataDialog {
        fn realize(&self) {
            self.parent_realize();

            // todo init here
        }
    }

    impl AdwDialogImpl for SmartDataDialog {
        fn closed(&self) {

        }
    }
}

glib::wrapper! {
    pub struct SmartDataDialog(ObjectSubclass<imp::SmartDataDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

fn to_signal_id(id: u64) -> SignalHandlerId {
    unsafe { std::mem::transmute(id) }
}

fn from_signal_id(id: SignalHandlerId) -> u64 {
    unsafe { id.as_raw() }
}
