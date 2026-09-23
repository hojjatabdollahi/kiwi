//! Settings window view and related logic

use cosmic::iced::{Alignment, Color, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, scrollable, segmented_button, settings, svg, Svg};

use crate::config::{IconStyle, KeyDisplayMode, OverlayPosition, APP_VERSION};
use crate::customize::CustomizeMessage;
use crate::keystroke::{keystrokes_row, KeyModifiers, Keystroke};
use crate::theme::{Layout, Theme, ThemeChoice};
use crate::{KiwiApp, Message};

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

const TOUCHSCREEN_ICON: &[u8] = include_bytes!("../data/icons/kiwi-tap.svg");

/// Height of a theme card's sample area
const CARD_PREVIEW_HEIGHT: f32 = 64.0;

/// A segmented button model offering `options`, with `current` selected
pub fn segmented_model<T: Copy + PartialEq + 'static>(
    options: &[(&'static str, T)],
    current: T,
) -> segmented_button::SingleSelectModel {
    let mut model = segmented_button::SingleSelectModel::default();
    for (label, value) in options {
        let entity = model.insert().text(*label).data(*value).id();
        if *value == current {
            model.activate(entity);
        }
    }
    model
}

/// Select the option holding `value`, e.g. after the config changed elsewhere
pub fn select_segment<T: PartialEq + 'static>(
    model: &mut segmented_button::SingleSelectModel,
    value: T,
) {
    let found = model.iter().find(|e| model.data::<T>(*e) == Some(&value));
    if let Some(entity) = found {
        model.activate(entity);
    }
}

pub const DISPLAY_MODES: &[(&str, KeyDisplayMode)] = &[
    ("Character @", KeyDisplayMode::TypedCharacter),
    ("Key 2", KeyDisplayMode::PhysicalKey),
];

pub const ICON_STYLES: &[(&str, IconStyle)] =
    &[("Icons", IconStyle::Symbol), ("Names", IconStyle::Text)];

/// Renders the settings window
pub fn settings_view(app: &KiwiApp) -> Element<'_, Message> {
    let config = &app.config;
    let current = ThemeChoice::from_config(config);
    let current_name = app.current_theme_name();

    // Theme gallery: two cards per row, each drawn with its own theme, then the import card
    let mut cards: Vec<Element<'_, Message>> = app
        .themes
        .iter()
        .map(|(choice, theme)| {
            let selected = *choice == current;
            let name = if selected {
                current_name.clone()
            } else {
                choice.name().to_string()
            };
            theme_card(choice, name, theme, selected, config.icon_style)
        })
        .collect();
    cards.push(import_card());

    let mut theme_grid = widget::Column::new().spacing(8);
    let mut cards = cards.into_iter();
    while let Some(first) = cards.next() {
        let second = cards
            .next()
            .unwrap_or_else(|| widget::Space::new().width(Length::Fill).into());
        theme_grid = theme_grid.push(widget::Row::new().spacing(8).push(first).push(second));
    }

    let mut theme_section = widget::Column::new()
        .spacing(8)
        .push(
            widget::Row::new()
                .align_y(Alignment::Center)
                .push(widget::text::heading("Theme"))
                .push(widget::Space::new().width(Length::Fill))
                .push(
                    widget::button::link("Open themes folder").on_press(Message::OpenThemesFolder),
                ),
        )
        .push(theme_grid)
        .push(
            widget::Row::new()
                .push(widget::Space::new().width(Length::Fill))
                .push(
                    widget::button::standard(format!("Customize {current_name}"))
                        .on_press(Message::Customize(CustomizeMessage::Open)),
                ),
        );
    if let Some(message) = &app.theme_message {
        theme_section = theme_section.push(widget::text::caption(message.as_str()));
    }

    let placement = settings::section().title("Placement").add(
        settings::item::builder(config.position.name())
            .description(format!("{:.0} px from the edge", config.margin))
            .control(
                widget::button::suggested("Arrange on screen").on_press(Message::StartArranging),
            ),
    );

    let behavior = settings::section()
        .title("Behavior")
        .add(settings::item(
            "Stay visible for",
            widget::Row::new()
                .spacing(12)
                .align_y(Alignment::Center)
                .push(
                    widget::slider(1.0..=10.0, config.fade_duration, Message::SetFadeDuration)
                        .width(Length::Fixed(150.0)),
                )
                .push(
                    widget::text::body(format!("{:.1} s", config.fade_duration))
                        .width(Length::Fixed(44.0))
                        .align_x(cosmic::iced::alignment::Horizontal::Right),
                ),
        ))
        .add(settings::item(
            "Shift+2 shows",
            widget::segmented_control::horizontal(&app.display_mode_model)
                .on_activate(Message::DisplayModeTab)
                .width(Length::Shrink),
        ))
        .add(
            settings::item::builder("Special keys")
                .description("Only for keys the theme has no icon for")
                .control(
                    widget::segmented_control::horizontal(&app.icon_style_model)
                        .on_activate(Message::IconStyleTab)
                        .width(Length::Shrink),
                ),
        );

    let named_icon = |name: &'static str| widget::icon::from_name(name).size(20).icon();
    let input_row =
        |title: &'static str, icon: widget::Icon, on: bool, message: fn(bool) -> Message| {
            settings::item::builder(title)
                .icon(icon)
                .toggler(on, message)
        };
    let inputs = settings::section()
        .title("Show input from")
        .add(input_row(
            "Keyboard",
            named_icon("input-keyboard-symbolic"),
            config.show_keyboard,
            Message::SetShowKeyboard,
        ))
        .add(
            settings::item::builder("Mouse")
                .description("Clicks and scrolling")
                .icon(named_icon("input-mouse-symbolic"))
                .toggler(config.show_mouse, Message::SetShowMouse),
        )
        .add(input_row(
            "Touchpad gestures",
            named_icon("input-touchpad-symbolic"),
            config.show_gestures,
            Message::SetShowGestures,
        ))
        .add(input_row(
            "Touchscreen",
            widget::icon::from_svg_bytes(TOUCHSCREEN_ICON)
                .symbolic(true)
                .icon()
                .size(20),
            config.show_touch,
            Message::SetShowTouch,
        ))
        .add(input_row(
            "Drawing tablet",
            named_icon("input-tablet-symbolic"),
            config.show_tablet,
            Message::SetShowTablet,
        ));

    let content = widget::Column::new()
        .padding([0, 16, 16, 16])
        .spacing(24)
        .max_width(520.0)
        .push(theme_section)
        .push(placement)
        .push(behavior)
        .push(inputs);

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

