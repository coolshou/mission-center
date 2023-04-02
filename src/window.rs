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
use plotters::{drawing::IntoDrawingArea, prelude::*};

use crate::cairo_plotter_backend;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/window.ui")]
    pub struct MissionCenterWindow {
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub drawing_area: TemplateChild<gtk::DrawingArea>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MissionCenterWindow {
        const NAME: &'static str = "MissionCenterWindow";
        type Type = super::MissionCenterWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
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

        this.imp().drawing_area.set_draw_func(|_, cr, w, h| {
            let backend = cairo_plotter_backend::CairoBackend::new(cr, (w as u32, h as u32)).unwrap();
            let root = IntoDrawingArea::into_drawing_area(backend);

            let draw_func = move || -> Result<(), Box<dyn std::error::Error>> {
                // After this point, we should be able to construct a chart context
                let mut chart = ChartBuilder::on(&root)
                    // Set the caption of the chart
                    .caption("This is our first plot", ("sans-serif", 40).into_font())
                    // Set the size of the label region
                    .x_label_area_size(20)
                    .y_label_area_size(40)
                    // Finally attach a coordinate on the drawing area and make a chart context
                    .build_cartesian_2d(0f32..10f32, 0f32..10f32)?;

                // Then we can draw a mesh
                chart
                    .configure_mesh()
                    // We can customize the maximum number of labels allowed for each axis
                    .x_labels(5)
                    .y_labels(5)
                    // We can also change the format of the label text
                    .y_label_formatter(&|x| format!("{:.3}", x))
                    .draw()?;

                // And we can draw something in the drawing area
                chart.draw_series(LineSeries::new(
                    vec![(0.0, 0.0), (5.0, 5.0), (8.0, 7.0)],
                    &RED,
                ))?;
                // Similarly, we can draw point series
                chart.draw_series(PointSeries::of_element(
                    vec![(0.0, 0.0), (5.0, 5.0), (8.0, 7.0)],
                    5,
                    &RED,
                    &|c, s, st| {
                        return EmptyElement::at(c)    // We want to construct a composed element on-the-fly
                            + Circle::new((0, 0), s, st.filled()) // At this point, the new pixel coordinate is established
                            + Text::new(format!("{:?}", c), (10, 0), ("sans-serif", 10).into_font());
                    },
                ))?;
                root.present()?;

                Ok(())
            };
            draw_func().unwrap();
        });

        this
    }
}
