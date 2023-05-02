pub struct SkiaPlotterBackend<'a> {
    canvas: &'a mut skia_safe::canvas::Canvas,
    width: u32,
    height: u32,
    scale_factor: f32,
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
    pub fn new(
        canvas: &'a mut skia_safe::canvas::Canvas,
        width: u32,
        height: u32,
        scale_factor: f32,
    ) -> Self {
        Self {
            canvas,
            width,
            height,
            scale_factor,
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
        (
            (self.width as f32 / self.scale_factor).round() as _,
            (self.height as f32 / self.scale_factor).round() as _,
        )
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
            (point.0 as f32 + self.scale_factor).round() as _,
            (point.1 as f32 + self.scale_factor).round() as _,
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
        paint.set_stroke_width(style.stroke_width() as f32 * self.scale_factor);

        let start = Point::new(
            from.0 as f32 * self.scale_factor,
            from.1 as f32 * self.scale_factor,
        );
        let end = Point::new(
            to.0 as f32 * self.scale_factor,
            to.1 as f32 * self.scale_factor,
        );

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
        paint.set_stroke_width(style.stroke_width() as f32 * self.scale_factor);

        if fill {
            paint.set_style(PaintStyle::Fill);
        } else {
            paint.set_style(PaintStyle::Stroke);
        }

        self.canvas.draw_rect(
            Rect::new(
                upper_left.0 as f32 * self.scale_factor,
                upper_left.1 as f32 * self.scale_factor,
                bottom_right.0 as f32 * self.scale_factor,
                bottom_right.1 as f32 * self.scale_factor,
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
        paint.set_stroke_width(style.stroke_width() as f32 * self.scale_factor);

        let mut path = path.into_iter();
        if let Some((x, y)) = path.next() {
            let mut start = (x as f32 * self.scale_factor, y as f32 * self.scale_factor);
            for (x, y) in path {
                let end = (x as f32 * self.scale_factor, y as f32 * self.scale_factor);

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
        paint.set_stroke_width(style.stroke_width() as f32 * self.scale_factor);
        paint.set_style(PaintStyle::Fill);

        let mut path = vert.into_iter();
        let mut path_builder = Path::new();

        if let Some((x, y)) = path.next() {
            fn point(x: i32, y: i32, scale_factor: f32) -> Point {
                Point::new(x as f32 * scale_factor, y as f32 * scale_factor)
            }
            path_builder.move_to(point(x, y, self.scale_factor));
            for (x, y) in path {
                path_builder.line_to(point(x, y, self.scale_factor));
            }
            path_builder.close();
        }

        self.canvas.draw_path(&path_builder, &paint);

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
        paint.set_stroke_width(style.stroke_width() as f32 * self.scale_factor);

        if fill {
            paint.set_style(PaintStyle::Fill);
        } else {
            paint.set_style(PaintStyle::Stroke);
        }

        self.canvas.draw_circle(
            Point::new(
                center.0 as f32 * self.scale_factor,
                center.1 as f32 * self.scale_factor,
            ),
            radius as f32 * self.scale_factor,
            &paint,
        );

        Ok(())
    }

    fn estimate_text_size<S: plotters_backend::BackendTextStyle>(
        &self,
        text: &str,
        style: &S,
    ) -> Result<(u32, u32), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        use plotters_backend::*;
        use skia_safe::*;

        let font_family = style.family().as_str().to_owned();
        let font_style = self.to_sk_font_style(style);
        let typeface = Typeface::new(&font_family, font_style).ok_or(
            DrawingErrorKind::DrawingError(SkiaError::FontNotFound(font_family)),
        )?;

        let font = Font::new(typeface, Some(style.size() as f32 * self.scale_factor));

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
        let extents = font.measure_str(text, Some(&paint)).1;

        Ok((
            (extents.width() / self.scale_factor).round() as u32,
            (extents.height() / self.scale_factor).round() as u32,
        ))
    }

    fn draw_text<S: plotters_backend::BackendTextStyle>(
        &mut self,
        text: &str,
        style: &S,
        pos: plotters_backend::BackendCoord,
    ) -> Result<(), plotters_backend::DrawingErrorKind<Self::ErrorType>> {
        use skia_safe::*;
        use {plotters_backend::text_anchor::*, plotters_backend::*};

        let color = style.color();
        if color.alpha == 0.0 {
            return Ok(());
        };

        let (mut x, mut y) = (
            pos.0 as f32 * self.scale_factor,
            pos.1 as f32 * self.scale_factor,
        );

        let degree: f32 = match style.transform() {
            FontTransform::None => 0.0,
            FontTransform::Rotate90 => 90.0,
            FontTransform::Rotate180 => 180.0,
            FontTransform::Rotate270 => 270.0,
        };

        if degree != 0.0 {
            self.canvas.save();
            self.canvas.translate(Point::new(x, y));
            self.canvas.rotate(degree, None);

            x = 0.;
            y = 0.;
        }

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

        let font_family = style.family().as_str().to_owned();
        let font_style = self.to_sk_font_style(style);
        let typeface = Typeface::new(&font_family, font_style).ok_or(
            DrawingErrorKind::DrawingError(SkiaError::FontNotFound(font_family)),
        )?;

        let font = Font::new(typeface, Some(style.size() as f32 * self.scale_factor));

        let extents = font.measure_str(text, Some(&paint)).1;
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

        let origin = Point::new(
            x as f32 + dx - extents.x(),
            y as f32 + dy - extents.y() - extents.height(),
        );
        self.canvas.draw_str(text, origin, &font, &paint);

        if degree != 0.0 {
            self.canvas.restore();
        }

        Ok(())
    }
}
