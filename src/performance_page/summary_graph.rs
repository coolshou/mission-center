/* performance_page/summary_graph.rs
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
use glib::{ParamSpec, Properties, Value};
use gtk::{gdk, glib, Ordering, prelude::*};

use super::widgets::GraphWidget;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::SummaryGraph)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/summary_graph.ui")]
    #[allow(dead_code)]
    pub struct SummaryGraph {
        #[template_child]
        pub graph_widget: TemplateChild<GraphWidget>,
        #[template_child]
        label_heading: TemplateChild<gtk::Label>,
        #[template_child]
        label_info1: TemplateChild<gtk::Label>,
        #[template_child]
        label_info2: TemplateChild<gtk::Label>,

        #[property(get = Self::base_color, set = Self::set_base_color, type = gdk::RGBA)]
        base_color: [u8; 0],
        #[property(get = Self::heading, set = Self::set_heading, type = String)]
        heading: [u8; 0],
        #[property(get = Self::info1, set = Self::set_info1, type = String)]
        info1: [u8; 0],
        #[property(get = Self::info2, set = Self::set_info2, type = String)]
        info2: [u8; 0],
    }

    impl Default for SummaryGraph {
        fn default() -> Self {
            Self {
                graph_widget: Default::default(),
                label_heading: Default::default(),
                label_info1: Default::default(),
                label_info2: Default::default(),

                base_color: [0; 0],
                heading: [0; 0],
                info1: [0; 0],
                info2: [0; 0],
            }
        }
    }

    impl SummaryGraph {
        fn base_color(&self) -> gdk::RGBA {
            self.graph_widget.base_color()
        }

        fn set_base_color(&self, base_color: gdk::RGBA) {
            self.graph_widget.set_base_color(base_color);
        }

        fn heading(&self) -> String {
            self.label_heading.text().to_string()
        }

        fn set_heading(&self, heading: String) {
            self.label_heading.set_text(&heading);
        }

        fn info1(&self) -> String {
            self.label_info1.text().to_string()
        }

        fn set_info1(&self, info1: String) {
            self.label_info1.set_text(&info1);
            if info1.is_empty() {
                self.label_info1.set_visible(false);
            } else {
                self.label_info1.set_visible(true);
            }
        }

        fn info2(&self) -> String {
            self.label_info2.text().to_string()
        }

        fn set_info2(&self, info2: String) {
            self.label_info2.set_text(&info2);
            if info2.is_empty() {
                self.label_info2.set_visible(false);
            } else {
                self.label_info2.set_visible(true);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SummaryGraph {
        const NAME: &'static str = "SummaryGraph";
        type Type = super::SummaryGraph;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SummaryGraph {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for SummaryGraph {}

    impl BoxImpl for SummaryGraph {}
}

glib::wrapper! {
    pub struct SummaryGraph(ObjectSubclass<imp::SummaryGraph>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Buildable;
}

impl SummaryGraph {
    pub fn new() -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(Self::static_type(), &mut [])
                .downcast()
                .unwrap()
        };
        this
    }

    pub fn graph_widget(&self) -> GraphWidget {
        self.imp().graph_widget.clone()
    }

    pub fn set_page_indicies(&self, primary: &usize, secondary: &usize) {
        self.clone().set_page_primary_index(primary);
        self.clone().set_page_secondary_index(secondary);
    }

    pub fn set_page_secondary_index(self, index: &usize) {
        unsafe {
            self.set_data("secondary_index", *index as u32)
        }
    }

    pub fn set_page_primary_index(self, index: &usize) {
        unsafe {
            self.set_data("ordinal", *index as u32)
        }
    }

    pub fn get_primary_ordinal(&self) -> u32 {
        let mut out = 0;
        unsafe {
            let data = self.data::<u32>("ordinal");
            if data.is_some() {
                let null = data.unwrap();
                out = *null.as_ref();
            }
        }

        out
    }

    pub fn get_secondary_ordinal(&self) -> u32 {
        let mut out = 0;
        unsafe {
            let data = self.data::<u32>("secondary_index");
            if data.is_some() {
                let null = data.unwrap();
                out = *null.as_ref();
            }
        }

        out
    }
}

pub fn compare_to(graph1: SummaryGraph, graph2: SummaryGraph) -> Ordering {
    let primary1 = graph1.get_primary_ordinal();
    let primary2 = graph2.get_primary_ordinal();
    if primary1 > primary2 { Ordering::Larger } else if primary1 < primary2 { Ordering::Smaller } else {
        let secondary1 = graph1.get_secondary_ordinal();//widget_name_primary_ordinal(graph1.widget_name());
        let secondary2 = graph2.get_secondary_ordinal();
        if secondary1 > secondary2 { Ordering::Larger } else if secondary1 < secondary2 { Ordering::Smaller } else { Ordering::Equal /* should never get here */ }
    }
}
