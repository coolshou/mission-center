/* performance_page/widgets/mem_composition_widget.rs
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

use glib::{ParamSpec, Properties, Value};
use gtk::{gdk, gdk::prelude::*, glib, prelude::*, subclass::prelude::*};

mod imp {
    use pathfinder_gl::GLDevice;
    use pathfinder_renderer::gpu::renderer::Renderer;

    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::MemoryCompositionWidget)]
    pub struct MemoryCompositionWidget {
        #[property(get, set)]
        base_color: Cell<gdk::RGBA>,
        #[property(get, set)]
        range_min: Cell<f32>,
        #[property(get, set)]
        range_max: Cell<f32>,

        pub(crate) mem_info: Cell<crate::sys_info_v2::MemInfo>,

        renderer: Cell<Option<Renderer<GLDevice>>>,
        render_function: Cell<fn(&Self, width: i32, height: i32, scale_factor: f32)>,

        tooltip_texts: Cell<Vec<(f32, String)>>,
    }

    impl Default for MemoryCompositionWidget {
        fn default() -> Self {
            Self {
                base_color: Cell::new(gdk::RGBA::new(0., 0., 0., 1.)),
                range_min: Cell::new(0.0),
                range_max: Cell::new(100.0),

                mem_info: Cell::new(crate::sys_info_v2::MemInfo::default()),

                renderer: Cell::new(None),
                render_function: Cell::new(Self::render_init_pathfinder),

                tooltip_texts: Cell::new(vec![]),
            }
        }
    }

    impl MemoryCompositionWidget {
        #[inline]
        fn render_bar(
            &self,
            canvas: &mut pathfinder_canvas::CanvasRenderingContext2D,
            x: f32,
            width: f32,
            height: i32,
            fill: bool,
        ) -> f32 {
            use pathfinder_canvas::*;

            let mut path = Path2D::new();
            path.move_to(vec2f(x, 0.));
            path.line_to(vec2f(x, height as f32));
            canvas.stroke_path(path);

            if fill {
                canvas.fill_rect(RectF::new(vec2f(x, 0.), vec2f(width, height as f32)));
            }

            x + width
        }

        #[inline]
        fn render_outline(
            &self,
            canvas: &mut pathfinder_canvas::CanvasRenderingContext2D,
            width: i32,
            height: i32,
            scale_factor: f32,
        ) {
            use pathfinder_canvas::*;

            canvas.stroke_rect(RectF::new(
                vec2f(scale_factor / 2., scale_factor / 2.),
                vec2f(width as f32 - scale_factor, height as f32 - scale_factor),
            ));
        }

        fn configure_tooltips(&self) {
            let this = self.obj();
            let this = this.upcast_ref::<super::MemoryCompositionWidget>();

            this.set_has_tooltip(true);
            this.set_tooltip_text(Some("Memory Composition"));
            this.connect_query_tooltip(|this, x, _, keyboard, tooltip| {
                if keyboard {
                    tooltip.set_text(Some("Memory Composition"));
                    return true;
                }

                let tooltip_texts = this.imp().tooltip_texts.take();

                let x = x as f32 * this.scale_factor() as f32;
                let mut text_pos = tooltip_texts.len() - 1;
                for (i, (pos, _)) in tooltip_texts.iter().enumerate().rev() {
                    if x <= *pos {
                        text_pos = i;
                    } else {
                        break;
                    }
                }

                tooltip.set_text(Some(&tooltip_texts[text_pos].1));
                this.imp().tooltip_texts.set(tooltip_texts);

                true
            });
        }

        fn render_init_pathfinder(&self, width: i32, height: i32, scale_factor: f32) {
            use pathfinder_canvas::*;
            use pathfinder_gl::*;
            use pathfinder_renderer::gpu::{options::*, renderer::*};
            use pathfinder_resources::embedded::*;

            let mut fboid: gl::types::GLint = 0;
            unsafe {
                gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid);
            }

            let device = GLDevice::new(GLVersion::GL3, fboid as _);
            let mode = RendererMode::default_for_device(&device);

            let framebuffer_size = Vector2I::new(width, height);
            let options = RendererOptions {
                dest: DestFramebuffer::full_window(framebuffer_size.clone()),
                background_color: None,
                ..RendererOptions::default()
            };

            self.renderer.set(Some(Renderer::new(
                device,
                &EmbeddedResourceLoader::new(),
                mode,
                options,
            )));

            self.render_function.set(Self::render_all);
            self.render_all(width, height, scale_factor);
        }

        fn render_all(&self, width: i32, height: i32, scale_factor: f32) {
            use pathfinder_canvas::*;
            use pathfinder_renderer::{concurrent::*, gpu::options::*, options::*};

            let framebuffer_size = Vector2I::new(width, height);

            let mut renderer = self.renderer.take().expect("Uninitialized renderer");
            renderer.options_mut().dest = DestFramebuffer::full_window(framebuffer_size);

            let this = self.obj();
            let this = this.upcast_ref::<super::MemoryCompositionWidget>();

            let mem_info = self.mem_info.get();

            let total = mem_info.mem_total as f32;

            let mut canvas =
                Canvas::new(framebuffer_size.to_f32()).get_context_2d(CanvasFontContext {});

            let base_color = this.imp().base_color.get();
            canvas.set_fill_style(FillStyle::Color(ColorU::new(
                (base_color.red() * 255.) as u8,
                (base_color.green() * 255.) as u8,
                (base_color.blue() * 255.) as u8,
                50,
            )));
            canvas.set_stroke_style(FillStyle::Color(ColorU::new(
                (base_color.red() * 255.) as u8,
                (base_color.green() * 255.) as u8,
                (base_color.blue() * 255.) as u8,
                255,
            )));
            canvas.set_line_width(scale_factor as f32);

            let mut tooltip_texts = self.tooltip_texts.take();
            tooltip_texts.clear();

            let used = (mem_info.mem_total - mem_info.mem_available) as f32;
            let x = self.render_bar(&mut canvas, 0., width as f32 * (used / total), height, true);
            let used_hr = crate::to_human_readable(used, 1024.);

            let modified = mem_info.dirty as f32;
            let bar_width = width as f32 * (modified / total);
            let x = self.render_bar(&mut canvas, x - bar_width, bar_width, height, false);
            let modified = crate::to_human_readable(modified, 1024.);
            tooltip_texts.push((
                x - bar_width,
                gettextrs::gettext!(
                    "In use ({}iB)\n\nMemory used by the operating system and running applications",
                    format!("{:.2} {}", used_hr.0, used_hr.1)
                ),
            ));
            tooltip_texts.push((
                x,
                gettextrs::gettext!(
                    "Modified ({}iB)\n\nMemory whose contents must be written to disk before it can be used by another process",
                    format!("{:.2} {}", modified.0, modified.1)
               )
            ));

            self.render_bar(&mut canvas, x, 1., height, false);

            let free = mem_info.mem_free as f32;
            let bar_width = width as f32 * (free / total);
            self.render_bar(
                &mut canvas,
                width as f32 - bar_width,
                bar_width,
                height,
                false,
            );

            let standby = crate::to_human_readable(total - (used + free), 1024.);
            tooltip_texts.push((
                width as f32 - bar_width,
                gettextrs::gettext!(
                    "Standby ({}iB)\n\nMemory that contains cached data and code that is not actively in use",
                    format!("{:.2} {}", standby.0, standby.1)
                )
            ));

            let free = crate::to_human_readable(free, 1024.);
            tooltip_texts.push((
                width as f32 + 1.,
                gettextrs::gettext!(
                    "Free ({}iB)\n\nMemory that is not currently in use, and that will be repurposed first when the operating system, drivers, or applications need more memory",
                    format!("{:.2} {}", free.0, free.1)
                ),
            ));

            self.render_outline(&mut canvas, width, height, scale_factor);

            canvas.into_canvas().into_scene().build_and_render(
                &mut renderer,
                BuildOptions::default(),
                executor::SequentialExecutor,
            );

            self.tooltip_texts.set(tooltip_texts);
            self.renderer.set(Some(renderer));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MemoryCompositionWidget {
        const NAME: &'static str = "MemoryCompositionWidget";
        type Type = super::MemoryCompositionWidget;
        type ParentType = gtk::GLArea;
    }

    impl ObjectImpl for MemoryCompositionWidget {
        fn constructed(&self) {
            self.parent_constructed();

            self.configure_tooltips();
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

    impl WidgetImpl for MemoryCompositionWidget {
        fn realize(&self) {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::MemoryCompositionWidget>();

            self.parent_realize();

            this.set_has_stencil_buffer(true);
            this.set_auto_render(true);
        }
    }

    impl GLAreaImpl for MemoryCompositionWidget {
        fn render(&self, _: &gdk::GLContext) -> bool {
            let obj = self.obj();
            let this = obj.upcast_ref::<super::MemoryCompositionWidget>();

            let scale_factor = this.scale_factor();
            let mut viewport_info: [gl::types::GLint; 4] = [0; 4];
            unsafe {
                gl::GetIntegerv(gl::VIEWPORT, &mut viewport_info[0]);
            }
            let width = viewport_info[2];
            let height = viewport_info[3];

            unsafe {
                gl::ClearColor(0.0, 0.0, 0.0, 0.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);
            }

            (self.render_function.get())(self, width, height, scale_factor as f32);

            true
        }
    }
}

glib::wrapper! {
    pub struct MemoryCompositionWidget(ObjectSubclass<imp::MemoryCompositionWidget>)
        @extends gtk::GLArea, gtk::Widget,
        @implements gtk::Buildable;
}

impl MemoryCompositionWidget {
    pub fn new() -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(MemoryCompositionWidget::static_type(), &mut [])
                .downcast()
                .unwrap()
        };
        this
    }

    pub fn update_memory_information(&self, mem_info: &crate::sys_info_v2::MemInfo) {
        self.imp().mem_info.set(mem_info.clone());
        self.queue_render();
    }
}
