use gtk::cairo;
use plotters::{
    prelude::*,
    style::text_anchor::{HPos, VPos},
};
use plotters_backend::*;

pub type CairoContext = cairo::Context;

type FontSlant = cairo::FontSlant;
type FontWeight = cairo::FontWeight;

/// The drawing backend that is backed with a Cairo context
pub struct CairoBackend<'a> {
    context: &'a CairoContext,
    width: u32,
    height: u32,
    init_flag: bool,
}

#[derive(Debug)]
pub struct CairoError;

impl std::fmt::Display for CairoError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{:?}", self)
    }
}

impl std::error::Error for CairoError {}

impl<'a> CairoBackend<'a> {
    fn set_color(&self, color: &BackendColor) {
        self.context.set_source_rgba(
            f64::from(color.rgb.0) / 255.0,
            f64::from(color.rgb.1) / 255.0,
            f64::from(color.rgb.2) / 255.0,
            color.alpha,
        );
    }

    fn set_stroke_width(&self, width: u32) {
        self.context.set_line_width(f64::from(width));
    }

    fn set_font<S: BackendTextStyle>(&self, font: &S) {
        match font.style() {
            FontStyle::Normal => self.context.select_font_face(
                font.family().as_str(),
                FontSlant::Normal,
                FontWeight::Normal,
            ),
            FontStyle::Bold => self.context.select_font_face(
                font.family().as_str(),
                FontSlant::Normal,
                FontWeight::Bold,
            ),
            FontStyle::Oblique => self.context.select_font_face(
                font.family().as_str(),
                FontSlant::Oblique,
                FontWeight::Normal,
            ),
            FontStyle::Italic => self.context.select_font_face(
                font.family().as_str(),
                FontSlant::Italic,
                FontWeight::Normal,
            ),
        };
        self.context.set_font_size(font.size());
    }

    pub fn new(context: &'a CairoContext, (w, h): (u32, u32)) -> Result<Self, CairoError> {
        Ok(Self {
            context,
            width: w,
            height: h,
            init_flag: false,
        })
    }
}

impl<'a> DrawingBackend for CairoBackend<'a> {
    type ErrorType = cairo::Error;

    fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn ensure_prepared(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        if !self.init_flag {
            let (x0, y0, x1, y1) = self
                .context
                .clip_extents()
                .map_err(DrawingErrorKind::DrawingError)?;

            self.context.scale(
                (x1 - x0) / f64::from(self.width),
                (y1 - y0) / f64::from(self.height),
            );

            self.init_flag = true;
        }

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
        self.context
            .rectangle(f64::from(point.0), f64::from(point.1), 1.0, 1.0);
        self.context.set_source_rgba(
            f64::from(color.rgb.0) / 255.0,
            f64::from(color.rgb.1) / 255.0,
            f64::from(color.rgb.2) / 255.0,
            color.alpha,
        );

        self.context
            .fill()
            .map_err(DrawingErrorKind::DrawingError)?;

        Ok(())
    }

    fn draw_line<S: BackendStyle>(
        &mut self,
        from: BackendCoord,
        to: BackendCoord,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.set_color(&style.color());
        self.set_stroke_width(style.stroke_width());

        self.context.move_to(f64::from(from.0), f64::from(from.1));
        self.context.line_to(f64::from(to.0), f64::from(to.1));

        self.context
            .stroke()
            .map_err(DrawingErrorKind::DrawingError)?;

        Ok(())
    }

    fn draw_rect<S: BackendStyle>(
        &mut self,
        upper_left: BackendCoord,
        bottom_right: BackendCoord,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.set_color(&style.color());
        self.set_stroke_width(style.stroke_width());

        self.context.rectangle(
            f64::from(upper_left.0),
            f64::from(upper_left.1),
            f64::from(bottom_right.0 - upper_left.0),
            f64::from(bottom_right.1 - upper_left.1),
        );

        if fill {
            self.context
                .fill()
                .map_err(DrawingErrorKind::DrawingError)?;
        } else {
            self.context
                .stroke()
                .map_err(DrawingErrorKind::DrawingError)?;
        }

        Ok(())
    }

