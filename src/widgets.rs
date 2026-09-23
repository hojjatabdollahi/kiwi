//! Small widgets shared by the settings window, the Customize drawer and the
//! arrange toolbar: a −/+ stepper and a color chip.

use std::borrow::Cow;

use cosmic::iced::{mouse, Alignment, Color, Length, Rectangle};
use cosmic::widget::canvas::{self, Frame, Path, Stroke};
use cosmic::widget::{self, button, container};
use cosmic::Element;

/// A pill with − and + buttons around a value, like "− 12 +". A `None`
/// message disables that button (at the end of the range).
pub fn stepper<'a, M: Clone + 'static>(
    value: impl Into<Cow<'a, str>> + 'a,
    value_width: f32,
    less: Option<M>,
    more: Option<M>,
) -> Element<'a, M> {
    let side = |icon: &'static str, message: Option<M>| {
        button::icon(widget::icon::from_name(icon))
            .extra_small()
            .on_press_maybe(message)
    };
    widget::container(
        widget::Row::new()
            .align_y(Alignment::Center)
            .push(side("list-remove-symbolic", less))
            .push(
                widget::text::body(value)
                    .width(Length::Fixed(value_width))
                    .align_x(cosmic::iced::alignment::Horizontal::Center),
            )
            .push(side("list-add-symbolic", more)),
    )
    .padding(2)
    .class(cosmic::theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        container::Style {
            background: Some(Color::from(cosmic.button.base).into()),
            border: cosmic::iced::Border {
                radius: cosmic.radius_xl().into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }))
    .into()
}

/// A pill showing a color or a gradient and its hex codes: a round swatch and
/// "#ffffff38" for one color, a gradient swatch and "#4d5973b8 → #33405966" for a
/// gradient. `end` is the gradient's end color, where its two colors sit (0-1)
/// and its angle in degrees.
pub fn fill_chip<'a, M: Clone + 'static>(
    start: Color,
    end: Option<(Color, (f32, f32), f32)>,
    selected: bool,
    on_press: M,
) -> Element<'a, M> {
    let hex = |color: Color| {
        let [r, g, b, a] = color.into_rgba8();
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    };
    // The color is drawn over a checkerboard of the same shape, so any
    // transparency shows
    let on_checkers = |width: f32, color: Element<'a, M>| -> Element<'a, M> {
        cosmic::iced::widget::stack![checkered_pill(width, 20.0), color].into()
    };
    let (swatch, label): (Element<'a, M>, String) = match end {
        None => (
            on_checkers(
                20.0,
                widget::Canvas::new(RoundSwatch(start))
                    .width(Length::Fixed(20.0))
                    .height(Length::Fixed(20.0))
                    .into(),
            ),
            hex(start),
        ),
        Some((end, (start_at, end_at), angle)) => (
            on_checkers(
                36.0,
                widget::Canvas::new(GradientSwatch {
                    stops: [(start_at, start), (end_at, end)],
                    angle,
                })
                .width(Length::Fixed(36.0))
                .height(Length::Fixed(20.0))
                .into(),
            ),
            format!("{} → {}", hex(start), hex(end)),
        ),
    };
    let content = widget::Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(swatch)
        .push(widget::text::caption(label).font(cosmic::font::mono()));
    button::custom(content)
        .padding([3, 10, 3, 3])
        .class(cosmic::theme::Button::Standard)
        .selected(selected)
        .on_press(on_press)
        .into()
}

/// A checkerboard in a pill shape (a circle when it's square), to put a color on
/// so its transparency shows. It's an SVG because a canvas can only clip to
/// rectangles, and SVG can clip the pattern to the round ends.
fn checkered_pill<'a, M: 'a>(width: f32, height: f32) -> Element<'a, M> {
    let radius = height / 2.0 - 1.0;
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\">\
         <defs><pattern id=\"c\" width=\"8\" height=\"8\" patternUnits=\"userSpaceOnUse\">\
         <rect width=\"8\" height=\"8\" fill=\"#cccccc\"/>\
         <rect width=\"4\" height=\"4\" fill=\"#999999\"/>\
         <rect x=\"4\" y=\"4\" width=\"4\" height=\"4\" fill=\"#999999\"/>\
         </pattern></defs>\
         <rect x=\"1\" y=\"1\" width=\"{w}\" height=\"{h}\" rx=\"{radius}\" fill=\"url(#c)\"/></svg>",
        w = width - 2.0,
        h = height - 2.0,
    );
    widget::svg(widget::svg::Handle::from_memory(svg.into_bytes()))
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .into()
}

/// A gradient in a pill shape, left to right, over a checkerboard so transparency shows
struct GradientSwatch {
    /// The two colors and where they sit, from 0 to 1
    stops: [(f32, Color); 2],
    /// The gradient's angle in degrees
    angle: f32,
}

impl<M> canvas::Program<M, cosmic::Theme> for GradientSwatch {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &cosmic::Renderer,
        theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        use cosmic::iced::{Point, Size};
        use cosmic::widget::canvas::gradient::Linear;

        let mut frame = Frame::new(renderer, bounds.size());
        let radius = bounds.height / 2.0 - 1.0;
        let pill = Path::rounded_rectangle(
            Point::new(1.0, 1.0),
            Size::new(bounds.width - 2.0, bounds.height - 2.0),
            radius.into(),
        );
        let (from, to) = cosmic::iced::Radians(self.angle.to_radians())
            .to_distance(&Rectangle::new(Point::ORIGIN, bounds.size()));
        let gradient = Linear::new(from, to)
            .add_stop(self.stops[0].0, self.stops[0].1)
            .add_stop(self.stops[1].0, self.stops[1].1);
        frame.fill(&pill, gradient);
        frame.stroke(
            &pill,
            Stroke::default()
                .with_color(Color::from(theme.cosmic().bg_divider()))
                .with_width(1.0),
        );
        vec![frame.into_geometry()]
    }
}

