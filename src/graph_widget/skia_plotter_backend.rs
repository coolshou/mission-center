pub struct SkiaPlotterBackend<'a> {
    canvas: &'a mut skia_safe::canvas::Canvas,
    width: u32,
    height: u32,
}

#[derive(Debug)]
pub enum SkiaError {
    FontNotFound(String),
}

impl std::fmt::Display for SkiaError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{:?}", self)
    }
}

impl std::error::Error for SkiaError {}

impl<'a> SkiaPlotterBackend<'a> {
    pub fn new(canvas: &'a mut skia_safe::canvas::Canvas, width: u32, height: u32) -> Self {
        Self {
            canvas,
            width,
            height,
        }
    }

    fn to_sk_font_style<S: plotters_backend::BackendTextStyle>(
        &self,
        font: &S,
    ) -> skia_safe::font_style::FontStyle {
        use skia_safe::*;

        let (weight, slant) = match font.style() {
            plotters_backend::FontStyle::Normal => {
                (font_style::Weight::NORMAL, font_style::Slant::Upright)
            }
            plotters_backend::FontStyle::Oblique => {
                (font_style::Weight::NORMAL, font_style::Slant::Oblique)
            }
            plotters_backend::FontStyle::Italic => {
                (font_style::Weight::NORMAL, font_style::Slant::Italic)
            }
            plotters_backend::FontStyle::Bold => {
                (font_style::Weight::BOLD, font_style::Slant::Upright)
            }
        };

        FontStyle::new(weight, font_style::Width::NORMAL, slant)
    }
}

impl<'a> plotters_backend::DrawingBackend for SkiaPlotterBackend<'a> {
    type ErrorType = SkiaError;

    fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn ensure_prepared(
        &mut self,
    ) -> Result<(), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn present(&mut self) -> Result<(), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn draw_pixel(
        &mut self,
        point: plotters_backend::BackendCoord,
        color: plotters_backend::BackendColor,
    ) -> Result<(), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        use skia_safe::*;

        let rect = Rect::new(
            point.0 as _,
            point.1 as _,
            (point.0 + 1) as _,
            (point.1 + 1) as _,
        );

