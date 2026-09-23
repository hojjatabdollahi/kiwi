//! Keystroke visualization widget

use std::time::Instant;

use crate::config::{IconStyle, OverlayPosition};
use crate::theme::{Fill, Repeats, Theme};

// Bundled font for keystroke text
const FONT_BYTES: &[u8] = include_bytes!("../data/GemunuLibre-VariableFont_wght.ttf");

/// Font name constant for the bundled Gemunu Libre font
pub const FONT_NAME: &str = "Gemunu Libre";
use cosmic::iced::{self, gradient, Background, Border, Color, Length};
use cosmic::widget::container;
use cosmic::widget::svg::{self, Svg};
use cosmic::widget::{self, text};
use cosmic::Element;

// Embed icons at compile time - symbol variants
const ICON_ENTER_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-enter-symbol.svg");
const ICON_BACKSPACE_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-backspace-symbol.svg");
const ICON_SHIFT_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-shift-symbol.svg");
const ICON_CTRL_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-control-symbol.svg");
const ICON_ALT_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-alt-symbol.svg");
const ICON_TAB_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-tab-symbol.svg");
const ICON_SPACE_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-space-symbol.svg");
const ICON_CAPS_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-capslock-symbol.svg");
const ICON_SUPER_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-super-symbol.svg");
const ICON_ESCAPE_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-escape-symbol.svg");
const ICON_DELETE_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-del-symbol.svg");
const ICON_PRTSCR_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-prtscr-symbol.svg");
const ICON_HOME_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-home-symbol.svg");
const ICON_END_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-end-symbol.svg");
const ICON_PGUP_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-pgup-symbol.svg");
const ICON_PGDOWN_SYMBOL: &[u8] = include_bytes!("../data/icons/kiwi-pgdown-symbol.svg");

// Text variants (for keys that have them)
const ICON_SHIFT_TEXT: &[u8] = include_bytes!("../data/icons/kiwi-shift-text.svg");
const ICON_CTRL_TEXT: &[u8] = include_bytes!("../data/icons/kiwi-control-text.svg");
const ICON_ALT_TEXT: &[u8] = include_bytes!("../data/icons/kiwi-alt-text.svg");
const ICON_TAB_TEXT: &[u8] = include_bytes!("../data/icons/kiwi-tab-text.svg");
const ICON_CAPS_TEXT: &[u8] = include_bytes!("../data/icons/kiwi-capslock-text.svg");
const ICON_SUPER_TEXT: &[u8] = include_bytes!("../data/icons/kiwi-super-text.svg");

// Mouse icons
const ICON_LEFT_CLICK: &[u8] = include_bytes!("../data/icons/kiwi-left-click-symbolic.svg");
const ICON_RIGHT_CLICK: &[u8] = include_bytes!("../data/icons/kiwi-right-click-symbolic.svg");
const ICON_MIDDLE_CLICK: &[u8] = include_bytes!("../data/icons/kiwi-middle-click-symbolic.svg");
const ICON_SCROLL_UP: &[u8] = include_bytes!("../data/icons/kiwi-scroll-up-symbolic.svg");
const ICON_SCROLL_DOWN: &[u8] = include_bytes!("../data/icons/kiwi-scroll-down-symbolic.svg");
const ICON_CLICK_DRAG: &[u8] = include_bytes!("../data/icons/kiwi-click-drag-symbol.svg");