/// A theme card's frame: the image-button style, outlined in the accent color when selected
fn card<'a>(
    content: impl Into<Element<'a, Message>>,
    selected: bool,
) -> widget::Button<'a, Message> {
    widget::button::custom(content)
        .class(cosmic::theme::Button::Image)
        .selected(selected)
        .padding(4)
        .width(Length::Fill)
}

/// Card text in the normal text color (the image-button style would make it the accent)
fn card_label<'a>(
    label: impl Into<std::borrow::Cow<'a, str>> + 'a,
) -> widget::Text<'a, cosmic::Theme> {
    let color = Color::from(cosmic::theme::active().cosmic().on_bg_color());
    widget::text::body(label).class(cosmic::theme::Text::Color(color))
}

/// A clickable theme card showing a short sample drawn with that theme, and a preview button
fn theme_card(
    choice: &ThemeChoice,
    name: String,
    theme: &Theme,
    selected: bool,
    icon_style: IconStyle,
) -> Element<'static, Message> {
    let ctrl = KeyModifiers {
        ctrl: true,
        ..Default::default()
    };
    let key = |k: &str| Keystroke::single(k, false);
    let keys = match theme.layout {
        Layout::Keys => vec![key("V"), Keystroke::combination(&ctrl, "C", false)],
        Layout::Text => vec![
            key("g"),
            key("i"),
            key("t"),
            Keystroke::combination(&ctrl, "S", false),
        ],
    };
    // Right-aligned so the order reads left to right, like typing
    let sample = keystrokes_row::<Message>(
        &keys,
        26.0,
        60.0, // long enough that the sample never fades
        theme,
        110.0, // a typewriter line short enough to fit the card
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
    .height(Length::Fixed(CARD_PREVIEW_HEIGHT))
    .clip(true);

    let play = widget::tooltip(
        widget::button::icon(widget::icon::from_name("media-playback-start-symbolic"))
            .extra_small()
            .on_press(Message::PreviewTheme(choice.clone())),
        "Preview",
        widget::tooltip::Position::Top,
    );
    let label = widget::Row::new()
        .align_y(Alignment::Center)
        .push(card_label(name).width(Length::Fill))
        .push(play);

    card(
        widget::Column::new()
            .spacing(4)
            .width(Length::Fill)
            .push(preview)
            .push(label),
        selected,
    )
    .on_press(Message::SelectTheme(choice.clone()))
    .into()
}

/// The last card: import a theme from a zip file
fn import_card() -> Element<'static, Message> {
    let content = widget::container(
        widget::Column::new()
            .spacing(2)
            .align_x(Alignment::Center)
            .push(card_label("Import a theme…"))
            .push(widget::text::caption(".zip file")),
    )
    .width(Length::Fill)
    // Same height as a theme card: the sample area plus a label row
    .height(Length::Fixed(CARD_PREVIEW_HEIGHT + 32.0))
    .align_x(cosmic::iced::alignment::Horizontal::Center)
    .align_y(cosmic::iced::alignment::Vertical::Center);

    card(content, false).on_press(Message::ImportTheme).into()
}
