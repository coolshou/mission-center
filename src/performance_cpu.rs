/* performance_cpu.rs
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
use gtk::{gio, glib};

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/performance_cpu.ui")]
    pub struct PerformanceCpu {}

    impl Default for PerformanceCpu {
        fn default() -> Self {
            Self {}
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformanceCpu {
        const NAME: &'static str = "PerformanceCpu";
        type Type = super::PerformanceCpu;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformanceCpu {}

    impl WidgetImpl for PerformanceCpu {}

    impl BoxImpl for PerformanceCpu {}
}

glib::wrapper! {
    pub struct PerformanceCpu(ObjectSubclass<imp::PerformanceCpu>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}