// Touchpad gestures
const ICON_TAP: &[u8] = include_bytes!("../data/icons/kiwi-tap.svg");
const ICON_TWO_TAP: &[u8] = include_bytes!("../data/icons/kiwi-two-tap.svg");
const ICON_TWO_UP: &[u8] = include_bytes!("../data/icons/kiwi-two-up.svg");
const ICON_TWO_DOWN: &[u8] = include_bytes!("../data/icons/kiwi-two-down.svg");
const ICON_TWO_LEFT: &[u8] = include_bytes!("../data/icons/kiwi-two-left.svg");
const ICON_TWO_RIGHT: &[u8] = include_bytes!("../data/icons/kiwi-two-right.svg");
const ICON_THREE_TAP: &[u8] = include_bytes!("../data/icons/kiwi-three-tap.svg");
const ICON_THREE_UP: &[u8] = include_bytes!("../data/icons/kiwi-three-up.svg");
const ICON_THREE_DOWN: &[u8] = include_bytes!("../data/icons/kiwi-three-down.svg");
const ICON_FOUR_TAP: &[u8] = include_bytes!("../data/icons/kiwi-four-tap.svg");
const ICON_FOUR_UP: &[u8] = include_bytes!("../data/icons/kiwi-four-up.svg");
const ICON_FOUR_DOWN: &[u8] = include_bytes!("../data/icons/kiwi-four-down.svg");
const ICON_TAP_DRAG: &[u8] = include_bytes!("../data/icons/kiwi-tap-drag.svg");

// Tablet icons
const ICON_PEN_TAP: &[u8] = include_bytes!("../data/icons/kiwi-pen-tap.svg");
const ICON_PEN_DRAG: &[u8] = include_bytes!("../data/icons/kiwi-pen-drag.svg");
const ICON_PEN_BUTTON_1: &[u8] = include_bytes!("../data/icons/kiwi-pen-button-1.svg");
const ICON_PEN_BUTTON_2: &[u8] = include_bytes!("../data/icons/kiwi-pen-button-2.svg");
const ICON_ERASER: &[u8] = include_bytes!("../data/icons/kiwi-eraser.svg");
const ICON_ERASER_DRAG: &[u8] = include_bytes!("../data/icons/kiwi-eraser-drag.svg");
const ICON_PAD: [&[u8]; 8] = [
    include_bytes!("../data/icons/kiwi-pad-1.svg"),
    include_bytes!("../data/icons/kiwi-pad-2.svg"),
    include_bytes!("../data/icons/kiwi-pad-3.svg"),
    include_bytes!("../data/icons/kiwi-pad-4.svg"),
    include_bytes!("../data/icons/kiwi-pad-5.svg"),
    include_bytes!("../data/icons/kiwi-pad-6.svg"),
    include_bytes!("../data/icons/kiwi-pad-7.svg"),
    include_bytes!("../data/icons/kiwi-pad-8.svg"),
];

// Media keys (symbol only)
const ICON_VOLUME_UP: &[u8] = include_bytes!("../data/icons/kiwi-volume-plus-symbol.svg");
const ICON_VOLUME_DOWN: &[u8] = include_bytes!("../data/icons/kiwi-volume-minus-symbol.svg");
const ICON_VOLUME_MUTE: &[u8] = include_bytes!("../data/icons/kiwi-volume-mute-symbol.svg");
const ICON_PLAY_PAUSE: &[u8] = include_bytes!("../data/icons/kiwi-play-pause-symbol.svg");
const ICON_MEDIA: &[u8] = include_bytes!("../data/icons/kiwi-media-symbol.svg");
const ICON_PREV: &[u8] = include_bytes!("../data/icons/kiwi-backward-symbol.svg");
const ICON_NEXT: &[u8] = include_bytes!("../data/icons/kiwi-forward-symbol.svg");
const ICON_AIRPLANE: &[u8] = include_bytes!("../data/icons/kiwi-airplane-symbol.svg");
const ICON_BRIGHTNESS_UP: &[u8] = include_bytes!("../data/icons/kiwi-brightness-high-symbol.svg");
const ICON_BRIGHTNESS_DOWN: &[u8] = include_bytes!("../data/icons/kiwi-brightness-low-symbol.svg");
const ICON_PAUSE: &[u8] = include_bytes!("../data/icons/kiwi-pause-symbol.svg");
const ICON_SCROLL_LOCK: &[u8] = include_bytes!("../data/icons/kiwi-scroll-lock-symbol.svg");
const ICON_INSERT: &[u8] = include_bytes!("../data/icons/kiwi-insert-symbol.svg");

