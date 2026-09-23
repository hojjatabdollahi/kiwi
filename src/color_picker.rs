//! A small color picker that includes transparency: a saturation/brightness
//! square, a hue bar, an opacity bar and a hex field. Plus a swatch to show a color.

use std::rc::Rc;

use cosmic::iced::{mouse, Color, Length, Point, Rectangle, Size};
use cosmic::widget::canvas::{self, gradient::Linear, Frame, Path, Stroke};
use cosmic::widget::{self};
use cosmic::Element;

/// A color as hue (0-360), saturation, value and alpha (all 0-1).
///
/// The caller keeps this rather than a `Color`, so the hue isn't lost while
/// the color is gray or black.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsva {
    pub h: f32,
    pub s: f32,
    pub v: f32,
    pub a: f32,
}

impl Hsva {
    pub fn from_color(color: Color) -> Self {
        let (r, g, b) = (color.r, color.g, color.b);
        let max = r.max(g).max(b);
        let delta = max - r.min(g).min(b);
        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        let s = if max == 0.0 { 0.0 } else { delta / max };
        Self {
            h,
            s,
            v: max,
            a: color.a,
        }
    }

    pub fn to_color(self) -> Color {
        let chroma = self.v * self.s;
        let sector = self.h.rem_euclid(360.0) / 60.0;
        let x = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
        let (r, g, b) = match sector as u32 {
            0 => (chroma, x, 0.0),
            1 => (x, chroma, 0.0),
            2 => (0.0, chroma, x),
            3 => (0.0, x, chroma),
            4 => (x, 0.0, chroma),
            _ => (chroma, 0.0, x),
        };
        let m = self.v - chroma;
        Color::from_rgba(r + m, g + m, b + m, self.a)
    }

    /// The fully saturated, fully bright color of this hue
    fn pure_hue(self) -> Color {
        Self {
            s: 1.0,
            v: 1.0,
            a: 1.0,
            ..self
        }
        .to_color()
    }
}

/// The picker. `hex` is the text in the hex field, kept by the caller so a
/// half-typed value isn't thrown away.
pub fn color_picker<'a, M: Clone + 'static>(
    hsva: Hsva,
    hex: &'a str,
    on_change: impl Fn(Hsva) -> M + 'a,
    on_hex: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    let on_change: Rc<dyn Fn(Hsva) -> M + 'a> = Rc::new(on_change);
    let pad = |part: Part, height: f32| {
        widget::Canvas::new(Pad {
            part,
            hsva,
            on_change: on_change.clone(),
        })
        .width(Length::Fill)
        .height(Length::Fixed(height))
    };

    widget::Column::new()
        .spacing(8)
        .push(pad(Part::SaturationValue, 140.0))
        .push(pad(Part::Hue, 16.0))
        .push(pad(Part::Opacity, 16.0))
        .push(
            widget::Row::new()
                .spacing(8)
                .align_y(cosmic::iced::Alignment::Center)
                .push(swatch(hsva.to_color()))
                .push(
                    widget::text_input("#rrggbbaa", hex)
                        .on_input(on_hex)
                        .width(Length::Fill),
                ),
        )
        .into()
}

/// A gradient's two colors as a bar with a handle at each end. The handle for
/// the color being edited is ringed in the accent color; clicking near an end
/// selects that color.
pub fn gradient_bar<'a, M: Clone + 'static>(
    start: Color,
    end: Color,
    active: usize,
    on_select: impl Fn(usize) -> M + 'a,
) -> Element<'a, M> {
    widget::Canvas::new(GradientBar {
        start,
        end,
        active,
        on_select: Box::new(on_select),
    })
    .width(Length::Fill)
    .height(Length::Fixed(28.0))
    .into()
}

struct GradientBar<'a, M> {
    start: Color,
    end: Color,
    active: usize,
    on_select: Box<dyn Fn(usize) -> M + 'a>,
}

