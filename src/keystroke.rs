//! Keystroke visualization widget

use std::time::Instant;

use crate::config::{IconStyle, OverlayPosition, Palette, PaletteType};

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

// Style constants
const BORDER_WIDTH: f32 = 1.0;
const BORDER_RADIUS: f32 = 6.0;
const PLUS_WIDTH: f32 = 10.0; // Width for the "+" separator
const KEY_GAP: f32 = 4.0;

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
fn get_icon_for_key_with_style(key: &str, icon_style: IconStyle) -> Option<(&'static [u8], bool)> {
    let use_text = matches!(icon_style, IconStyle::Text);

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
) -> Element<'a, M> {
    let icon_size = icon_size_for_key(key_size);
    let font_size = font_size_for_key(key_size);

    if let Some((icon_data, apply_color)) = get_icon_for_key_with_style(key, icon_style) {
        // Use embedded SVG icon
        let handle = svg::Handle::from_memory(icon_data);
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
) -> Element<'a, M> {
    let content = key_content(key, text_color, key_size, icon_style);

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
    palette_type: PaletteType,
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
    let palette = Palette::from_type(palette_type).with_opacity(opacity);

    let background = if keystroke.pressed {
        Background::Color(palette.bg_pressed)
    } else if let Some(gradient_end) = palette.bg_gradient_end {
        // Use gradient for frosted glass effect
        let grad = gradient::Linear::new(std::f32::consts::PI / 4.0) // 45 degree angle
            .add_stop(0.0, palette.bg_released)
            .add_stop(1.0, gradient_end);
        Background::Gradient(gradient::Gradient::Linear(grad))
    } else {
        Background::Color(palette.bg_released)
    };

    let border_color = palette.border;
    let text_color = palette.text;
    let plus_color = palette.plus;
    let count_color = palette.count;

    if keystroke.is_combination() {
        // Combination: outer border, inner key boxes without borders
        let mut row_children: Vec<Element<'a, M>> = Vec::new();

        for (i, key) in keystroke.keys.iter().enumerate() {
            if i > 0 {
                // Add "+" separator (fixed width, centered)
                row_children.push(
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
            // Key box: key_size square, no border, centered content (text or icon + emblem if pressed)
            row_children.push(
                widget::container(key_content_with_emblem(
                    key,
                    text_color,
                    key_size,
                    keystroke.pressed,
                    icon_style,
                ))
                .width(Length::Fixed(key_size))
                .height(Length::Fixed(key_size))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .into(),
            );
        }

        // Outer container with border, no padding, no spacing (children handle their own size)
        let combo_widget = widget::container(
            widget::row::with_children(row_children)
                .spacing(0)
                .align_y(iced::Alignment::Center),
        )
        .height(Length::Fixed(key_size))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .class(cosmic::theme::Container::custom(move |_| {
            container::Style {
                background: Some(background),
                border: Border {
                    color: border_color,
                    width: BORDER_WIDTH,
                    radius: BORDER_RADIUS.into(),
                },
                ..Default::default()
            }
        }));

        // Wrap with badge area (always reserve space for badge to prevent relayout)
        wrap_with_badge_area(
            combo_widget.into(),
            keystroke.count,
            key_size,
            count_color,
            palette.count_bg,
            position,
        )
    } else {
        // Single key: square with border
        let key_widget = widget::container(key_content_with_emblem(
            &keystroke.keys[0],
            text_color,
            key_size,
            keystroke.pressed,
            icon_style,
        ))
        .width(Length::Fixed(key_size))
        .height(Length::Fixed(key_size))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .class(cosmic::theme::Container::custom(move |_| {
            container::Style {
                background: Some(background),
                border: Border {
                    color: border_color,
                    width: BORDER_WIDTH,
                    radius: BORDER_RADIUS.into(),
                },
                ..Default::default()
            }
        }));

        // Wrap with badge area (always reserve space for badge to prevent relayout)
        wrap_with_badge_area(
            key_widget.into(),
            keystroke.count,
            key_size,
            count_color,
            palette.count_bg,
            position,
        )
    }
}
/// Renders a row of keystrokes.
/// Filters out expired keystrokes and limits to history_count keystrokes.
pub fn keystrokes_row<'a, M: 'a + Clone>(
    keystrokes: &[Keystroke],
    key_size: f32,
    fade_duration: f32,
    palette_type: PaletteType,
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

    let children: Vec<Element<'a, M>> = visible_keystrokes
        .into_iter()
        .map(|k| {
            keystroke_widget(
                k,
                key_size,
                fade_duration,
                palette_type,
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

    widget::row::with_children(ordered_children)
        .spacing(KEY_GAP)
        .align_y(if is_bottom {
            iced::Alignment::End
        } else {
            iced::Alignment::Start
        })
        .into()
}