// Special emblems
const ICON_PRESSED_DOWN: &[u8] = include_bytes!("../data/icons/kiwi-pressed-down.svg");

/// Threshold for combining repeated keystrokes (in milliseconds)
pub const REPEAT_THRESHOLD_MS: u128 = 200;

/// Represents a keystroke to display
#[derive(Debug, Clone)]
pub struct Keystroke {
    /// All keys in this keystroke (modifiers + main key)
    pub keys: Vec<String>,
    /// Whether this keystroke is currently being held
    pub pressed: bool,
    /// When this keystroke was created (or last repeated)
    pub timestamp: Instant,
    /// Number of times this keystroke was repeated
    pub count: u32,
}

/// Active modifier keys for a keystroke
#[derive(Debug, Clone, Default)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

impl KeyModifiers {
    /// Returns true if any modifier is active
    pub fn any(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.super_key
    }

    /// Returns the modifier keys as display strings
    pub fn to_parts(&self) -> Vec<String> {
        let mut parts = Vec::new();
        if self.super_key {
            parts.push("Super".to_string());
        }
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("⇧".to_string());
        }
        parts
    }

    /// Create keystroke from just modifiers (for showing held modifiers)
    pub fn to_keystroke(&self, pressed: bool) -> Option<Keystroke> {
        let keys = self.to_parts();
        if keys.is_empty() {
            None
        } else {
            Some(Keystroke {
                keys,
                pressed,
                timestamp: Instant::now(),
                count: 1,
            })
        }
    }
}

impl Keystroke {
    /// Create a single key keystroke
    pub fn single(key: impl Into<String>, pressed: bool) -> Self {
        Self {
            keys: vec![key.into()],
            pressed,
            timestamp: Instant::now(),
            count: 1,
        }
    }

    /// Create a combination keystroke from modifiers + key
    pub fn combination(modifiers: &KeyModifiers, key: impl Into<String>, pressed: bool) -> Self {
        let mut keys = modifiers.to_parts();
        keys.push(key.into());
        Self {
            keys,
            pressed,
            timestamp: Instant::now(),
            count: 1,
        }
    }

    /// Check if this keystroke matches another (same keys)
    pub fn matches(&self, other: &Self) -> bool {
        self.keys == other.keys
    }

    /// Check if this keystroke can be merged with a new one (same keys, within threshold)
    pub fn can_merge(&self, other: &Self) -> bool {
        self.matches(other) && self.timestamp.elapsed().as_millis() < REPEAT_THRESHOLD_MS
    }

    /// Increment the repeat count and update timestamp
    pub fn increment(&mut self) {
        self.count += 1;
        self.timestamp = Instant::now();
    }

    /// Create from just modifiers
    pub fn from_modifiers(modifiers: &KeyModifiers, pressed: bool) -> Option<Self> {
        modifiers.to_keystroke(pressed)
    }

    /// Check if this is a combination (multiple keys)
    pub fn is_combination(&self) -> bool {
        self.keys.len() > 1
    }

    /// Get age in seconds
    pub fn age_secs(&self) -> f32 {
        self.timestamp.elapsed().as_secs_f32()
    }

    /// Check if this keystroke has expired (older than fade duration)
    pub fn is_expired(&self, fade_duration_secs: f32) -> bool {
        self.age_secs() >= fade_duration_secs
    }

    /// Get opacity based on age (1.0 = new, 0.0 = fully faded)
    /// Stays at 1.0 for the first 70% of duration, then fades in the last 30% with easing
    pub fn opacity(&self, fade_duration_secs: f32) -> f32 {
        if self.pressed {
            1.0 // Pressed keys are always fully visible
        } else {
            let age = self.age_secs();
            let fade_start = fade_duration_secs * 0.7; // Start fading at 70%

            if age >= fade_duration_secs {
                0.0
            } else if age <= fade_start {
                1.0 // Full opacity for first 70%
            } else {
                // Fade from 1.0 to 0.0 in the last 30% with ease-out
                let fade_phase = fade_duration_secs - fade_start;
                let t = (age - fade_start) / fade_phase; // 0.0 -> 1.0
                let eased = ease_in_cubic(t);
                1.0 - eased
            }
        }
    }
}

