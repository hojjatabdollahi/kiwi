//! Settings window view and related logic

use cosmic::iced::{Color, Length};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::scrollable;
use cosmic::widget::svg;
use cosmic::widget::Svg;

use crate::config::{IconStyle, KeyDisplayMode, OverlayPosition, PaletteType, APP_VERSION};
use crate::keystroke::{keystroke_widget, Keystroke};
use crate::position_selector::PositionSelector;
use crate::Message;

// Checkerboard pattern SVG for transparency preview
const CHECKERBOARD_SVG: &[u8] =
    b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\">\
  <rect width=\"8\" height=\"8\" fill=\"rgb(204,204,204)\"/>\
  <rect x=\"8\" width=\"8\" height=\"8\" fill=\"rgb(153,153,153)\"/>\
  <rect y=\"8\" width=\"8\" height=\"8\" fill=\"rgb(153,153,153)\"/>\
  <rect x=\"8\" y=\"8\" width=\"8\" height=\"8\" fill=\"rgb(204,204,204)\"/>\
</svg>";

/// Static palette names for dropdown
const PALETTE_NAMES: &[&str] = &["Dark", "Light", "Frosted", "Kiwi"];

/// Renders the settings view for the application
pub fn settings_view(
    key_size: f32,
    fade_duration: f32,
    palette: PaletteType,
    position: OverlayPosition,
    key_display_mode: KeyDisplayMode,
    icon_style: IconStyle,
    history_count: u8,
    is_active: bool,
    show_keyboard: bool,
    show_mouse: bool,
    show_gestures: bool,
) -> Element<'static, Message> {
    // Find current selection index
    let current_index = PaletteType::ALL.iter().position(|p| *p == palette);

    // Position selector widget (larger size for better visibility)
    let position_selector = PositionSelector::new(200.0, position, Message::SetPosition);

    // Sample keystroke preview (scales with slider, cap at 250 for window)
    let preview_size = key_size.min(250.0);
    let mut sample_keystroke = Keystroke::single("Alt", false);
    sample_keystroke.count = 2; // Show multiplier in preview
    let preview = keystroke_widget::<Message>(
        &sample_keystroke,
        preview_size,
        1.0, // fade_duration (unused when fade disabled)
        palette,
        false,    // fade_enabled = false for static preview
        position, // Use current position setting for preview
        icon_style,
    );

    // Checkerboard background for transparency preview
    let checkerboard = Svg::new(svg::Handle::from_memory(CHECKERBOARD_SVG))
        .width(Length::Fill)
        .height(Length::Fill);

    // Stack preview on top of checkerboard
    let preview_with_bg = widget::container(cosmic::iced::widget::stack![
        widget::container(checkerboard)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center),
        widget::container(preview)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center),
    ])
    .width(Length::Fill)
    .height(Length::Fill);

    // Fixed-size container for preview (tall enough for max preview size)
    let preview_container = widget::container(preview_with_bg)
        .width(Length::Fill)
        .height(Length::Fixed(260.0))
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .align_y(cosmic::iced::alignment::Vertical::Center);

    let position_container = widget::container(position_selector)
        .width(Length::Fill)
        .padding([10, 0]) // vertical padding
        .align_x(cosmic::iced::alignment::Horizontal::Center);

    // Key Display Mode radio buttons
    let display_mode_section = widget::Column::new()
        .spacing(4)
        .push(widget::text::body("Key Display Mode"))
        .push(
            widget::Row::new()
                .spacing(15)
                .push(widget::radio(
                    KeyDisplayMode::TypedCharacter.name(),
                    KeyDisplayMode::TypedCharacter,
                    Some(key_display_mode),
                    Message::SetKeyDisplayMode,
                ))
                .push(widget::radio(
                    KeyDisplayMode::PhysicalKey.name(),
                    KeyDisplayMode::PhysicalKey,
                    Some(key_display_mode),
                    Message::SetKeyDisplayMode,
                )),
        )
        .push(
            widget::text::caption(format!("Example: {}", key_display_mode.example())).class(
                cosmic::theme::Text::Color(Color::from_rgba(0.6, 0.6, 0.6, 1.0)),
            ),
        );

    // Icon Style radio buttons
    let icon_style_section = widget::Column::new()
        .spacing(4)
        .push(widget::text::body("Icon Style"))
        .push(
            widget::Row::new()
                .spacing(15)
                .push(widget::radio(
                    IconStyle::Symbol.name(),
                    IconStyle::Symbol,
                    Some(icon_style),
                    Message::SetIconStyle,
                ))
                .push(widget::radio(
                    IconStyle::Text.name(),
                    IconStyle::Text,
                    Some(icon_style),
                    Message::SetIconStyle,
                )),
        );

    // Input sources section
    let input_sources_section = widget::Column::new()
        .spacing(6)
        .push(widget::text::body("Input Sources"))
        .push(
            widget::Row::new()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::caption("Keyboard"))
                .push(widget::Space::new().width(Length::Fill))
                .push(widget::toggler(show_keyboard).on_toggle(Message::SetShowKeyboard)),
        )
        .push(
            widget::Row::new()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::caption("Mouse"))
                .push(widget::Space::new().width(Length::Fill))
                .push(widget::toggler(show_mouse).on_toggle(Message::SetShowMouse)),
        )
        .push(
            widget::Row::new()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::caption("Gestures"))
                .push(widget::Space::new().width(Length::Fill))
                .push(widget::toggler(show_gestures).on_toggle(Message::SetShowGestures)),
        );

    let content = widget::Column::new()
        .padding(10)
        .spacing(8)
        .max_width(300.0)
        // Active toggle at top
        .push(
            widget::Row::new()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::body("Active"))
                .push(widget::Space::new().width(Length::Fill))
                .push(widget::toggler(is_active).on_toggle(Message::ToggleActive)),
        )
        // Separator
        .push(widget::divider::horizontal::default())
        // Key Display Mode section
        .push(display_mode_section)
        // Icon Style section
        .push(icon_style_section)
        // Separator
        .push(widget::divider::horizontal::default())
        // Input Sources section
        .push(input_sources_section)
        // Separator
        .push(widget::divider::horizontal::default())
        // Preview in fixed container (centered)
        .push(preview_container)
        // Size slider and Theme dropdown on same row
        .push(
            widget::Row::new()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::tooltip(
                    widget::slider(32.0..=160.0, key_size, Message::SetKeySize).width(Length::Fill),
                    "Size",
                    widget::tooltip::Position::Bottom,
                ))
                .push(widget::tooltip(
                    widget::dropdown(PALETTE_NAMES, current_index, Message::SetPaletteIndex),
                    "Theme",
                    widget::tooltip::Position::Bottom,
                )),
        )
        // Separator
        .push(widget::divider::horizontal::default())
        // Fade slider
        .push(
            widget::Row::new()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::body(format!("Fade: {:.1}s", fade_duration)))
                .push(
                    widget::slider(1.0..=10.0, fade_duration, Message::SetFadeDuration)
                        .width(Length::Fill),
                ),
        )
        // History count slider
        .push(
            widget::Row::new()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::body(format!("History: {}", history_count)))
                .push(
                    widget::slider(1.0..=10.0, history_count as f32, |v| {
                        Message::SetHistoryCount(v as u8)
                    })
                    .width(Length::Fill),
                ),
        )
        .push(widget::divider::horizontal::default())
        // Position selector (centered, no label, with padding)
        .push(widget::text::body("Position"))
        .push(position_container);

    // Version text (bottom right)
    let version_text = widget::text::caption(format!("v{}", APP_VERSION)).class(
        cosmic::theme::Text::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.8)),
    );

    // Wrap content in scrollable and center it (with clipping to prevent overflow into header)
    let scrollable_content = widget::container(
        scrollable(
            widget::container(content)
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .clip(true);

    // Main layout: scrollable content + version at bottom right
    widget::Column::new()
        .push(scrollable_content)
        .push(
            widget::container(version_text)
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Right)
                .padding([0, 10, 5, 0]),
        )
        .into()
}
