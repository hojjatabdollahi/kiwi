//! Settings window view and related logic

use cosmic::iced::{Color, Length};
use cosmic::iced_widget::svg;
use cosmic::iced_widget::Svg;
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::scrollable;

use crate::config::{KeyDisplayMode, OverlayPosition, PaletteType, APP_VERSION};
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
    is_active: bool,
) -> Element<'static, Message> {
    // Find current selection index
    let current_index = PaletteType::ALL.iter().position(|p| *p == palette);

    // Position selector widget (larger size for better visibility)
    let position_selector = PositionSelector::new(200.0, position, Message::SetPosition);

    // Sample keystroke preview (scales with slider, cap at 250 for window)
    let preview_size = key_size.min(250.0);
    let sample_keystroke = Keystroke::single("Alt", false);
    let preview = keystroke_widget::<Message>(
        &sample_keystroke,
        preview_size,
        1.0, // fade_duration (unused when fade disabled)
        palette,
        false, // fade_enabled = false for static preview
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
    let display_mode_section = widget::column()
        .spacing(4)
        .push(widget::text::body("Key Display Mode"))
        .push(
            widget::row()
                .spacing(15)
                .push(
                    widget::radio(
                        KeyDisplayMode::TypedCharacter.name(),
                        KeyDisplayMode::TypedCharacter,
                        Some(key_display_mode),
                        Message::SetKeyDisplayMode,
                    ),
                )
                .push(
                    widget::radio(
                        KeyDisplayMode::PhysicalKey.name(),
                        KeyDisplayMode::PhysicalKey,
                        Some(key_display_mode),
                        Message::SetKeyDisplayMode,
                    ),
                ),
        )
        .push(
            widget::text::caption(format!("Example: {}", key_display_mode.example()))
                .class(cosmic::theme::Text::Color(Color::from_rgba(0.6, 0.6, 0.6, 1.0))),
        );

    let content = widget::column()
        .padding(10)
        .spacing(8)
        .max_width(300.0)
        // Active toggle at top
        .push(
            widget::row()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::body("Active"))
                .push(widget::Space::with_width(Length::Fill))
                .push(widget::toggler(is_active).on_toggle(Message::ToggleActive)),
        )
        // Separator
        .push(widget::divider::horizontal::default())
        // Key Display Mode section
        .push(display_mode_section)
        // Separator
        .push(widget::divider::horizontal::default())
        // Preview in fixed container (centered)
        .push(preview_container)
        // Size slider and Theme dropdown on same row
        .push(
            widget::row()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::tooltip(
                    widget::slider(32.0..=256.0, key_size, Message::SetKeySize)
                        .width(Length::Fill),
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
            widget::row()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::body(format!("Fade: {:.1}s", fade_duration)))
                .push(
                    widget::slider(1.0..=10.0, fade_duration, Message::SetFadeDuration)
                        .width(Length::Fill),
                ),
        )
        .push(widget::divider::horizontal::default())
        // Position selector (centered, no label, with padding)
        .push(position_container);

    // Version text (bottom right)
    let version_text = widget::text::caption(format!("v{}", APP_VERSION))
        .class(cosmic::theme::Text::Color(Color::from_rgba(0.5, 0.5, 0.5, 0.8)));

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
    widget::column()
        .push(scrollable_content)
        .push(
            widget::container(version_text)
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Right)
                .padding([0, 10, 5, 0]),
        )
        .into()
}
