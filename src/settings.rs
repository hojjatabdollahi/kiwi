//! Settings window view and related logic

use cosmic::iced::{Alignment, Color, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, scrollable, segmented_button, settings, svg, Svg};

use crate::config::{IconStyle, KeyDisplayMode, OverlayPosition, PreviewBackground, APP_VERSION};
use crate::keystroke::{keystrokes_row, KeyModifiers, Keystroke};
use crate::theme::{Layout, Theme, ThemeChoice};
use crate::{KiwiApp, Message};

// Checkerboard pattern SVG for transparency preview
// (wide, with small squares, so it can cover a card without the squares growing)
pub(crate) const CHECKERBOARD_SVG: &[u8] =
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
            theme_card(
                choice,
                name,
                theme,
                selected,
                config.icon_style,
                config.preview_background,
            )
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
                .spacing(8)
                .align_y(Alignment::Center)
                .push(widget::text::heading("Theme"))
                .push(widget::Space::new().width(Length::Fill))
                // What the previews are drawn on; translucent themes need a backdrop
                .push(preview_background_toggle(config.preview_background)),
        )
        .push(theme_grid);
    theme_section = theme_section.push(
        widget::Row::new()
            .push(widget::Space::new().width(Length::Fill))
            .push(widget::button::link("Open themes folder").on_press(Message::OpenThemesFolder)),
    );
    if let Some(message) = &app.theme_message {
        theme_section = theme_section.push(widget::text::caption(message.as_str()));
    }

    // Everything that's set on screen, spelled out so it's clear what "adjust" covers
    let length = match app.shared_state.lock().map(|s| s.theme.layout) {
        Ok(Layout::Text) => format!(
            "{:.0} px long",
            config.line_width.unwrap_or_else(|| app
                .shared_state
                .lock()
                .map_or(460.0, |s| s.theme.line_width))
        ),
        _ => format!("{} keys long", config.history_count),
    };
    let placement = settings::section().title("Position and size").add(
        settings::item::builder("Drag the keys anywhere on screen")
            .description(format!(
                "{:.0} px keys, {length}, growing {}",
                config.key_size,
                if config.grows_left() {
                    "leftward"
                } else {
                    "rightward"
                }
            ))
            .control(
                widget::button::suggested("Adjust on screen").on_press(Message::StartArranging),
            ),
    );

    let behavior = settings::section()
        .title("Behavior")
        .add(
            settings::item::builder("Linger")
                .description("How long keys stay fully visible")
                .control(seconds_slider(
                    1.0..=10.0,
                    config.fade_duration,
                    Message::SetFadeDuration,
                )),
        )
        .add(
            settings::item::builder("Disappear")
                .description("How long keys take to fade or wipe away")
                .control(seconds_slider(
                    0.0..=3.0,
                    config.disappear_duration,
                    Message::SetDisappearDuration,
                )),
        )
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

/// A theme card's frame: a card with a thin border, outlined in the accent color
/// when selected and shaded on hover. `dashed` leaves the border off, for a
/// dashed outline drawn on top instead.
fn card<'a>(
    content: impl Into<Element<'a, Message>>,
    selected: bool,
    dashed: bool,
) -> widget::Button<'a, Message> {
    let style = move |hovered: bool, theme: &cosmic::Theme| {
        let cosmic = theme.cosmic();
        let component = &theme.current_container().component;
        let mut style = widget::button::Style::new();
        style.border_radius = cosmic.corner_radii.radius_s.into();
        style.background = match (dashed, hovered) {
            (_, true) => Some(Color::from(component.hover).into()),
            (false, false) => Some(Color::from(component.base).into()),
            (true, false) => None,
        };
        style.border_width = match (selected, dashed) {
            (true, _) => 2.0,
            (false, false) => 1.0,
            (false, true) => 0.0,
        };
        style.border_color = if selected {
            Color::from(cosmic.accent_color())
        } else {
            Color::from(component.divider)
        };
        style.text_color = Some(Color::from(component.on));
        style.icon_color = Some(Color::from(component.on));
        style
    };
    widget::button::custom(content)
        .class(cosmic::theme::Button::Custom {
            active: Box::new(move |_, theme| style(false, theme)),
            disabled: Box::new(move |theme| style(false, theme)),
            hovered: Box::new(move |_, theme| style(true, theme)),
            pressed: Box::new(move |_, theme| style(true, theme)),
        })
        .padding(4)
        .width(Length::Fill)
}

/// A checkerboard icon, drawn in the icon color
const CHECKERED_ICON: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\">\
<path fill=\"currentColor\" d=\"M2 2h6v6H2zM8 8h6v6H8z\"/>\
<path fill=\"currentColor\" opacity=\".35\" d=\"M8 2h6v6H8zM2 8h6v6H2z\"/></svg>";

/// One button that switches what the previews are drawn on. Its icon shows the
/// current background.
fn preview_background_toggle<'a>(current: PreviewBackground) -> Element<'a, Message> {
    let icon = match current {
        PreviewBackground::Desktop => widget::icon::from_name("image-x-generic-symbolic").into(),
        PreviewBackground::Checkered => widget::icon::from_svg_bytes(CHECKERED_ICON).symbolic(true),
    };
    widget::tooltip(
        widget::button::icon(icon).on_press(Message::TogglePreviewBackground),
        "Toggle preview background",
        widget::tooltip::Position::Bottom,
    )
    .into()
}