/// Ease-in cubic: slow start, fast end
/// t: 0.0 -> 1.0, returns 0.0 -> 1.0
fn ease_in_cubic(t: f32) -> f32 {
    t.powi(3)
}

// Layout constants (colors and shapes come from the theme)
const PLUS_WIDTH: f32 = 10.0; // Width for the "+" separator

/// Calculate font size based on key size
fn font_size_for_key(key_size: f32) -> f32 {
    key_size * 0.55
}

/// Calculate icon size based on key size
fn icon_size_for_key(key_size: f32) -> f32 {
    key_size * 0.65
}

/// Calculate plus font size based on key size
fn plus_font_size_for_key(key_size: f32) -> f32 {
    key_size * 0.4
}

/// Returns (icon_data, should_apply_color) based on key and icon style preference
/// The label to show when a button was moved while held (e.g. "LClick" -> "LDrag")
pub fn drag_variant(key: &str) -> Option<&'static str> {
    match key {
        "LClick" => Some("LDrag"),
        "Tap" => Some("TapDrag"),
        "PenTap" => Some("PenDrag"),
        "Eraser" => Some("EraserDrag"),
        _ => None,
    }
}

fn get_icon_for_key_with_style(key: &str, icon_style: IconStyle) -> Option<(&'static [u8], bool)> {
    let use_text = matches!(icon_style, IconStyle::Text);

    if let Some(n) = key
        .strip_prefix("Pad")
        .and_then(|n| n.parse::<usize>().ok())
    {
        return ICON_PAD.get(n.wrapping_sub(1)).map(|icon| (*icon, true));
    }

    match key {
        // Keys with symbol and text variants
        "↵" => Some((ICON_ENTER_SYMBOL, true)),
        "⇧" => Some((
            if use_text {
                ICON_SHIFT_TEXT
            } else {
                ICON_SHIFT_SYMBOL
            },
            true,
        )),
        "Ctrl" => Some((
            if use_text {
                ICON_CTRL_TEXT
            } else {
                ICON_CTRL_SYMBOL
            },
            true,
        )),
        "Alt" => Some((
            if use_text {
                ICON_ALT_TEXT
            } else {
                ICON_ALT_SYMBOL
            },
            true,
        )),
        "Tab" => Some((
            if use_text {
                ICON_TAB_TEXT
            } else {
                ICON_TAB_SYMBOL
            },
            true,
        )),
        "Caps" => Some((
            if use_text {
                ICON_CAPS_TEXT
            } else {
                ICON_CAPS_SYMBOL
            },
            true,
        )),
        "Super" => Some(if use_text {
            (ICON_SUPER_TEXT, true)
        } else {
            (ICON_SUPER_SYMBOL, false)
        }),
        "Esc" => Some((ICON_ESCAPE_SYMBOL, true)),
        "PrtSc" => Some((ICON_PRTSCR_SYMBOL, true)),
        "Home" => Some((ICON_HOME_SYMBOL, true)),
        "End" => Some((ICON_END_SYMBOL, true)),
        "PgUp" => Some((ICON_PGUP_SYMBOL, true)),
        "PgDn" => Some((ICON_PGDOWN_SYMBOL, true)),
        // Symbol-only keys
        "⌫" => Some((ICON_BACKSPACE_SYMBOL, true)),
        "␣" => Some((ICON_SPACE_SYMBOL, true)),
        "Del" => Some((ICON_DELETE_SYMBOL, true)),
        // Media keys (symbol only)
        "VolUp" => Some((ICON_VOLUME_UP, true)),
        "VolDown" => Some((ICON_VOLUME_DOWN, true)),
        "Mute" => Some((ICON_VOLUME_MUTE, true)),
        "Play" => Some((ICON_PLAY_PAUSE, true)),
        "Media" => Some((ICON_MEDIA, true)),
        "Prev" => Some((ICON_PREV, true)),
        "Next" => Some((ICON_NEXT, true)),
        "Airplane" => Some((ICON_AIRPLANE, true)),
        "BriUp" => Some((ICON_BRIGHTNESS_UP, true)),
        "BriDown" => Some((ICON_BRIGHTNESS_DOWN, true)),
        "Pause" => Some((ICON_PAUSE, true)),
        "ScrLk" => Some((ICON_SCROLL_LOCK, true)),
        "Ins" => Some((ICON_INSERT, true)),
        // Mouse
        "LClick" => Some((ICON_LEFT_CLICK, true)),
        "RClick" => Some((ICON_RIGHT_CLICK, true)),
        "MClick" => Some((ICON_MIDDLE_CLICK, true)),
        "ScrollUp" => Some((ICON_SCROLL_UP, true)),
        "ScrollDown" => Some((ICON_SCROLL_DOWN, true)),
        // Touchpad gestures
        "Tap" => Some((ICON_TAP, true)),
        "2Tap" => Some((ICON_TWO_TAP, true)),
        "2Up" => Some((ICON_TWO_UP, true)),
        "2Down" => Some((ICON_TWO_DOWN, true)),
        "2Left" => Some((ICON_TWO_LEFT, true)),
        "2Right" => Some((ICON_TWO_RIGHT, true)),
        "3Tap" => Some((ICON_THREE_TAP, true)),
        "3Up" => Some((ICON_THREE_UP, true)),
        "3Down" => Some((ICON_THREE_DOWN, true)),
        "4Tap" => Some((ICON_FOUR_TAP, true)),
        "4Up" => Some((ICON_FOUR_UP, true)),
        "4Down" => Some((ICON_FOUR_DOWN, true)),
        // Drag gestures
        "LDrag" => Some((ICON_CLICK_DRAG, true)),
        "TapDrag" => Some((ICON_TAP_DRAG, true)),
        // Tablet
        "PenTap" => Some((ICON_PEN_TAP, true)),
        "PenDrag" => Some((ICON_PEN_DRAG, true)),
        "Pen1" => Some((ICON_PEN_BUTTON_1, true)),
        "Pen2" | "Pen3" => Some((ICON_PEN_BUTTON_2, true)),
        "Eraser" => Some((ICON_ERASER, true)),
        "EraserDrag" => Some((ICON_ERASER_DRAG, true)),
        _ => None,
    }
}

