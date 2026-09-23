//! Settings window view and related logic

use cosmic::iced::{Color, Length};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::scrollable;
use cosmic::widget::svg;
use cosmic::widget::Svg;

use std::sync::Arc;

use crate::config::{IconStyle, KeyDisplayMode, OverlayPosition, APP_VERSION};
use crate::keystroke::{keystrokes_row, KeyModifiers, Keystroke};
use crate::position_selector::PositionSelector;
use crate::theme::{Theme, ThemeChoice};
use crate::Message;

// Checkerboard pattern SVG for transparency preview
// (wide, with small squares, so it can cover a card without the squares growing)
const CHECKERBOARD_SVG: &[u8] =
    b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"320\" height=\"80\">\
  <defs><pattern id=\"c\" width=\"16\" height=\"16\" patternUnits=\"userSpaceOnUse\">\
  <rect width=\"16\" height=\"16\" fill=\"rgb(204,204,204)\"/>\
  <rect x=\"8\" width=\"8\" height=\"8\" fill=\"rgb(153,153,153)\"/>\
  <rect y=\"8\" width=\"8\" height=\"8\" fill=\"rgb(153,153,153)\"/>\
  </pattern></defs>\
  <rect width=\"320\" height=\"80\" fill=\"url(#c)\"/>\
</svg>";

/// Renders the settings view for the application
pub fn settings_view(
    key_size: f32,
    fade_duration: f32,
    themes: &[(ThemeChoice, Arc<Theme>)],
    current_theme: &ThemeChoice,
    theme_message: Option<&str>,
    position: OverlayPosition,
    key_display_mode: KeyDisplayMode,
    icon_style: IconStyle,
    history_count: u8,
    is_active: bool,
    show_keyboard: bool,
    show_mouse: bool,
    show_gestures: bool,
    show_touch: bool,
) -> Element<'static, Message> {
    // Position selector widget (larger size for better visibility)
    let position_selector = PositionSelector::new(200.0, position, Message::SetPosition);

    // Theme gallery: two cards per row, each drawn with its own theme
    let mut theme_grid = widget::Column::new().spacing(8);
    for pair in themes.chunks(2) {
        let mut row = widget::Row::new().spacing(8);
        for (choice, theme) in pair {
            row = row.push(theme_card(
                choice,
                theme,
                choice == current_theme,
                icon_style,
            ));
        }
        if pair.len() == 1 {
            row = row.push(widget::Space::new().width(Length::Fill));
        }
        theme_grid = theme_grid.push(row);
    }

    let mut theme_section = widget::Column::new()
        .spacing(8)
        .push(
            widget::Row::new()
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::body("Theme"))
                .push(widget::Space::new().width(Length::Fill))
                .push(
                    widget::button::link("Open themes folder").on_press(Message::OpenThemesFolder),
                ),
        )
        .push(theme_grid)
        .push(
            widget::Row::new()
                .spacing(8)
                .push(widget::Space::new().width(Length::Fill))
                .push(widget::button::standard("Import…").on_press(Message::ImportTheme))
                .push(widget::button::standard("Export…").on_press(Message::ExportTheme)),
        );
    if let Some(message) = theme_message {
        theme_section = theme_section.push(widget::text::caption(message.to_string()));
    }

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
        )
        .push(
            widget::Row::new()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::caption("Touchscreen"))
                .push(widget::Space::new().width(Length::Fill))
                .push(widget::toggler(show_touch).on_toggle(Message::SetShowTouch)),
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
        .push(theme_section)
        .push(
            widget::Row::new()
                .spacing(10)
                .align_y(cosmic::iced::Alignment::Center)
                .push(widget::text::body(format!("Size: {:.0}", key_size)))
                .push(
                    widget::slider(32.0..=160.0, key_size, Message::SetKeySize).width(Length::Fill),
                ),
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

/// A clickable theme card showing "V" then "Ctrl + C" drawn with that theme
fn theme_card(
    choice: &ThemeChoice,
    theme: &Theme,
    selected: bool,
    icon_style: IconStyle,
) -> Element<'static, Message> {
    let ctrl = KeyModifiers {
        ctrl: true,
        ..Default::default()
    };
    let keys = [
        Keystroke::single("V", false),
        Keystroke::combination(&ctrl, "C", false),
    ];
    // Right-aligned so the order reads left to right, like typing
    let sample = keystrokes_row::<Message>(
        &keys,
        26.0,
        60.0, // long enough that the sample never fades
        theme,
        OverlayPosition::TopRight,
        keys.len(),
        icon_style,
    );

    // Checkerboard behind the keys shows how transparent the theme is
    let checkerboard = Svg::new(svg::Handle::from_memory(CHECKERBOARD_SVG))
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(cosmic::iced::ContentFit::Cover);
    let preview = widget::container(cosmic::iced::widget::stack![
        checkerboard,
        widget::container(sample)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center),
    ])
    .width(Length::Fill)
    .height(Length::Fixed(64.0))
    .clip(true);

    let content = widget::Column::new()
        .spacing(6)
        .width(Length::Fill)
        .push(preview)
        .push(widget::text::body(choice.name().to_string()));

    widget::button::custom(content)
        .class(cosmic::theme::Button::Image)
        .selected(selected)
        .padding(4)
        .width(Length::Fill)
        .on_press(Message::SelectTheme(choice.clone()))
        .into()
}