impl<M: Clone> canvas::Program<M, cosmic::Theme> for GradientBar<'_, M> {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<M>> {
        match event {
            cosmic::iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let point = cursor.position_in(bounds)?;
                let stop = usize::from(point.x > bounds.width / 2.0);
                Some(canvas::Action::publish((self.on_select)(stop)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &cosmic::Renderer,
        theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let handle = h / 2.0 - 2.0;

        // The bar runs between the handles' centers
        let bar = Rectangle::new(
            Point::new(handle, h * 0.25),
            Size::new((w - 2.0 * handle).max(0.0), h * 0.5),
        );
        frame.with_clip(bar, |frame| checkerboard(frame, bar.size()));
        let gradient = Linear::new(Point::new(bar.x, 0.0), Point::new(bar.x + bar.width, 0.0))
            .add_stop(0.0, self.start)
            .add_stop(1.0, self.end);
        frame.fill(&Path::rectangle(bar.position(), bar.size()), gradient);

        let accent = Color::from(theme.cosmic().accent_color());
        for (stop, color, x) in [(0, self.start, handle), (1, self.end, w - handle)] {
            let circle = Path::circle(Point::new(x, h / 2.0), handle - 1.0);
            frame.fill(&circle, Color { a: 1.0, ..color });
            ring(&mut frame, &circle);
            if stop == self.active {
                frame.stroke(
                    &Path::circle(Point::new(x, h / 2.0), handle + 0.5),
                    Stroke::default().with_color(accent).with_width(2.5),
                );
            }
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

/// A small box showing a color over a checkerboard, so transparency shows
pub fn swatch<'a, M: 'a>(color: Color) -> Element<'a, M> {
    widget::Canvas::new(Swatch(color))
        .width(Length::Fixed(36.0))
        .height(Length::Fixed(22.0))
        .into()
}

#[derive(Debug, Clone, Copy)]
enum Part {
    /// Saturation left to right, brightness bottom to top
    SaturationValue,
    Hue,
    Opacity,
}

struct Pad<'a, M> {
    part: Part,
    hsva: Hsva,
    on_change: Rc<dyn Fn(Hsva) -> M + 'a>,
}

impl<M> Pad<'_, M> {
    /// The color with this part set from a point on it
    fn pick(&self, bounds: Rectangle, point: Point) -> Hsva {
        let x = ((point.x - bounds.x) / bounds.width).clamp(0.0, 1.0);
        let y = ((point.y - bounds.y) / bounds.height).clamp(0.0, 1.0);
        match self.part {
            Part::SaturationValue => Hsva {
                s: x,
                v: 1.0 - y,
                ..self.hsva
            },
            Part::Hue => Hsva {
                h: x * 360.0,
                ..self.hsva
            },
            Part::Opacity => Hsva { a: x, ..self.hsva },
        }
    }
}

impl<M: Clone> canvas::Program<M, cosmic::Theme> for Pad<'_, M> {
    /// Whether a drag is in progress
    type State = bool;

    fn update(
        &self,
        dragging: &mut bool,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<M>> {
        use mouse::{Button, Event as Mouse};
        let pick = |point| canvas::Action::publish((self.on_change)(self.pick(bounds, point)));
        match event {
            cosmic::iced::Event::Mouse(Mouse::ButtonPressed(Button::Left)) => {
                let point = cursor.position_over(bounds)?;
                *dragging = true;
                Some(pick(point).and_capture())
            }
            // Keep following the pointer outside the pad until the button is released
            cosmic::iced::Event::Mouse(Mouse::CursorMoved { position }) if *dragging => {
                Some(pick(*position).and_capture())
            }
            cosmic::iced::Event::Mouse(Mouse::ButtonReleased(Button::Left)) if *dragging => {
                *dragging = false;
                Some(canvas::Action::capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _dragging: &bool,
        renderer: &cosmic::Renderer,
        _theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let area = Path::rectangle(Point::ORIGIN, bounds.size());
        let clear = |color: Color| Color { a: 0.0, ..color };

        let marker_x = match self.part {
            Part::SaturationValue => {
                frame.fill(&area, self.hsva.pure_hue());
                let whiten = Linear::new(Point::ORIGIN, Point::new(w, 0.0))
                    .add_stop(0.0, Color::WHITE)
                    .add_stop(1.0, clear(Color::WHITE));
                frame.fill(&area, whiten);
                let darken = Linear::new(Point::ORIGIN, Point::new(0.0, h))
                    .add_stop(0.0, clear(Color::BLACK))
                    .add_stop(1.0, Color::BLACK);
                frame.fill(&area, darken);

                let center = Point::new(self.hsva.s * w, (1.0 - self.hsva.v) * h);
                ring(&mut frame, &Path::circle(center, 6.0));
                return vec![frame.into_geometry()];
            }
            Part::Hue => {
                let mut rainbow = Linear::new(Point::ORIGIN, Point::new(w, 0.0));
                for i in 0..=6 {
                    let hue = Hsva {
                        h: i as f32 * 60.0,
                        ..self.hsva
                    };
                    rainbow = rainbow.add_stop(i as f32 / 6.0, hue.pure_hue());
                }
                frame.fill(&area, rainbow);
                self.hsva.h / 360.0 * w
            }
            Part::Opacity => {
                checkerboard(&mut frame, bounds.size());
                let opaque = Hsva {
                    a: 1.0,
                    ..self.hsva
                }
                .to_color();
                let fade = Linear::new(Point::ORIGIN, Point::new(w, 0.0))
                    .add_stop(0.0, clear(opaque))
                    .add_stop(1.0, opaque);
                frame.fill(&area, fade);
                self.hsva.a * w
            }
        };

        let handle = Path::rounded_rectangle(
            Point::new((marker_x - 3.0).clamp(0.0, w - 6.0), 0.0),
            Size::new(6.0, h),
            3.0.into(),
        );
        ring(&mut frame, &handle);
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        dragging: &bool,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if *dragging || cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

/// A white outline with a dark edge, visible on any color
fn ring(frame: &mut Frame, path: &Path) {
    frame.stroke(
        path,
        Stroke::default()
            .with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.6))
            .with_width(3.5),
    );
    frame.stroke(
        path,
        Stroke::default().with_color(Color::WHITE).with_width(2.0),
    );
}

pub(crate) fn checkerboard(frame: &mut Frame, size: Size) {
    let square = 6.0;
    frame.fill_rectangle(Point::ORIGIN, size, Color::from_rgb8(204, 204, 204));
    let (columns, rows) = (
        (size.width / square).ceil() as u32,
        (size.height / square).ceil() as u32,
    );
    for row in 0..rows {
        for column in (row % 2..columns).step_by(2) {
            let corner = Point::new(column as f32 * square, row as f32 * square);
            let visible = Size::new(
                square.min(size.width - corner.x),
                square.min(size.height - corner.y),
            );
            frame.fill_rectangle(corner, visible, Color::from_rgb8(153, 153, 153));
        }
    }
}

struct Swatch(Color);

impl<M> canvas::Program<M, cosmic::Theme> for Swatch {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &cosmic::Renderer,
        theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        checkerboard(&mut frame, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.0);
        frame.stroke(
            &Path::rectangle(Point::ORIGIN, bounds.size()),
            Stroke::default()
                .with_color(Color::from(theme.cosmic().bg_divider()))
                .with_width(1.0),
        );
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsva_round_trips() {
        for color in [
            Color::from_rgba(1.0, 0.0, 0.0, 1.0),
            Color::from_rgba(0.2, 0.6, 0.4, 0.5),
            Color::from_rgba(0.3, 0.35, 0.45, 0.72),
            Color::from_rgba(0.9, 0.1, 0.8, 0.0),
            Color::WHITE,
            Color::BLACK,
        ] {
            let back = Hsva::from_color(color).to_color();
            for (a, b) in [
                (back.r, color.r),
                (back.g, color.g),
                (back.b, color.b),
                (back.a, color.a),
            ] {
                assert!((a - b).abs() < 1e-5, "{color:?} came back as {back:?}");
            }
        }
        let green = Hsva {
            h: 120.0,
            s: 1.0,
            v: 1.0,
            a: 1.0,
        };
        assert_eq!(green.to_color(), Color::from_rgb(0.0, 1.0, 0.0));
    }
}
