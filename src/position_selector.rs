//! Widget for selecting overlay position with clickable corner and bottom regions

use cosmic::iced::Size;
use cosmic::iced_core::{
    layout::{self, Layout},
    renderer::Quad,
    Background, Border, Element, Length,
};

use crate::config::OverlayPosition;

/// Widget for selecting overlay position
///
/// Layout:
/// ```text
/// ⌜      ⌝
///  ⌞  _  ⌟
/// ```
pub struct PositionSelector<Msg> {
    size: f32,
    current_position: OverlayPosition,
    on_select: fn(OverlayPosition) -> Msg,
}

impl<Msg: Clone> PositionSelector<Msg> {
    pub fn new(
        size: f32,
        current_position: OverlayPosition,
        on_select: fn(OverlayPosition) -> Msg,
    ) -> Self {
        Self {
            size,
            current_position,
            on_select,
        }
    }

    /// Determine which region a point falls into
    fn get_region(
        &self,
        x: f32,
        y: f32,
        bounds: cosmic::iced_core::Rectangle,
    ) -> Option<OverlayPosition> {
        let local_x = x - bounds.x;
        let local_y = y - bounds.y;

        // Check if point is inside bounds
        if local_x < 0.0 || local_x > bounds.width || local_y < 0.0 || local_y > bounds.height {
            return None;
        }

        let corner_size = bounds.width * 0.3;
        let center_width = bounds.width * 0.3;
        let center_x_start = (bounds.width - center_width) / 2.0;
        let center_x_end = center_x_start + center_width;

        // Top row (top 30%)
        if local_y < corner_size {
            if local_x < corner_size {
                return Some(OverlayPosition::TopLeft);
            } else if local_x > bounds.width - corner_size {
                return Some(OverlayPosition::TopRight);
            }
        }

        // Bottom row (bottom 30%)
        if local_y > bounds.height - corner_size {
            if local_x < corner_size {
                return Some(OverlayPosition::BottomLeft);
            } else if local_x > bounds.width - corner_size {
                return Some(OverlayPosition::BottomRight);
            } else if local_x >= center_x_start && local_x <= center_x_end {
                return Some(OverlayPosition::BottomCenter);
            }
        }

        None
    }
}

