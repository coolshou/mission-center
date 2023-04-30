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

        let mut path = vert.into_iter();
        let mut path_builder = Path::new();

        if let Some((x, y)) = path.next() {
            path_builder.move_to(Point::new(x as _, y as _));

            for (x, y) in path {
                path_builder.line_to(Point::new(x as _, y as _));
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
}