/// Load the bundled font (call once at startup)
pub fn load_font() -> cosmic::iced::Task<Result<(), cosmic::iced::font::Error>> {
    cosmic::iced::font::load(std::borrow::Cow::Borrowed(FONT_BYTES))
}

/// Creates the content element for a key - either an icon or text
fn key_content<'a, M: 'a>(
    key: &str,
    text_color: Color,
    key_size: f32,
    icon_style: IconStyle,
    theme: &Theme,
) -> Element<'a, M> {
    let icon_size = icon_size_for_key(key_size);
    let font_size = font_size_for_key(key_size);

    // The theme's own icon wins, then Kiwi's built-in one, then the key name as text
    let icon = theme
        .icon(key)
        .map(|handle| (handle.clone(), theme.recolor_icons))
        .or_else(|| {
            get_icon_for_key_with_style(key, icon_style)
                .map(|(data, apply_color)| (svg::Handle::from_memory(data), apply_color))
        });

    if let Some((handle, apply_color)) = icon {
        let mut svg = Svg::new(handle)
            .width(Length::Fixed(icon_size))
            .height(Length::Fixed(icon_size));

        if apply_color {
            svg = svg.class(cosmic::theme::Svg::custom(move |_| svg::Style {
                color: Some(text_color),
            }));
        }

        svg.into()
    } else {
        // Use bundled Gemunu Libre font with bold weight
        text::Text::new(key.to_string())
            .size(font_size)
            .font(cosmic::iced::Font {
                family: cosmic::iced::font::Family::Name(FONT_NAME),
                weight: cosmic::iced::font::Weight::Bold,
                ..Default::default()
            })
            .class(cosmic::theme::Text::Color(text_color))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    }
}