impl<Msg: Clone + 'static> cosmic::widget::Widget<Msg, cosmic::Theme, cosmic::Renderer>
    for PositionSelector<Msg>
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size * 0.6))
    }

    fn layout(
        &self,
        _tree: &mut cosmic::iced_core::widget::Tree,
        _renderer: &cosmic::Renderer,
        _limits: &cosmic::iced::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(self.size, self.size * 0.6))
    }

    fn draw(
        &self,
        _tree: &cosmic::iced_core::widget::Tree,
        renderer: &mut cosmic::Renderer,
        theme: &cosmic::Theme,
        _style: &cosmic::iced_core::renderer::Style,
        layout: Layout<'_>,
        cursor: cosmic::iced_core::mouse::Cursor,
        _viewport: &cosmic::iced_core::Rectangle,
    ) {
        use cosmic::iced_core::Renderer as _;

        let bounds = layout.bounds();
        let cosmic_theme = theme.cosmic();
        let accent = cosmic::iced::Color::from(cosmic_theme.accent_color());
        let radius = cosmic_theme.radius_xs();

        let base_color = cosmic::iced::Color::from_rgba(0.4, 0.4, 0.4, 0.6);
        let hover_color = cosmic::iced::Color::from_rgba(0.6, 0.6, 0.6, 0.8);

        // Determine hovered region
        let hovered_region = cursor
            .position()
            .and_then(|pos| self.get_region(pos.x, pos.y, bounds));

        let corner_size = 12.0;
        let thickness = 4.0;

        // Helper to get color for a position
        let get_color = |pos: OverlayPosition| -> cosmic::iced::Color {
            if self.current_position == pos {
                accent
            } else if hovered_region == Some(pos) {
                hover_color
            } else {
                base_color
            }
        };

        // Top-left corner (⌜) - L shape: vertical then horizontal (no overlap)
        let tl_color = get_color(OverlayPosition::TopLeft);
        // Vertical part (full height)
        renderer.fill_quad(
            Quad {
                bounds: cosmic::iced_core::Rectangle {
                    x: bounds.x,
                    y: bounds.y,
                    width: thickness,
                    height: corner_size,
                },
                border: Border {
                    radius: [radius[0], 0.0, 0.0, 0.0].into(),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                shadow: cosmic::iced_core::Shadow::default(),
            },
            Background::Color(tl_color),
        );
        // Horizontal part (starts after vertical to avoid overlap)
        renderer.fill_quad(
            Quad {
                bounds: cosmic::iced_core::Rectangle {
                    x: bounds.x + thickness,
                    y: bounds.y,
                    width: corner_size - thickness,
                    height: thickness,
                },
                border: Border {
                    radius: [0.0, radius[0], 0.0, 0.0].into(),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                shadow: cosmic::iced_core::Shadow::default(),
            },
            Background::Color(tl_color),
        );

        // Top-right corner (⌝) - L shape
        let tr_color = get_color(OverlayPosition::TopRight);
        // Vertical part (full height)
        renderer.fill_quad(
            Quad {
                bounds: cosmic::iced_core::Rectangle {
                    x: bounds.x + bounds.width - thickness,
                    y: bounds.y,
                    width: thickness,
                    height: corner_size,
                },
                border: Border {
                    radius: [0.0, radius[0], 0.0, 0.0].into(),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                shadow: cosmic::iced_core::Shadow::default(),
            },
            Background::Color(tr_color),
        );
        // Horizontal part (ends before vertical to avoid overlap)
        renderer.fill_quad(
            Quad {
                bounds: cosmic::iced_core::Rectangle {
                    x: bounds.x + bounds.width - corner_size,
                    y: bounds.y,
                    width: corner_size - thickness,
                    height: thickness,
                },
                border: Border {
                    radius: [radius[0], 0.0, 0.0, 0.0].into(),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                shadow: cosmic::iced_core::Shadow::default(),
            },
            Background::Color(tr_color),
        );

        // Bottom-left corner (⌞) - L shape
        let bl_color = get_color(OverlayPosition::BottomLeft);
        // Vertical part (full height)
        renderer.fill_quad(
            Quad {
                bounds: cosmic::iced_core::Rectangle {
                    x: bounds.x,
                    y: bounds.y + bounds.height - corner_size,
                    width: thickness,
                    height: corner_size,
                },
                border: Border {
                    radius: [0.0, 0.0, 0.0, radius[0]].into(),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                shadow: cosmic::iced_core::Shadow::default(),
            },
            Background::Color(bl_color),
        );
        // Horizontal part (starts after vertical to avoid overlap)
        renderer.fill_quad(
            Quad {
                bounds: cosmic::iced_core::Rectangle {
                    x: bounds.x + thickness,
                    y: bounds.y + bounds.height - thickness,
                    width: corner_size - thickness,
                    height: thickness,
                },
                border: Border {
                    radius: [0.0, 0.0, radius[0], 0.0].into(),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                shadow: cosmic::iced_core::Shadow::default(),
            },
            Background::Color(bl_color),
        );

        // Bottom-right corner (⌟) - L shape
        let br_color = get_color(OverlayPosition::BottomRight);
        // Vertical part (full height)
        renderer.fill_quad(
            Quad {
                bounds: cosmic::iced_core::Rectangle {
                    x: bounds.x + bounds.width - thickness,
                    y: bounds.y + bounds.height - corner_size,
                    width: thickness,
                    height: corner_size,
                },
                border: Border {
                    radius: [0.0, 0.0, radius[0], 0.0].into(),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                shadow: cosmic::iced_core::Shadow::default(),
            },
            Background::Color(br_color),
        );
        // Horizontal part (ends before vertical to avoid overlap)
        renderer.fill_quad(
            Quad {
                bounds: cosmic::iced_core::Rectangle {
                    x: bounds.x + bounds.width - corner_size,
                    y: bounds.y + bounds.height - thickness,
                    width: corner_size - thickness,
                    height: thickness,
                },
                border: Border {
                    radius: [0.0, 0.0, 0.0, radius[0]].into(),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                shadow: cosmic::iced_core::Shadow::default(),
            },
            Background::Color(br_color),
        );

        // Bottom center (underscore _)
        let bc_color = get_color(OverlayPosition::BottomCenter);
        let center_width = bounds.width * 0.25;
        let center_x = bounds.x + (bounds.width - center_width) / 2.0;
        renderer.fill_quad(
            Quad {
                bounds: cosmic::iced_core::Rectangle {
                    x: center_x,
                    y: bounds.y + bounds.height - thickness,
                    width: center_width,
                    height: thickness,
                },
                border: Border {
                    radius: radius.into(),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                shadow: cosmic::iced_core::Shadow::default(),
            },
            Background::Color(bc_color),
        );
    }

    fn mouse_interaction(
        &self,
        _state: &cosmic::iced_core::widget::Tree,
        layout: Layout<'_>,
        cursor: cosmic::iced_core::mouse::Cursor,
        _viewport: &cosmic::iced_core::Rectangle,
        _renderer: &cosmic::Renderer,
    ) -> cosmic::iced_core::mouse::Interaction {
        if let Some(pos) = cursor.position() {
            if self.get_region(pos.x, pos.y, layout.bounds()).is_some() {
                return cosmic::iced_core::mouse::Interaction::Pointer;
            }
        }
        cosmic::iced_core::mouse::Interaction::default()
    }

    fn on_event(
        &mut self,
        _state: &mut cosmic::iced_core::widget::Tree,
        event: cosmic::iced_core::Event,
        layout: Layout<'_>,
        cursor: cosmic::iced_core::mouse::Cursor,
        _renderer: &cosmic::Renderer,
        _clipboard: &mut dyn cosmic::iced_core::Clipboard,
        shell: &mut cosmic::iced_core::Shell<'_, Msg>,
        _viewport: &cosmic::iced_core::Rectangle,
    ) -> cosmic::iced_core::event::Status {
        if let cosmic::iced_core::Event::Mouse(cosmic::iced_core::mouse::Event::ButtonPressed(
            cosmic::iced_core::mouse::Button::Left,
        )) = event
        {
            if let Some(pos) = cursor.position() {
                if let Some(region) = self.get_region(pos.x, pos.y, layout.bounds()) {
                    let msg = (self.on_select)(region);
                    shell.publish(msg);
                    return cosmic::iced_core::event::Status::Captured;
                }
            }
        }
        cosmic::iced_core::event::Status::Ignored
    }
}

impl<'a, Msg: Clone + 'static> From<PositionSelector<Msg>>
    for Element<'a, Msg, cosmic::Theme, cosmic::Renderer>
{
    fn from(widget: PositionSelector<Msg>) -> Self {
        Element::new(widget)
    }
}