        let mut paint = Paint::new(
            Color4f::new(
                color.rgb.0 as f32 / 256.,
                color.rgb.1 as f32 / 256.,
                color.rgb.2 as f32 / 256.,
                color.alpha as _,
            ),
            None,
        );
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);

        self.canvas.draw_rect(rect, &paint);

        Ok(())
    }

    fn draw_line<S: plotters_backend::BackendStyle>(
        &mut self,
        from: plotters_backend::BackendCoord,
        to: plotters_backend::BackendCoord,
        style: &S,
    ) -> Result<(), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        use skia_safe::*;

        let color = style.color();
        let mut paint = Paint::new(
            Color4f::new(
                color.rgb.0 as f32 / 256.,
                color.rgb.1 as f32 / 256.,
                color.rgb.2 as f32 / 256.,
                color.alpha as _,
            ),
            None,
        );
        paint.set_anti_alias(true);
        paint.set_stroke_width(style.stroke_width() as _);

        let start = Point::new(from.0 as _, from.1 as _);
        let end = Point::new(to.0 as _, to.1 as _);

        self.canvas.draw_line(start, end, &paint);

        Ok(())
    }

    fn draw_rect<S: plotters_backend::BackendStyle>(
        &mut self,
        upper_left: plotters_backend::BackendCoord,
        bottom_right: plotters_backend::BackendCoord,
        style: &S,
        fill: bool,
    ) -> Result<(), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        use skia_safe::*;

        let color = style.color();
        let mut paint = Paint::new(
            Color4f::new(
                color.rgb.0 as f32 / 256.,
                color.rgb.1 as f32 / 256.,
                color.rgb.2 as f32 / 256.,
                color.alpha as _,
            ),
            None,
        );
        paint.set_anti_alias(true);
        paint.set_stroke_width(style.stroke_width() as _);

        if fill {
            paint.set_style(PaintStyle::Fill);
        } else {
            paint.set_style(PaintStyle::Stroke);
        }

        self.canvas.draw_rect(
            Rect::new(
                upper_left.0 as _,
                upper_left.1 as _,
                bottom_right.0 as _,
                bottom_right.1 as _,
            ),
            &paint,
        );

        Ok(())
    }

    fn draw_path<
        S: plotters_backend::BackendStyle,
        I: IntoIterator<Item = plotters_backend::BackendCoord>,
    >(
        &mut self,
        path: I,
        style: &S,
    ) -> Result<(), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        use skia_safe::*;

        let color = style.color();
        let mut paint = Paint::new(
            Color4f::new(
                color.rgb.0 as f32 / 256.,
                color.rgb.1 as f32 / 256.,
                color.rgb.2 as f32 / 256.,
                color.alpha as _,
            ),
            None,
        );
        paint.set_anti_alias(true);
        paint.set_stroke_width(style.stroke_width() as _);

        let mut path = path.into_iter();
        if let Some((x, y)) = path.next() {
            let mut start = (x as f32, y as f32);
            for (x, y) in path {
                let end = (x as _, y as _);

                self.canvas.draw_line(
                    Point::new(start.0, start.1),
                    Point::new(end.0, end.1),
                    &paint,
                );

                start = end;
            }
        }

        Ok(())
    }

    fn fill_polygon<
        S: plotters_backend::BackendStyle,
        I: IntoIterator<Item = plotters_backend::BackendCoord>,
    >(
        &mut self,
        vert: I,
        style: &S,
    ) -> Result<(), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        use skia_safe::*;

        let color = style.color();
        let mut paint = Paint::new(
            Color4f::new(
                color.rgb.0 as f32 / 256.,
                color.rgb.1 as f32 / 256.,
                color.rgb.2 as f32 / 256.,
                color.alpha as _,
            ),
            None,
        );
        paint.set_anti_alias(true);
        paint.set_stroke_width(style.stroke_width() as _);
        paint.set_style(PaintStyle::Fill);

        self.canvas.draw_points(
            canvas::PointMode::Polygon,
            vert.into_iter()
                .map(|(x, y)| Point::new(x as _, y as _))
                .collect::<Vec<_>>()
                .as_slice(),
            &paint,
        );

        Ok(())
    }

    fn draw_circle<S: plotters_backend::BackendStyle>(
        &mut self,
        center: plotters_backend::BackendCoord,
        radius: u32,
        style: &S,
        fill: bool,
    ) -> Result<(), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        use skia_safe::*;

        let color = style.color();
        let mut paint = Paint::new(
            Color4f::new(
                color.rgb.0 as f32 / 256.,
                color.rgb.1 as f32 / 256.,
                color.rgb.2 as f32 / 256.,
                color.alpha as _,
            ),
            None,
        );
        paint.set_anti_alias(true);
        paint.set_stroke_width(style.stroke_width() as _);

        if fill {
            paint.set_style(PaintStyle::Fill);
        } else {
            paint.set_style(PaintStyle::Stroke);
        }

        self.canvas.draw_circle(
            Point::new(center.0 as _, center.1 as _),
            radius as _,
            &paint,
        );

        Ok(())
    }

    fn estimate_text_size<TStyle: plotters_backend::BackendTextStyle>(
        &self,
        text: &str,
        style: &TStyle,
    ) -> Result<(u32, u32), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        use skia_safe::*;
        use std::ops::Add;

        let color = style.color();
        let mut paint = Paint::new(
            Color4f::new(
                color.rgb.0 as f32 / 256.,
                color.rgb.1 as f32 / 256.,
                color.rgb.2 as f32 / 256.,
                color.alpha as _,
            ),
            None,
        );
        paint.set_anti_alias(true);

        let font = Font::from_typeface(
            Typeface::from_name(style.family().as_str(), self.to_sk_font_style(style)).ok_or(
                plotters_backend::DrawingErrorKind::FontError(
                    SkiaError::FontNotFound(style.family().as_str().to_owned()).into(),
                ),
            )?,
            Some(style.size() as _),
        );
        let (_, extents) = font.measure_str(text, Some(&paint));

        Ok((
            extents.width().add(0.5).round() as _,
            extents.height().add(0.5).round() as _,
        ))
    }

    fn draw_text<S: plotters_backend::BackendTextStyle>(
        &mut self,
        text: &str,
        style: &S,
        pos: plotters_backend::BackendCoord,
    ) -> Result<(), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        use skia_safe::*;

        let color = style.color();
        let mut paint = Paint::new(
            Color4f::new(
                color.rgb.0 as f32 / 256.,
                color.rgb.1 as f32 / 256.,
                color.rgb.2 as f32 / 256.,
                color.alpha as _,
            ),
            None,
        );
        paint.set_anti_alias(true);

        let degree = match style.transform() {
            plotters_backend::FontTransform::None => 0.0,
            plotters_backend::FontTransform::Rotate90 => 90.0,
            plotters_backend::FontTransform::Rotate180 => 180.0,
            plotters_backend::FontTransform::Rotate270 => 270.0,
        } / 180.0
            * std::f64::consts::PI;

        let mut matrix = Matrix::default();
        if degree != 0.0 {
            matrix.set_rotate(degree as _, None);
        }

        let font = Font::from_typeface(
            Typeface::from_name(style.family().as_str(), self.to_sk_font_style(style)).ok_or(
                plotters_backend::DrawingErrorKind::FontError(
                    SkiaError::FontNotFound(style.family().as_str().to_owned()).into(),
                ),
            )?,
            Some(style.size() as _),
        );
        let (_, extents) = font.measure_str(text, Some(&paint));
        let (ewidth, eheight) = (extents.width(), extents.height());

        let mut start = Point::new(pos.0 as _, pos.1 as _);
        self.canvas.save();
        {
            self.canvas.translate(start.clone());
            self.canvas.rotate(degree as _, None);

            start.x = 0.;
            start.y = 0.;
            self.canvas.draw_str(text, start, &font, &paint);
        }
        self.canvas.restore();

        Ok(())
    }
}