/// A color in a circle, over a checkered circle so transparency shows
struct RoundSwatch(Color);

impl<M> canvas::Program<M, cosmic::Theme> for RoundSwatch {
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
        let center = frame.center();
        let radius = bounds.width.min(bounds.height) / 2.0 - 1.0;
        let circle = Path::circle(center, radius);

        frame.fill(&circle, self.0);
        frame.stroke(
            &Path::circle(center, radius),
            Stroke::default()
                .with_color(Color::from(theme.cosmic().bg_divider()))
                .with_width(1.0),
        );
        vec![frame.into_geometry()]
    }
}

/// Shows a fraction of its content's width, clipped, keeping one side. Growing
/// the fraction from 0 to 1 opens room and pushes the neighbors along; shrinking
/// it closes the room again. The content can also be drawn nudged toward the kept
/// side, by a fraction of its own width, without moving anything else.
pub struct Reveal<'a, M> {
    content: Element<'a, M>,
    fraction: f32,
    keep_right: bool,
    nudge: f32,
    max_width: f32,
}

impl<'a, M> Reveal<'a, M> {
    /// `fraction` of the width stays visible; `keep_right` keeps the right side (else the left)
    pub fn new(content: impl Into<Element<'a, M>>, fraction: f32, keep_right: bool) -> Self {
        Self {
            content: content.into(),
            fraction: fraction.clamp(0.0, 1.0),
            keep_right,
            nudge: 0.0,
            max_width: f32::INFINITY,
        }
    }

    /// Never take up more than this width; the rest of the content is clipped
    /// away on the side that isn't kept
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_width = max_width.max(0.0);
        self
    }

    /// Draw the content shifted toward the kept side by this fraction of its width
    pub fn nudge(mut self, nudge: f32) -> Self {
        self.nudge = nudge;
        self
    }
}

impl<M> cosmic::widget::Widget<M, cosmic::Theme, cosmic::Renderer> for Reveal<'_, M> {
    fn size(&self) -> cosmic::iced::Size<Length> {
        cosmic::iced::Size::new(Length::Shrink, Length::Shrink)
    }

    fn children(&self) -> Vec<cosmic::iced::core::widget::Tree> {
        vec![cosmic::iced::core::widget::Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut cosmic::iced::core::widget::Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn layout(
        &mut self,
        tree: &mut cosmic::iced::core::widget::Tree,
        renderer: &cosmic::Renderer,
        limits: &cosmic::iced::core::layout::Limits,
    ) -> cosmic::iced::core::layout::Node {
        use cosmic::iced::core::layout::{Limits, Node};
        use cosmic::iced::{Point, Size};

        // The content gets its natural width, however little of it shows
        let unbounded = Limits::new(Size::ZERO, Size::new(f32::INFINITY, limits.max().height));
        let content =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &unbounded);
        let full = content.size();
        let width = (full.width * self.fraction).min(self.max_width);
        let x = if self.keep_right {
            width - full.width
        } else {
            0.0
        };
        Node::with_children(
            Size::new(width, full.height),
            vec![content.move_to(Point::new(x, 0.0))],
        )
    }

    fn draw(
        &self,
        tree: &cosmic::iced::core::widget::Tree,
        renderer: &mut cosmic::Renderer,
        theme: &cosmic::Theme,
        style: &cosmic::iced::core::renderer::Style,
        layout: cosmic::iced::core::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use cosmic::iced::core::Renderer as _;

        let Some(content) = layout.children().next() else {
            return;
        };
        let direction = if self.keep_right { 1.0 } else { -1.0 };
        let shift = cosmic::iced::Vector::new(direction * self.nudge * content.bounds().width, 0.0);
        let draw = |renderer: &mut cosmic::Renderer| {
            renderer.with_translation(shift, |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    content,
                    cursor,
                    viewport,
                );
            });
        };
        // Only clip while part of the content is hidden
        if layout.bounds().width < content.bounds().width - 0.5 {
            renderer.with_layer(layout.bounds(), draw);
        } else {
            draw(renderer);
        }
    }
}

impl<'a, M: 'a> From<Reveal<'a, M>> for Element<'a, M> {
    fn from(reveal: Reveal<'a, M>) -> Self {
        Element::new(reveal)
    }
}

/// A dashed rounded outline filling its space, for "add something here" spots.
/// Stack it over the content; it doesn't take any input.
pub fn dashed_outline<'a, M: 'a>(radius: f32) -> Element<'a, M> {
    widget::Canvas::new(DashedOutline { radius })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

struct DashedOutline {
    radius: f32,
}

impl<M> canvas::Program<M, cosmic::Theme> for DashedOutline {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &cosmic::Renderer,
        theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        use cosmic::iced::{Point, Size};
        use cosmic::widget::canvas::LineDash;

        let mut frame = Frame::new(renderer, bounds.size());
        let outline = Path::rounded_rectangle(
            Point::new(1.0, 1.0),
            Size::new(bounds.width - 2.0, bounds.height - 2.0),
            self.radius.into(),
        );
        let color = Color::from(theme.current_container().component.divider);
        frame.stroke(
            &outline,
            Stroke {
                line_dash: LineDash {
                    segments: &[6.0, 4.0],
                    offset: 0,
                },
                ..Stroke::default().with_color(color).with_width(1.5)
            },
        );
        vec![frame.into_geometry()]
    }
}
