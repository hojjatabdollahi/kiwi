//! Small widgets shared by the settings window, the Customize drawer and the
//! arrange toolbar: a −/+ stepper and a color chip.

use std::borrow::Cow;
use std::f32::consts::PI;

use cosmic::iced::{mouse, Alignment, Color, Length, Radians, Rectangle};
use cosmic::widget::canvas::{self, path::Arc, Frame, Path, Stroke};
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
/// "#ffffff38" for one color, a gradient swatch and "#4d5973b8 → #33405966" for two
pub fn fill_chip<'a, M: Clone + 'static>(
    start: Color,
    end: Option<Color>,
    selected: bool,
    on_press: M,
) -> Element<'a, M> {
    let hex = |color: Color| {
        let [r, g, b, a] = color.into_rgba8();
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    };
    let (swatch, label): (Element<'a, M>, String) = match end {
        None => (
            widget::Canvas::new(RoundSwatch(start))
                .width(Length::Fixed(20.0))
                .height(Length::Fixed(20.0))
                .into(),
            hex(start),
        ),
        Some(end) => (
            widget::Canvas::new(GradientSwatch { start, end })
                .width(Length::Fixed(36.0))
                .height(Length::Fixed(20.0))
                .into(),
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

/// A gradient in a pill shape, left to right, over a checkerboard so transparency shows
struct GradientSwatch {
    start: Color,
    end: Color,
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
        // Checkered between the round ends (the canvas can only clip to rectangles)
        frame.fill(&pill, Color::from_rgb8(204, 204, 204));
        frame.with_clip(
            Rectangle::new(
                Point::new(radius, 1.0),
                Size::new(bounds.width - 2.0 * radius, bounds.height - 2.0),
            ),
            |frame| crate::color_picker::checkerboard(frame, bounds.size()),
        );
        let gradient = Linear::new(Point::ORIGIN, Point::new(bounds.width, 0.0))
            .add_stop(0.0, self.start)
            .add_stop(1.0, self.end);
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

        // Checkered: light, with two dark quarters
        frame.fill(&circle, Color::from_rgb8(204, 204, 204));
        for start in [PI, 0.0] {
            let quarter = Path::new(|p| {
                p.move_to(center);
                p.arc(Arc {
                    center,
                    radius,
                    start_angle: Radians(start),
                    end_angle: Radians(start + PI / 2.0),
                });
                p.close();
            });
            frame.fill(&quarter, Color::from_rgb8(153, 153, 153));
        }
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
/// the fraction from 0 to 1 slides the content in from that side and pushes its
/// neighbors along; shrinking it slides the content out.
pub struct Reveal<'a, M> {
    content: Element<'a, M>,
    fraction: f32,
    keep_right: bool,
}

impl<'a, M> Reveal<'a, M> {
    /// `fraction` of the width stays visible; `keep_right` keeps the right side (else the left)
    pub fn new(content: impl Into<Element<'a, M>>, fraction: f32, keep_right: bool) -> Self {
        Self {
            content: content.into(),
            fraction: fraction.clamp(0.0, 1.0),
            keep_right,
        }
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
        let width = full.width * self.fraction;
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
        renderer.with_layer(layout.bounds(), |renderer| {
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
    }
}

impl<'a, M: 'a> From<Reveal<'a, M>> for Element<'a, M> {
    fn from(reveal: Reveal<'a, M>) -> Self {
        Element::new(reveal)
    }
}