/// Creates the pressed-down emblem - a small icon at bottom of a pressed key
fn pressed_emblem<'a, M: 'a>(color: Color, emblem_size: f32) -> Element<'a, M> {
    let handle = svg::Handle::from_memory(ICON_PRESSED_DOWN);
    Svg::new(handle)
        .width(Length::Fixed(emblem_size))
        .height(Length::Fixed(emblem_size))
        .class(cosmic::theme::Svg::custom(move |_| svg::Style {
            color: Some(color),
        }))
        .into()
}

/// Wraps key content with the pressed emblem overlaid in the corner if pressed
/// Uses a stack so the emblem doesn't affect the layout
fn key_content_with_emblem<'a, M: 'a>(
    key: &str,
    text_color: Color,
    key_size: f32,
    pressed: bool,
    icon_style: IconStyle,
    theme: &Theme,
) -> Element<'a, M> {
    let content = key_content(key, text_color, key_size, icon_style, theme);

    if pressed {
        let emblem_size = key_size * 0.22; // Smaller emblem
                                           // Use stack to overlay emblem in bottom-right corner without changing layout
        cosmic::iced::widget::stack![
            // Main content centered
            widget::container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
            // Emblem at bottom-right corner (overlaid)
            widget::container(pressed_emblem::<M>(text_color, emblem_size))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding([0, 2, 2, 0]) // small padding from corner
        ]
        .into()
    } else {
        content
    }
}

/// Wraps a widget with a badge area.
/// Badge is above for bottom positions, below for top positions.
/// Always reserves space for the badge to prevent layout shifts when count changes.
fn wrap_with_badge_area<'a, M: 'a>(
    widget: Element<'a, M>,
    count: u32,
    key_size: f32,
    count_color: Color,
    count_bg: Color,
    position: OverlayPosition,
) -> Element<'a, M> {
    let count_font_size = key_size * 0.3;
    // Badge height = font + padding (approximately)
    let badge_height = count_font_size + 4.0;
    let spacing = (key_size * 0.05) as u16;

    // Always create a column with widget + badge area
    let badge: Element<'a, M> = if count > 1 {
        widget::container(
            text::Text::new(format!("x{}", count))
                .size(count_font_size)
                .class(cosmic::theme::Text::Color(count_color))
                .align_x(iced::alignment::Horizontal::Center),
        )
        .padding([2, 6])
        .class(cosmic::theme::Container::custom(move |_| {
            container::Style {
                background: Some(Background::Color(count_bg)),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: (count_font_size * 0.6).into(),
                },
                ..Default::default()
            }
        }))
        .into()
    } else {
        // Invisible placeholder to reserve space
        widget::Space::new()
            .height(Length::Fixed(badge_height))
            .into()
    };

    // For bottom positions, badge goes above; for top positions, badge goes below
    let is_bottom = matches!(
        position,
        OverlayPosition::BottomLeft | OverlayPosition::BottomRight | OverlayPosition::BottomCenter
    );

    let mut col = widget::Column::new()
        .align_x(iced::Alignment::Center)
        .spacing(spacing);
    if is_bottom {
        col = col.push(badge).push(widget);
    } else {
        col = col.push(widget).push(badge);
    }
    col.into()
}

