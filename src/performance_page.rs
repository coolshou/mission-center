/* performance_page.rs
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

use std::cell::RefCell;

use gtk::{gdk::GLContext, glib, prelude::*, subclass::prelude::*};
use skia_safe::{gpu, gpu::gl, *};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/performance_page.ui")]
    pub struct PerformancePage {
        pub context: RefCell<Option<gpu::DirectContext>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePage {
        const NAME: &'static str = "PerformancePage";
        type Type = super::PerformancePage;
        type ParentType = gtk::GLArea;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePage {}

    impl WidgetImpl for PerformancePage {
        fn realize(&self) {
            self.parent_realize();

            let binding = self.obj();
            let this = binding.upcast_ref::<gtk::GLArea>();

            this.make_current();

            this.set_has_stencil_buffer(true);
            this.set_has_depth_buffer(false);

            let this = binding.upcast_ref::<super::PerformancePage>();
            let interface = gl::Interface::new_native();
            *this.imp().context.borrow_mut() = gpu::DirectContext::new_gl(interface, None);
        }
    }

    impl GLAreaImpl for PerformancePage {
        fn create_context(&self) -> Option<GLContext> {
            if let Some(context) = self.parent_create_context() {
                // context.make_current();
                // context.set_use_es(1);
                Some(context)
            } else {
                None
            }
        }

        fn render(&self, context: &GLContext) -> bool {
            context.make_current();
            let fb_info = {
                gl::FramebufferInfo {
                    fboid: 0, //fboid.try_into().unwrap(),
                    format: gl::Format::RGBA8.into(),
                }
            };

            let render_target = gpu::BackendRenderTarget::new_gl((300, 200), 0, 8, fb_info);

            let mut binding = self.context.borrow_mut();
            let context = (&mut *binding).as_mut().unwrap();

            let mut surface = Surface::from_backend_render_target(
                context,
                &render_target,
                gpu::SurfaceOrigin::BottomLeft,
                ColorType::RGBA8888,
                None,
                None,
            )
            .unwrap();

            surface.canvas().clear(Color::WHITE);
            context.flush_and_submit();

            true
        }
    }
}

glib::wrapper! {
    pub struct PerformancePage(ObjectSubclass<imp::PerformancePage>)
        @extends gtk::Widget, gtk::GLArea,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl PerformancePage {
    pub fn new() -> Self {
        let this: PerformancePage = unsafe {
            glib::Object::new_internal(PerformancePage::static_type(), &mut [])
                .downcast()
                .unwrap()
        };
        this
    }
}