    fn draw_path<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
        &mut self,
        path: I,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.set_color(&style.color());
        self.set_stroke_width(style.stroke_width());

        let mut path = path.into_iter();
        if let Some((x, y)) = path.next() {
            self.context.move_to(f64::from(x), f64::from(y));
        }

        for (x, y) in path {
            self.context.line_to(f64::from(x), f64::from(y));
        }

        self.context
            .stroke()
            .map_err(DrawingErrorKind::DrawingError)?;

        Ok(())
    }

    fn fill_polygon<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
        &mut self,
        path: I,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.set_color(&style.color());
        self.set_stroke_width(style.stroke_width());

        let mut path = path.into_iter();

        if let Some((x, y)) = path.next() {
            self.context.move_to(f64::from(x), f64::from(y));

            for (x, y) in path {
                self.context.line_to(f64::from(x), f64::from(y));
            }

            self.context.close_path();
            self.context
                .fill()
                .map_err(DrawingErrorKind::DrawingError)?;
        }

        Ok(())
    }

    fn draw_circle<S: BackendStyle>(
        &mut self,
        center: BackendCoord,
        radius: u32,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.set_color(&style.color());
        self.set_stroke_width(style.stroke_width());

        self.context.new_sub_path();
        self.context.arc(
            f64::from(center.0),
            f64::from(center.1),
            f64::from(radius),
            0.0,
            std::f64::consts::PI * 2.0,
        );

        if fill {
            self.context
                .fill()
                .map_err(DrawingErrorKind::DrawingError)?;
        } else {
            self.context
                .stroke()
                .map_err(DrawingErrorKind::DrawingError)?;
        }

        Ok(())
    }

    fn estimate_text_size<S: BackendTextStyle>(
        &self,
        text: &str,
        font: &S,
    ) -> Result<(u32, u32), DrawingErrorKind<Self::ErrorType>> {
        self.set_font(font);

        let extents = self
            .context
            .text_extents(text)
            .map_err(DrawingErrorKind::DrawingError)?;

        Ok((extents.width() as u32, extents.height() as u32))
    }

    fn draw_text<S: BackendTextStyle>(
        &mut self,
        text: &str,
        style: &S,
        pos: BackendCoord,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let color = style.color();
        let (mut x, mut y) = (pos.0, pos.1);

        let degree = match style.transform() {
            FontTransform::None => 0.0,
            FontTransform::Rotate90 => 90.0,
            FontTransform::Rotate180 => 180.0,
            FontTransform::Rotate270 => 270.0,
            //FontTransform::RotateAngle(angle) => angle as f64,
        } / 180.0
            * std::f64::consts::PI;

        if degree != 0.0 {
            self.context
                .save()
                .map_err(DrawingErrorKind::DrawingError)?;
            self.context.translate(f64::from(x), f64::from(y));
            self.context.rotate(degree);

            x = 0;
            y = 0;
        }

        self.set_font(style);
        self.set_color(&color);

        let extents = self
            .context
            .text_extents(text)
            .map_err(DrawingErrorKind::DrawingError)?;

        let dx = match style.anchor().h_pos {
            HPos::Left => 0.0,
            HPos::Right => -extents.width(),
            HPos::Center => -extents.width() / 2.0,
        };
        let dy = match style.anchor().v_pos {
            VPos::Top => extents.height(),
            VPos::Center => extents.height() / 2.0,
            VPos::Bottom => 0.0,
        };

        self.context.move_to(
            f64::from(x) + dx - extents.x_bearing(),
            f64::from(y) + dy - extents.y_bearing() - extents.height(),
        );

        self.context
            .show_text(text)
            .map_err(DrawingErrorKind::DrawingError)?;

        if degree != 0.0 {
            self.context
                .restore()
                .map_err(DrawingErrorKind::DrawingError)?;
        }

        Ok(())
    }
}