/// What theme previews are drawn on
pub(crate) fn preview_backdrop<'a>(background: PreviewBackground) -> Element<'a, Message> {
    use cosmic::iced::gradient::Linear;

    // One gradient layer filling the preview
    let layer = |gradient: Linear| {
        widget::container(widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .class(cosmic::theme::Container::custom(move |_| {
                widget::container::Style {
                    background: Some(cosmic::iced::Background::Gradient(gradient.into())),
                    ..Default::default()
                }
            }))
    };
    match background {
        // Like a wallpaper: deep blue into mauve, with a warm glow in one corner
        PreviewBackground::Desktop => cosmic::iced::widget::stack![
            layer(
                Linear::new(cosmic::iced::Radians(2.9))
                    .add_stop(0.0, Color::from_rgb8(0x22, 0x38, 0x4a))
                    .add_stop(0.55, Color::from_rgb8(0x4b, 0x47, 0x64))
                    .add_stop(1.0, Color::from_rgb8(0x8b, 0x6a, 0x78)),
            ),
            layer(
                Linear::new(cosmic::iced::Radians(2.2))
                    .add_stop(0.0, Color::from_rgba8(0xd9, 0x9a, 0x6c, 0.0))
                    .add_stop(0.6, Color::from_rgba8(0xd9, 0x9a, 0x6c, 0.0))
                    .add_stop(1.0, Color::from_rgba8(0xd9, 0x9a, 0x6c, 0.55)),
            ),
        ]
        .into(),
        // Shows exactly how transparent the theme is
        PreviewBackground::Checkered => Svg::new(svg::Handle::from_memory(CHECKERBOARD_SVG))
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(cosmic::iced::ContentFit::Cover)
            .into(),
    }
}

/// A clickable theme card showing a short sample drawn with that theme, with
/// buttons to customize it and to preview it on screen
fn theme_card(
    choice: &ThemeChoice,
    name: String,
    theme: &Theme,
    selected: bool,
    icon_style: IconStyle,
    background: PreviewBackground,
) -> Element<'static, Message> {
    let ctrl = KeyModifiers {
        ctrl: true,
        ..Default::default()
    };
    let key = |k: &str| Keystroke::single(k, false);
    let keys = match theme.layout {
        Layout::Keys => vec![
            key("V"),
            Keystroke::combination(&ctrl, "C", false),
            key("LClick"),
        ],
        Layout::Text => vec![
            key("g"),
            key("i"),
            key("t"),
            Keystroke::combination(&ctrl, "S", false),
            key("LClick"),
        ],
    };
    // Right-aligned so the order reads left to right, like typing
    let sample = keystrokes_row::<Message>(
        &keys,
        26.0,
        crate::keystroke::Lifetime::FOREVER,
        theme,
        110.0, // a typewriter line short enough to fit the card
        OverlayPosition::TopRight,
        // The row's length is in key widths; leave room for all of the sample
        12,
        icon_style,
        crate::keystroke::Motion::default(),
    );

    let preview = widget::container(cosmic::iced::widget::stack![
        preview_backdrop(background),
        widget::container(sample)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center),
    ])
    .width(Length::Fill)
    .height(Length::Fixed(CARD_PREVIEW_HEIGHT))
    .clip(true);

    let action = |icon: &'static str, tip: &'static str, message: Message| {
        widget::tooltip(
            widget::button::icon(widget::icon::from_name(icon))
                .extra_small()
                .on_press(message),
            tip,
            widget::tooltip::Position::Top,
        )
    };
    let mut label = widget::Row::new()
        .align_y(Alignment::Center)
        .push(widget::text::body(name).width(Length::Fill));
    // Where the theme's artwork comes from, and its license
    for credit in &theme.credits {
        label = label.push(widget::tooltip(
            widget::button::icon(widget::icon::from_name("help-about-symbolic"))
                .extra_small()
                .on_press(Message::LaunchUrl(credit.url.clone())),
            widget::text::caption(format!(
                "{} by {}, {}",
                credit.work, credit.author, credit.license
            )),
            widget::tooltip::Position::Top,
        ));
    }
    let label = label
        .push(action(
            "edit-symbolic",
            "Customize",
            Message::CustomizeTheme(choice.clone()),
        ))
        .push(action(
            "media-playback-start-symbolic",
            "Preview",
            Message::PreviewTheme(choice.clone()),
        ));

    card(
        widget::Column::new()
            .spacing(4)
            .width(Length::Fill)
            .push(preview)
            .push(label),
        selected,
        false,
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
            .push(widget::text::body("Import a theme…"))
            .push(widget::text::caption(".zip file")),
    )
    .width(Length::Fill)
    // Same height as a theme card: the sample area plus a label row
    .height(Length::Fixed(CARD_PREVIEW_HEIGHT + 32.0))
    .align_x(cosmic::iced::alignment::Horizontal::Center)
    .align_y(cosmic::iced::alignment::Vertical::Center);

    // A dashed outline marks it as a place to add something
    let radius = cosmic::theme::active().cosmic().corner_radii.radius_s[0];
    cosmic::iced::widget::stack![
        card(content, false, true).on_press(Message::ImportTheme),
        crate::widgets::dashed_outline(radius),
    ]
    .into()
}

/// A slider for a duration, with its value in seconds next to it
fn seconds_slider<'a>(
    range: std::ops::RangeInclusive<f32>,
    value: f32,
    on_change: fn(f32) -> Message,
) -> Element<'a, Message> {
    widget::Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .push(
            widget::slider(range, value, on_change)
                .step(0.1_f32)
                .width(Length::Fixed(150.0)),
        )
        .push(
            widget::text::body(format!("{value:.1} s"))
                .width(Length::Fixed(44.0))
                .align_x(cosmic::iced::alignment::Horizontal::Right),
        )
        .into()
}
