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

use std::cell::Cell;

use gtk::{gdk::GLContext, glib, prelude::*, subclass::prelude::*};
use skia_safe::{gpu, gpu::gl, *};

mod imp {
    use super::*;

    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/me/kicsyromy/MissionCenter/ui/performance_page.ui")]
    pub struct PerformancePage {
        pub context: Cell<Option<gpu::DirectContext>>,
        pub current_frame: Cell<u32>,
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
            let gl_area = binding.upcast_ref::<gtk::GLArea>();

            gl_area.make_current();

            gl_area.set_has_stencil_buffer(true);
            gl_area.set_has_depth_buffer(false);

            let this = binding.upcast_ref::<super::PerformancePage>();

            let interface = gl::Interface::new_native();
            this.imp()
                .context
                .set(gpu::DirectContext::new_gl(interface, None));
            this.imp().current_frame.set(0);
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

            let framebuffer_info = {
                use gl_rs::{types::*, *};

                let mut fboid: GLint = 0;
                unsafe { GetIntegerv(FRAMEBUFFER_BINDING, &mut fboid) };

                gl::FramebufferInfo {
                    fboid: fboid.try_into().unwrap(),
                    format: gl::Format::RGBA8.into(),
                }
            };

            let render_target =
                gpu::BackendRenderTarget::new_gl((300, 200), 0, 8, framebuffer_info);

            let binding = unsafe { &mut *self.context.as_ptr() };
            if let Some(context) = (&mut *binding).as_mut() {
                let mut surface = Surface::from_backend_render_target(
                    context,
                    &render_target,
                    gpu::SurfaceOrigin::BottomLeft,
                    ColorType::RGBA8888,
                    None,
                    None,
                )
                .unwrap();

                let current_frame = self.current_frame.get();
                dbg!(current_frame);
                self.current_frame.set(current_frame.wrapping_add(1));
                surface
                    .canvas()
                    .clear(Color::new(current_frame | 0xFF000000));
                context.flush_and_submit();
            }

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
            glib::Object::new_internal(
                PerformancePage::static_type(),
                &mut [("auto-render", false.into())],
            )
            .downcast()
            .unwrap()
        };
        this
    }
}