/// Renders a keystroke widget with opacity based on age
///
/// - Single key: square with border
/// - Combination: outer container with border, inner key boxes (no border) + "+" separators
/// - If `fade_enabled` is false, the widget will always be fully opaque
pub fn keystroke_widget<'a, M: 'a>(
    keystroke: &Keystroke,
    key_size: f32,
    fade_duration: f32,
    theme: &Theme,
    fade_enabled: bool,
    position: OverlayPosition,
    icon_style: IconStyle,
) -> Element<'a, M> {
    let opacity = if fade_enabled {
        keystroke.opacity(fade_duration)
    } else {
        1.0
    };
    let plus_font_size = plus_font_size_for_key(key_size);
    let style = theme.key;
    let fade = |color: Color| Color {
        a: color.a * opacity,
        ..color
    };

    let fill = if keystroke.pressed {
        Fill::Solid(style.pressed)
    } else if keystroke.is_combination() {
        style.combo_background.unwrap_or(style.background)
    } else {
        style.background
    };
    let background = fill_background(fill, opacity);

    let border = Border {
        color: fade(style.border.color.0),
        width: style.border.width,
        radius: style.radius.into(),
    };
    let text_color = fade(style.text.0);
    let plus_color = fade(style.separator.0);

    // Each key is a key_size square; combinations put a "+" between them
    let mut parts: Vec<Element<'a, M>> = Vec::new();
    for (i, key) in keystroke.keys.iter().enumerate() {
        if i > 0 {
            // Add "+" separator (fixed width, centered)
            parts.push(
                widget::container(
                    text::Text::new("+")
                        .size(plus_font_size)
                        .class(cosmic::theme::Text::Color(plus_color))
                        .align_x(iced::alignment::Horizontal::Center)
                        .align_y(iced::alignment::Vertical::Center),
                )
                .width(Length::Fixed(PLUS_WIDTH))
                .height(Length::Fixed(key_size))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .into(),
            );
        }
        // Centered content: text or icon, plus the emblem if pressed
        parts.push(
            widget::container(key_content_with_emblem(
                key,
                text_color,
                key_size,
                keystroke.pressed,
                icon_style,
                theme,
            ))
            .width(Length::Fixed(key_size))
            .height(Length::Fixed(key_size))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .into(),
        );
    }

    if style.repeats == Repeats::Inline && keystroke.count > 1 {
        parts.push(
            widget::container(
                text::Text::new(format!("×{}", keystroke.count))
                    .size(key_size * 0.3)
                    .class(cosmic::theme::Text::Color(plus_color)),
            )
            .height(Length::Fixed(key_size))
            .padding([0.0, key_size * 0.15, 0.0, 0.0])
            .align_y(iced::alignment::Vertical::Center)
            .into(),
        );
    }

    let key_widget: Element<'a, M> = widget::container(
        widget::row::with_children(parts)
            .spacing(0)
            .align_y(iced::Alignment::Center),
    )
    .height(Length::Fixed(key_size))
    .class(cosmic::theme::Container::custom(move |_| {
        container::Style {
            background: Some(background),
            border,
            ..Default::default()
        }
    }))
    .into();

    match style.repeats {
        // Always reserve space for the badge to prevent relayout
        Repeats::Badge => wrap_with_badge_area(
            key_widget,
            keystroke.count,
            key_size,
            fade(style.badge_text.0),
            fade(style.badge_background.0),
            position,
        ),
        Repeats::Inline | Repeats::Hidden => key_widget,
    }
}

/// A theme fill as a widget background, with its alpha scaled by `opacity`
fn fill_background(fill: Fill, opacity: f32) -> Background {
    let fade = |color: Color| Color {
        a: color.a * opacity,
        ..color
    };
    match fill {
        Fill::Solid(color) => Background::Color(fade(color.0)),
        Fill::Gradient(start, end) => {
            let grad = gradient::Linear::new(std::f32::consts::PI / 4.0) // 45 degree angle
                .add_stop(0.0, fade(start.0))
                .add_stop(1.0, fade(end.0));
            Background::Gradient(gradient::Gradient::Linear(grad))
        }
    }
}

