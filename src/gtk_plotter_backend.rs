use std::fmt::{Debug, Display, Formatter};

use gtk::{cairo, gdk::RGBA, graphene::Rect, prelude::*, Snapshot};
use plotters_backend::*;

type FontSlant = cairo::FontSlant;
type FontWeight = cairo::FontWeight;

pub struct GtkSnapshotBackend<'a> {
    snapshot: &'a Snapshot,
    width: u32,
    height: u32,
}

impl<'a> GtkSnapshotBackend<'a> {
    pub fn new(snapshot: &'a Snapshot, width: i32, height: i32) -> Self {
        Self {
            snapshot,
            width: width as u32,
            height: height as u32,
        }
    }

    fn set_color(&self, context: &cairo::Context, color: &BackendColor) {
        context.set_source_rgba(
            f64::from(color.rgb.0) / 255.0,
            f64::from(color.rgb.1) / 255.0,
            f64::from(color.rgb.2) / 255.0,
            color.alpha,
        );
    }

    fn set_stroke_width(&self, context: &cairo::Context, width: u32) {
        context.set_line_width(f64::from(width));
    }

    fn set_font<S: BackendTextStyle>(&self, context: &cairo::Context, font: &S) {
        match font.style() {
            FontStyle::Normal => context.select_font_face(
                font.family().as_str(),
                FontSlant::Normal,
                FontWeight::Normal,
            ),
            FontStyle::Bold => context.select_font_face(
                font.family().as_str(),
                FontSlant::Normal,
                FontWeight::Bold,
            ),
            FontStyle::Oblique => context.select_font_face(
                font.family().as_str(),
                FontSlant::Oblique,
                FontWeight::Normal,
            ),
            FontStyle::Italic => context.select_font_face(
                font.family().as_str(),
                FontSlant::Italic,
                FontWeight::Normal,
            ),
        };
        context.set_font_size(font.size());
    }
}

pub struct GtkSnapshotBackendError;

impl Debug for GtkSnapshotBackendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("GtkSnapshotBackendError")
    }
}

impl Display for GtkSnapshotBackendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("GtkSnapshotBackendError")
    }
}

impl std::error::Error for GtkSnapshotBackendError {}

impl<'a> DrawingBackend for GtkSnapshotBackend<'a> {
    type ErrorType = GtkSnapshotBackendError;

    fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn ensure_prepared(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn present(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn draw_pixel(
        &mut self,
        point: BackendCoord,
        color: BackendColor,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let color = RGBA::new(
            color.rgb.0 as f32 / 256.,
            color.rgb.1 as f32 / 256.,
            color.rgb.2 as f32 / 256.,
            color.alpha as f32,
        );

        self.snapshot
            .append_color(&color, &Rect::new(point.0 as f32, point.1 as f32, 1., 1.));

        Ok(())
    }

    fn draw_line<S: BackendStyle>(
        &mut self,
        from: BackendCoord,
        to: BackendCoord,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let context =
            self.snapshot
                .append_cairo(&Rect::new(0., 0., self.width as f32, self.height as f32));

        self.set_color(&context, &style.color());
        self.set_stroke_width(&context, style.stroke_width());

        context.move_to(f64::from(from.0), f64::from(from.1));
        context.line_to(f64::from(to.0), f64::from(to.1));

        context.stroke().unwrap();

        Ok(())
    }
    //
    // fn draw_rect<S: BackendStyle>(
    //     &mut self,
    //     upper_left: BackendCoord,
    //     bottom_right: BackendCoord,
    //     style: &S,
    //     fill: bool,
    // ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
    //     todo!()
    // }
    //
    // fn draw_path<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
    //     &mut self,
    //     path: I,
    //     style: &S,
    // ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
    //     todo!()
    // }
    //
    // fn draw_circle<S: BackendStyle>(
    //     &mut self,
    //     center: BackendCoord,
    //     radius: u32,
    //     style: &S,
    //     fill: bool,
    // ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
    //     todo!()
    // }
    //
    // fn fill_polygon<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
    //     &mut self,
    //     vert: I,
    //     style: &S,
    // ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
    //     todo!()
    // }
    //
    // fn blit_bitmap(
    //     &mut self,
    //     pos: BackendCoord,
    //     (iw, ih): (u32, u32),
    //     src: &[u8],
    // ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
    //     todo!()
    // }
}