/// Renders a row of keystrokes.
/// Filters out expired keystrokes and limits to history_count keystrokes.
pub fn keystrokes_row<'a, M: 'a + Clone>(
    keystrokes: &[Keystroke],
    key_size: f32,
    fade_duration: f32,
    theme: &Theme,
    position: OverlayPosition,
    history_count: usize,
    icon_style: IconStyle,
) -> Element<'a, M> {
    // Filter non-expired keystrokes, newest first, limit count
    let mut visible_keystrokes: Vec<&Keystroke> = keystrokes
        .iter()
        .rev()
        .filter(|k| !k.is_expired(fade_duration))
        .take(history_count)
        .collect();

    // Reverse so oldest is first (left side for left-aligned, right side for right-aligned)
    visible_keystrokes.reverse();

    // The rail fades out along with its newest key
    let rail_opacity = visible_keystrokes
        .iter()
        .map(|k| k.opacity(fade_duration))
        .fold(0.0, f32::max);

    let children: Vec<Element<'a, M>> = visible_keystrokes
        .into_iter()
        .map(|k| {
            keystroke_widget(
                k,
                key_size,
                fade_duration,
                theme,
                true,
                position,
                icon_style,
            )
        })
        .collect();

    // Determine if we need to reverse the order for right-aligned positions
    // (newest should appear on the right edge)
    let is_right_aligned = matches!(
        position,
        OverlayPosition::TopRight | OverlayPosition::BottomRight | OverlayPosition::BottomCenter
    );

    let is_bottom = matches!(
        position,
        OverlayPosition::BottomLeft | OverlayPosition::BottomRight | OverlayPosition::BottomCenter
    );

    // For right-aligned: oldest on left, newest on right (natural order after reverse)
    // For left-aligned: oldest on right, newest on left (need to reverse display)
    let ordered_children = if is_right_aligned {
        children
    } else {
        children.into_iter().rev().collect()
    };

    let Some(rail) = theme.rail else {
        return widget::row::with_children(ordered_children)
            .spacing(theme.key.gap)
            .align_y(if is_bottom {
                iced::Alignment::End
            } else {
                iced::Alignment::Start
            })
            .into();
    };

    let fade = |color: Color| Color {
        a: color.a * rail_opacity,
        ..color
    };

    // Thin lines between keys
    let children: Vec<Element<'a, M>> = match rail.divider {
        None => ordered_children,
        Some(color) => {
            let color = fade(color.0);
            let mut with_dividers = Vec::new();
            for (i, child) in ordered_children.into_iter().enumerate() {
                if i > 0 {
                    with_dividers.push(
                        widget::container(widget::Space::new())
                            .width(Length::Fixed(1.0))
                            .height(Length::Fixed(key_size))
                            .class(cosmic::theme::Container::custom(move |_| {
                                container::Style {
                                    background: Some(Background::Color(color)),
                                    ..Default::default()
                                }
                            }))
                            .into(),
                    );
                }
                with_dividers.push(child);
            }
            with_dividers
        }
    };

    let background = fill_background(rail.background, rail_opacity);
    let border = Border {
        color: fade(rail.border.color.0),
        width: rail.border.width,
        radius: rail.radius.into(),
    };
    widget::container(
        widget::row::with_children(children)
            .spacing(theme.key.gap)
            .align_y(iced::Alignment::Center),
    )
    .padding(rail.padding)
    .class(cosmic::theme::Container::custom(move |_| {
        container::Style {
            background: Some(background),
            border,
            ..Default::default()
        }
    }))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tablet_labels_resolve() {
        let icon = |k| get_icon_for_key_with_style(k, IconStyle::Symbol).map(|(bytes, _)| bytes);
        assert_eq!(icon("Pad1"), Some(ICON_PAD[0]));
        assert_eq!(icon("Pad8"), Some(ICON_PAD[7]));
        assert!(icon("Pad0").is_none() && icon("Pad9").is_none() && icon("Padx").is_none());
        assert_eq!(icon("Pen3"), icon("Pen2"));
        assert_eq!(drag_variant("PenTap"), Some("PenDrag"));
        assert_eq!(drag_variant("Eraser"), Some("EraserDrag"));
        assert_eq!(drag_variant("Pen1"), None);
    }
}
