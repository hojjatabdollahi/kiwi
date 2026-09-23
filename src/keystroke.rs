//! Keystroke visualization widget

use std::time::Instant;

use crate::config::{IconStyle, OverlayPosition};
use crate::theme::{self, Expiry, Fill, Layout, RailStyle, RailVisibility, Repeats, Theme};
use crate::widgets::Reveal;

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

/// Pressing the same key again within this long (in milliseconds) counts up on
/// the last keystroke instead of adding another, so steady repeated clicking or
/// typing doesn't keep pushing the row along
pub const REPEAT_THRESHOLD_MS: u128 = 800;

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
    /// Keys of a held combination that were already let go, one bit per key
    /// (bit 0 is the first). They're drawn as empty space so nothing shifts.
    pub released_parts: u8,
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
                released_parts: 0,
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
            released_parts: 0,
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
            released_parts: 0,
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

    /// Check if this keystroke has expired (older than its lifetime)
    pub fn is_expired(&self, life: Lifetime) -> bool {
        self.age_secs() >= life.total()
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

/// Buttons and the label shown when they're moved while held
const DRAGS: &[(&str, &str)] = &[
    ("LClick", "LDrag"),
    ("Tap", "TapDrag"),
    ("PenTap", "PenDrag"),
    ("Eraser", "EraserDrag"),
];

/// The label to show when a button was moved while held (e.g. "LClick" -> "LDrag")
pub fn drag_variant(key: &str) -> Option<&'static str> {
    DRAGS
        .iter()
        .find(|(button, _)| *button == key)
        .map(|(_, drag)| *drag)
}

/// The button a drag label belongs to (e.g. "LDrag" -> "LClick")
pub fn dragged_button(key: &str) -> Option<&'static str> {
    DRAGS
        .iter()
        .find(|(_, drag)| *drag == key)
        .map(|(button, _)| *button)
}

/// Every key Kiwi has a built-in icon for, which themes can replace
pub const ICON_KEYS: &[&str] = &[
    "↵",
    "⇧",
    "Ctrl",
    "Alt",
    "Tab",
    "Caps",
    "Super",
    "Esc",
    "PrtSc",
    "Home",
    "End",
    "PgUp",
    "PgDn",
    "⌫",
    "␣",
    "Del",
    "Ins",
    "Pause",
    "ScrLk",
    "VolUp",
    "VolDown",
    "Mute",
    "Play",
    "Media",
    "Prev",
    "Next",
    "Airplane",
    "BriUp",
    "BriDown",
    "LClick",
    "RClick",
    "MClick",
    "LDrag",
    "ScrollUp",
    "ScrollDown",
    "Tap",
    "TapDrag",
    "2Tap",
    "2Up",
    "2Down",
    "2Left",
    "2Right",
    "3Tap",
    "3Up",
    "3Down",
    "4Tap",
    "4Up",
    "4Down",
    "PenTap",
    "PenDrag",
    "Pen1",
    "Pen2",
    "Pen3",
    "Eraser",
    "EraserDrag",
    "Pad1",
    "Pad2",
    "Pad3",
    "Pad4",
    "Pad5",
    "Pad6",
    "Pad7",
    "Pad8",
];

/// Returns (icon_data, should_apply_color) based on key and icon style preference
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
        // Tinting an SVG only sets its color, not its alpha, so the icon fades
        // with the key through the opacity instead
        let mut svg = Svg::new(handle)
            .width(Length::Fixed(icon_size))
            .height(Length::Fixed(icon_size))
            .opacity(text_color.a);

        if apply_color {
            svg = svg.class(cosmic::theme::Svg::custom(move |_| svg::Style {
                color: Some(text_color),
            }));
        }

        svg.into()
    } else {
        // The theme's font (Kiwi's bundled Gemunu Libre unless it picks another)
        text::Text::new(key.to_string())
            .size(font_size)
            .font(theme.font())
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
        // The tint doesn't carry alpha, so fade through the opacity
        .opacity(color.a)
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

/// Height of a keycap in a `key_size` row; the room left below lets a held cap drop
fn cap_height(key_size: f32) -> f32 {
    key_size * 0.94
}

/// The cap `key` is drawn as, if the theme draws keycaps: its own, or the blank one
fn cap_for<'t>(theme: &'t Theme, key: &str) -> Option<&'t theme::Cap> {
    if theme.caps.is_empty() {
        return None;
    }
    theme.cap(key).or(theme.blank_cap(key))
}

/// Width of one key: a cap is as wide as its shape, anything else is square
fn part_width(theme: &Theme, key: &str, key_size: f32) -> f32 {
    cap_for(theme, key).map_or(key_size, |cap| cap.aspect * cap_height(key_size))
}

/// A key drawn as a keycap: the theme's own cap for it, or the blank cap with the
/// usual label on top. A held cap sits a little lower. A key with neither gets a
/// box of its own, like the other themes draw.
fn keycap<'a, M: 'a>(
    key: &str,
    key_size: f32,
    opacity: f32,
    text_color: Color,
    pressed: bool,
    icon_style: IconStyle,
    theme: &Theme,
) -> Element<'a, M> {
    let own = theme.cap(key);
    let Some(cap) = own.or(theme.blank_cap(key)) else {
        let fill = if pressed {
            Fill::Solid(theme.key.pressed)
        } else {
            theme.key.background
        };
        return with_border(
            widget::container(key_content_with_emblem(
                key, text_color, key_size, pressed, icon_style, theme,
            ))
            .width(Length::Fixed(key_size))
            .height(Length::Fixed(key_size)),
            fill_background(fill, opacity),
            theme.key.border,
            theme.key.radius,
            opacity,
        );
    };

    let height = cap_height(key_size);
    let width = cap.aspect * height;
    let mut layers: Vec<Element<'a, M>> = vec![Svg::new(cap.svg.clone())
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .opacity(opacity)
        .into()];
    if own.is_none() {
        // Centered on the theme's label spot, not always the middle
        let spot = theme.cap_label.clamp(0.0, 1.0);
        let top = (2.0 * spot - 1.0).max(0.0) * height;
        let bottom = (1.0 - 2.0 * spot).max(0.0) * height;
        layers.push(
            widget::container(key_content(key, text_color, key_size, icon_style, theme))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([top, 0.0, bottom, 0.0])
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .into(),
        );
    }
    if pressed {
        layers.push(
            widget::container(pressed_emblem::<M>(text_color, key_size * 0.22))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding([0.0, width * 0.08, key_size * 0.08, 0.0])
                .into(),
        );
    }

    widget::container(cosmic::iced::widget::Stack::with_children(layers))
        .width(Length::Fixed(width))
        .height(Length::Fixed(key_size))
        .align_y(if pressed {
            iced::alignment::Vertical::Bottom
        } else {
            iced::alignment::Vertical::Top
        })
        .into()
}

/// Puts the repeat count ("×3") in a small pill in the key's top-right corner,
/// on top of the key, so a count appearing doesn't change the key's size
fn with_badge<'a, M: 'a>(
    widget: Element<'a, M>,
    count: u32,
    key_size: f32,
    count_color: Color,
    count_bg: Color,
) -> Element<'a, M> {
    if count <= 1 {
        return widget;
    }
    let font_size = key_size * 0.24;
    let badge = widget::container(
        text::Text::new(format!("×{count}"))
            .size(font_size)
            .class(cosmic::theme::Text::Color(count_color)),
    )
    .padding([0.0, font_size * 0.4])
    .class(cosmic::theme::Container::custom(move |_| {
        container::Style {
            background: Some(Background::Color(count_bg)),
            border: Border {
                radius: font_size.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }));
    // Centered on the key's top-right corner, clear of the icon. It's only drawn
    // there, so it never makes the key or the rail any bigger
    cosmic::iced::widget::stack![
        widget,
        widget::container(crate::widgets::Offset::new(badge, 0.5, -0.5))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Top),
    ]
    .into()
}

/// Renders a keystroke widget at the given opacity
///
/// - Single key: square with border
/// - Combination: outer container with border, inner key boxes (no border) + "+" separators
fn keystroke_widget<'a, M: 'a>(
    keystroke: &Keystroke,
    key_size: f32,
    opacity: f32,
    theme: &Theme,
    icon_style: IconStyle,
) -> Element<'a, M> {
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

    let text_color = fade(style.text.0);
    let plus_color = fade(style.separator.0);
    // A keycap theme draws each key as its own cap, with no box around them
    let caps = !theme.caps.is_empty();

    // Each key is a key_size square (or a cap as wide as its shape); combinations
    // put a "+" between them
    let mut parts: Vec<Element<'a, M>> = Vec::new();
    let released = |i: usize| i < 8 && keystroke.released_parts & (1 << i) != 0;
    for (i, key) in keystroke.keys.iter().enumerate() {
        // Keys already let go leave their space empty, and so does the "+" beside them
        if i > 0 && (released(i) || released(i - 1)) {
            parts.push(widget::Space::new().width(Length::Fixed(PLUS_WIDTH)).into());
        } else if i > 0 {
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
        if released(i) {
            let width = part_width(theme, key, key_size);
            parts.push(widget::Space::new().width(Length::Fixed(width)).into());
            continue;
        }
        if caps {
            parts.push(keycap(
                key,
                key_size,
                opacity,
                text_color,
                keystroke.pressed,
                icon_style,
                theme,
            ));
            continue;
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

    let row = widget::container(
        widget::row::with_children(parts)
            .spacing(0)
            .align_y(iced::Alignment::Center),
    )
    .height(Length::Fixed(key_size));
    let key_widget = if caps {
        row.into()
    } else {
        with_border(row, background, style.border, style.radius, opacity)
    };

    match style.repeats {
        Repeats::Badge => with_badge(
            key_widget,
            keystroke.count,
            key_size,
            fade(style.badge_text.0),
            fade(style.badge_background.0),
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
        Fill::Gradient(start, end, angle) => {
            let (start_at, end_at) = theme::stop_positions(&start, &end);
            let grad = gradient::Linear::new(iced::Radians(angle.to_radians()))
                .add_stop(start_at, fade(start.color.0))
                .add_stop(end_at, fade(end.color.0));
            Background::Gradient(gradient::Gradient::Linear(grad))
        }
    }
}

/// Renders a row of keystrokes.
/// Filters out expired keystrokes and limits to history_count keystrokes.
pub fn keystrokes_row<'a, M: 'a + Clone>(
    keystrokes: &[Keystroke],
    key_size: f32,
    life: Lifetime,
    theme: &Theme,
    line_width: f32,
    position: OverlayPosition,
    history_count: usize,
    icon_style: IconStyle,
    motion: Motion,
) -> Element<'a, M> {
    if theme.layout == Layout::Text {
        return typewriter_line(keystrokes, key_size, life, theme, line_width, icon_style);
    }

    // Keys run inward from the screen edge, newest at the edge. A held key sits
    // there while it builds up and stays in the same place once it's let go.
    // Keys only ever move away from the edge, or go away.
    let visible: Vec<&Keystroke> = keystrokes.iter().filter(|k| !k.is_expired(life)).collect();
    // The row shows this much, measured in single keys; older keys slide out past
    // its far edge, fading as they cross it
    let window = history_count as f32 * (key_size + theme.key.gap);
    let visibility = theme.rail.map(|rail| rail.visibility);
    let Some(newest) = visible.last() else {
        // A rail that's always visible stays, empty, at its full length
        return match theme.rail {
            Some(rail) if rail.visibility == RailVisibility::Always => on_rail(
                widget::Space::new()
                    .width(Length::Fixed(window))
                    .height(Length::Fixed(key_size))
                    .into(),
                &rail,
                1.0,
            ),
            _ => widget::Space::new().into(),
        };
    };

    // The rail fades out along with its newest key, unless it's always visible
    let rail_opacity = match visibility {
        Some(RailVisibility::Always) => 1.0,
        _ => visible
            .iter()
            .map(|k| key_opacity(theme, k, life))
            .fold(0.0, f32::max),
    };

    // Right-side and centered positions have the edge on the right, and the keys
    // run leftward; left-side positions mirror that
    let edge_on_right = matches!(
        position,
        OverlayPosition::TopRight
            | OverlayPosition::BottomRight
            | OverlayPosition::TopCenter
            | OverlayPosition::BottomCenter
    );
    let gap = theme.key.gap;
    let (opening, arriving) = slide_phases(motion.shifted_at, motion.rushed);

    // While the held combination grows (Ctrl, then Ctrl + Shift…), the newest key
    // widens smoothly: its keys glide inward and the new one slides in from the edge
    let grown = match motion.slot_grew {
        Some((at, from_parts)) if from_parts < newest.keys.len() => {
            let t = (at.elapsed().as_secs_f32() / SLOT_GROW_SECS).min(1.0);
            let eased = 1.0 - (1.0 - t).powi(3);
            let from = parts_width(&newest.keys[..from_parts], key_size, theme)
                / parts_width(&newest.keys, key_size, theme);
            from + (1.0 - from) * eased
        }
        _ => 1.0,
    };

    // Newest first. A new keystroke arrives in two steps: first the others slide
    // inward to open room at the edge, then it glides into that room from the edge.
    // Keys about to expire close up at a steady speed, so a wide combination takes
    // as long as its width needs.
    let mut children: Vec<Element<'a, M>> = Vec::new();
    // How far the next key starts from the edge (estimated)
    let mut distance = 0.0;
    for (i, k) in visible.iter().rev().enumerate() {
        if distance >= window {
            break;
        }
        let (width, dim, nudge) = if i == 0 {
            (opening, arriving, (1.0 - arriving) * 0.35)
        } else {
            (1.0, 1.0, 0.0)
        };
        let width = width.min(leaving(theme, k, life));
        if width <= 0.0 {
            continue;
        }
        let room = (key_width(k, key_size, theme) + gap) * width;
        // Fade out as the key crosses the far edge of the window
        let inside = ((window - distance) / room).clamp(0.0, 1.0);
        let opacity = key_opacity(theme, k, life) * dim * inside;

        let merging = if i == 0 {
            merge_progress(motion.merged_at)
        } else {
            1.0
        };
        let mut widget = if merging < 1.0 {
            // A repeat: a copy of the key glides in from the edge onto it, and its
            // count goes up once the copy lands. Nothing else moves.
            let before = Keystroke {
                count: k.count.saturating_sub(1).max(1),
                ..(*k).clone()
            };
            let copy = Keystroke {
                count: 1,
                ..(*k).clone()
            };
            let copy_opacity = opacity * ((1.0 - merging) / 0.25).min(1.0);
            cosmic::iced::widget::stack![
                keystroke_widget(&before, key_size, opacity, theme, icon_style),
                Reveal::new(
                    keystroke_widget(&copy, key_size, copy_opacity, theme, icon_style),
                    1.0,
                    edge_on_right,
                )
                .nudge(1.0 - merging),
            ]
            .into()
        } else {
            keystroke_widget(k, key_size, opacity, theme, icon_style)
        };
        if i == 0 && grown < 1.0 {
            widget = Reveal::new(widget, grown, false).into();
        }
        // The gap goes on the far side, so the newest key sits right at the edge
        let padding = if edge_on_right {
            [0.0, 0.0, 0.0, gap]
        } else {
            [0.0, gap, 0.0, 0.0]
        };
        let widget: Element<'a, M> = widget::container(widget).padding(padding).into();
        children.push(if width >= 1.0 && nudge == 0.0 {
            widget
        } else {
            Reveal::new(widget, width, edge_on_right)
                .nudge(nudge)
                .into()
        });
        distance += room;
    }
    if edge_on_right {
        children.reverse();
    }

    let is_bottom = matches!(
        position,
        OverlayPosition::BottomLeft | OverlayPosition::BottomRight | OverlayPosition::BottomCenter
    );
    let align = if is_bottom {
        iced::Alignment::End
    } else {
        iced::Alignment::Start
    };

    // Thin lines between keys, if the theme draws them
    let children = match theme.rail.and_then(|rail| rail.divider) {
        None => children,
        Some(color) => {
            let color = Color {
                a: color.0.a * rail_opacity,
                ..color.0
            };
            let mut joined = Vec::new();
            for (i, child) in children.into_iter().enumerate() {
                if i > 0 {
                    joined.push(
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
                joined.push(child);
            }
            joined
        }
    };

    // Clip whatever runs past the window's far edge
    let row: Element<'a, M> = Reveal::new(
        widget::row::with_children(children).align_y(align),
        1.0,
        edge_on_right,
    )
    .max_width(window)
    .into();
    // A rail at full length keeps the window's size, with the keys at the edge
    let row: Element<'a, M> = match visibility {
        Some(RailVisibility::Always | RailVisibility::WithKeys) => widget::container(row)
            .width(Length::Fixed(window))
            .align_x(if edge_on_right {
                iced::alignment::Horizontal::Right
            } else {
                iced::alignment::Horizontal::Left
            })
            .into(),
        _ => row,
    };

    match theme.rail {
        None => row,
        Some(rail) => on_rail(row, &rail, rail_opacity),
    }
}

/// What's moving in the key row: a new keystroke arriving at the edge, or the
/// held combination growing. The default is nothing moving.
#[derive(Debug, Clone, Copy, Default)]
pub struct Motion {
    /// When the newest keystroke appeared at the edge
    pub shifted_at: Option<Instant>,
    /// When the same key was pressed again and its copy started gliding onto
    /// the newest keystroke
    pub merged_at: Option<Instant>,
    /// It arrived while the one before was still sliding in (fast clicking or
    /// typing), so it slides in quicker and is visible from the start
    pub rushed: bool,
    /// When the held combination grew, and how many keys it had before
    pub slot_grew: Option<(Instant, usize)>,
}

/// How long a repeat's copy takes to glide onto the keystroke it counts up
pub const MERGE_SECS: f32 = 0.2;

/// How far a repeat's copy has glided onto its keystroke, eased (1.0 when done)
fn merge_progress(merged_at: Option<Instant>) -> f32 {
    let t = merged_at.map_or(1.0, |at| (at.elapsed().as_secs_f32() / MERGE_SECS).min(1.0));
    1.0 - (1.0 - t).powi(3)
}

/// How long a held combination takes to widen when a key joins it
pub const SLOT_GROW_SECS: f32 = 0.18;

/// Width of these keys side by side with a "+" between them ("Ctrl + ⇧ + C")
fn parts_width(keys: &[String], key_size: f32, theme: &Theme) -> f32 {
    if keys.is_empty() {
        return key_size;
    }
    let widths: f32 = keys
        .iter()
        .map(|key| part_width(theme, key, key_size))
        .sum();
    widths + (keys.len() - 1) as f32 * PLUS_WIDTH
}

/// A keystroke's width without its gap, estimated from its keys and repeat count
fn key_width(keystroke: &Keystroke, key_size: f32, theme: &Theme) -> f32 {
    let repeat = if theme.key.repeats == Repeats::Inline && keystroke.count > 1 {
        // "×12" at 0.3 of the key size, plus its padding
        let digits = keystroke.count.to_string().len() as f32 + 1.0;
        digits * key_size * 0.3 * 0.6 + key_size * 0.15
    } else {
        0.0
    };
    parts_width(&keystroke.keys, key_size, theme) + repeat
}

/// How long keystrokes stay on screen: fully visible while they linger, then
/// disappearing the way the theme says
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lifetime {
    pub linger: f32,
    pub disappear: f32,
}

impl Lifetime {
    /// Long enough that nothing ever disappears, for samples
    pub const FOREVER: Self = Self {
        linger: 3600.0,
        disappear: 0.0,
    };

    pub fn total(self) -> f32 {
        self.linger + self.disappear
    }
}

/// How long a new keystroke takes to arrive at the edge
pub const SLIDE_SECS: f32 = 0.3;

/// The share of the slide spent opening room in the row, before the key arrives
const OPENING_SHARE: f32 = 0.55;

/// How far along the two steps of a slide are, each eased from 0.0 to 1.0: the
/// row opening room, then the key arriving in it. Both are 1.0 when nothing slides.
fn slide_phases(shifted_at: Option<Instant>, rushed: bool) -> (f32, f32) {
    let ease = |x: f32| 1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3);
    if rushed {
        // Keys coming this fast would never get past the hidden first step, so
        // the room opens and the key shows at the same time, in half the time
        let t = shifted_at.map_or(1.0, |at| at.elapsed().as_secs_f32() / (SLIDE_SECS * 0.5));
        let eased = ease(t);
        return (eased, eased.max(0.6));
    }
    let t = shifted_at.map_or(1.0, |at| (at.elapsed().as_secs_f32() / SLIDE_SECS).min(1.0));
    (
        ease(t / OPENING_SHARE),
        ease((t - OPENING_SHARE) / (1.0 - OPENING_SHARE)),
    )
}

/// A key's opacity as it ages: fading out, or fully visible until it's gone.
/// A fading key finishes fading before it closes up, so the closing only moves
/// empty space and never looks like a wipe.
fn key_opacity(theme: &Theme, keystroke: &Keystroke, life: Lifetime) -> f32 {
    if keystroke.pressed {
        return 1.0;
    }
    match theme.key.expire {
        Expiry::Fade => {
            let fading = life.disappear - expiring_secs(theme, keystroke, life);
            let t = (keystroke.age_secs() - life.linger) / fading.max(f32::EPSILON);
            1.0 - ease_in_cubic(t.clamp(0.0, 1.0))
        }
        Expiry::Wipe | Expiry::Vanish if keystroke.is_expired(life) => 0.0,
        Expiry::Wipe | Expiry::Vanish => 1.0,
    }
}

/// A key's width as it expires. Fading keys close up over their last moments,
/// taking longer the more keys they have so every key closes at the same speed.
/// Wiping keys shrink from the far side over the end of their time. Vanishing
/// keys keep their width until they're gone.
fn leaving(theme: &Theme, keystroke: &Keystroke, life: Lifetime) -> f32 {
    if keystroke.pressed {
        return 1.0;
    }
    let secs = expiring_secs(theme, keystroke, life);
    if secs <= 0.0 {
        return 1.0;
    }
    ((life.total() - keystroke.age_secs()) / secs).clamp(0.0, 1.0)
}

/// How long before it expires a keystroke starts to shrink away (0 for none)
pub fn expiring_secs(theme: &Theme, keystroke: &Keystroke, life: Lifetime) -> f32 {
    match theme.key.expire {
        // Closing up takes longer for wider keys, but always leaves time to fade
        Expiry::Fade => (SLIDE_SECS * keystroke.keys.len().max(1) as f32).min(life.disappear * 0.4),
        Expiry::Wipe => life.disappear,
        Expiry::Vanish => 0.0,
    }
}

/// Put `content` on the theme's rail, faded to `opacity`
fn on_rail<'a, M: 'a>(content: Element<'a, M>, rail: &RailStyle, opacity: f32) -> Element<'a, M> {
    with_border(
        widget::container(content).padding(rail.padding),
        fill_background(rail.background, opacity),
        rail.border,
        rail.radius,
        opacity,
    )
}

/// Give a container a background and the theme's border. A solid border is the
/// container's own; a gradient border, which iced containers can't draw, is
/// stroked on a canvas laid over the container.
fn with_border<'a, M: 'a>(
    container: widget::Container<'a, M, cosmic::Theme>,
    background: Background,
    border: theme::Stroke,
    radius: f32,
    opacity: f32,
) -> Element<'a, M> {
    let fade = |color: Color| Color {
        a: color.a * opacity,
        ..color
    };
    let (solid, gradient) = match border.color {
        Fill::Solid(color) => (fade(color.0), None),
        Fill::Gradient(start, end, angle) => {
            let (start_at, end_at) = theme::stop_positions(&start, &end);
            let stops = [(start_at, fade(start.color.0)), (end_at, fade(end.color.0))];
            (Color::TRANSPARENT, Some((stops, angle)))
        }
    };
    let container = container.class(cosmic::theme::Container::custom(move |_| {
        container::Style {
            background: Some(background),
            border: Border {
                color: solid,
                width: if gradient.is_some() {
                    0.0
                } else {
                    border.width
                },
                radius: radius.into(),
            },
            ..Default::default()
        }
    }));
    match gradient {
        None => container.into(),
        Some((stops, angle)) => cosmic::iced::widget::stack![
            container,
            widget::Canvas::new(GradientBorder {
                stops,
                angle,
                width: border.width,
                radius,
            })
            .width(Length::Fill)
            .height(Length::Fill),
        ]
        .into(),
    }
}

/// A rounded outline drawn with a 45° gradient
struct GradientBorder {
    /// The gradient's two colors and where they sit, from 0 to 1
    stops: [(f32, Color); 2],
    /// Its angle in degrees, the same way backgrounds use it
    angle: f32,
    width: f32,
    radius: f32,
}

impl<M> widget::canvas::Program<M, cosmic::Theme> for GradientBorder {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &cosmic::Renderer,
        _theme: &cosmic::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<widget::canvas::Geometry> {
        use widget::canvas::{gradient::Linear, Frame, Gradient, Path, Style};

        let mut frame = Frame::new(renderer, bounds.size());
        if self.width <= 0.0 {
            return vec![frame.into_geometry()];
        }
        // Keep the whole line inside the bounds
        let inset = self.width / 2.0;
        let size = iced::Size::new(bounds.width - self.width, bounds.height - self.width);
        let radius = (self.radius - inset).clamp(0.0, size.width.min(size.height) / 2.0);
        let outline = Path::rounded_rectangle(iced::Point::new(inset, inset), size, radius.into());
        // The same start and end points iced uses for a background at this angle
        let (from, to) = iced::Radians(self.angle.to_radians())
            .to_distance(&iced::Rectangle::new(iced::Point::ORIGIN, bounds.size()));
        let gradient = Linear::new(from, to)
            .add_stop(self.stops[0].0, self.stops[0].1)
            .add_stop(self.stops[1].0, self.stops[1].1);
        frame.stroke(
            &outline,
            widget::canvas::Stroke {
                style: Style::Gradient(Gradient::Linear(gradient)),
                width: self.width,
                ..Default::default()
            },
        );
        vec![frame.into_geometry()]
    }
}

/// One piece of the typewriter line
#[derive(Debug)]
enum LinePiece<'k> {
    /// Text typed by one keystroke (several characters when it was repeated)
    Text(String, &'k Keystroke),
    /// Anything that isn't typing, drawn as a small key
    Key(&'k Keystroke),
}

/// The text a keystroke typed, if it was plain typing. Shift is allowed, since
/// that's how capitals and symbols are typed; any other modifier makes it a shortcut.
fn typed_text(keystroke: &Keystroke) -> Option<String> {
    let (key, modifiers) = keystroke.keys.split_last()?;
    if modifiers.iter().any(|m| m != "⇧") {
        return None;
    }
    let typed = match key.as_str() {
        " " | "␣" => ' ',
        other => {
            let mut chars = other.chars();
            let c = chars.next()?;
            // Single characters only; names like "Tab" and symbols like "↵" are keys
            if chars.next().is_some() || c.is_control() || matches!(c, '↵' | '⌫' | '⇧') {
                return None;
            }
            c
        }
    };
    Some(typed.to_string().repeat(keystroke.count as usize))
}

/// Turn keystrokes (oldest first) into the typewriter line
fn line_pieces<'k>(keystrokes: impl IntoIterator<Item = &'k Keystroke>) -> Vec<LinePiece<'k>> {
    keystrokes
        .into_iter()
        .map(|keystroke| match typed_text(keystroke) {
            Some(text) => LinePiece::Text(text, keystroke),
            None => LinePiece::Key(keystroke),
        })
        .collect()
}

/// The `Text` layout: a fixed-width line with the newest input at its end.
/// Older input fades by age and also toward the line's far end, then scrolls off.
fn typewriter_line<'a, M: 'a>(
    keystrokes: &[Keystroke],
    key_size: f32,
    life: Lifetime,
    theme: &Theme,
    width: f32,
    icon_style: IconStyle,
) -> Element<'a, M> {
    let font_size = font_size_for_key(key_size);
    let small_key = key_size * 0.7;
    let key_margin = small_key * 0.15;
    // The oldest 30% of the line fades out
    let fade_from = width * 0.7;
    // ponytail: widths are estimated from font size, not measured; the line is
    // clipped, so a bad guess only fades or cuts text a little early or late
    let char_width = font_size * 0.5;
    let space_width = font_size * 0.3;

    let pieces = line_pieces(keystrokes.iter().filter(|k| !k.is_expired(life)));
    let text_color = theme.key.text.0;
    let mut children: Vec<Element<'a, M>> = Vec::new();
    let visibility = theme.rail.map(|rail| rail.visibility);
    let mut rail_opacity: f32 = if visibility == Some(RailVisibility::Always) {
        1.0
    } else {
        0.0
    };
    let mut x = 0.0;

    // A caret after text shows where the next letter goes
    if matches!(pieces.last(), Some(LinePiece::Text(..))) {
        children.push(
            widget::container(widget::Space::new())
                .width(Length::Fixed(2.0))
                .height(Length::Fixed(font_size))
                .class(cosmic::theme::Container::custom(move |_| {
                    container::Style {
                        background: Some(Background::Color(text_color)),
                        ..Default::default()
                    }
                }))
                .into(),
        );
        x += 4.0;
    }

    // Newest first, until the line is full
    for piece in pieces.iter().rev() {
        if x >= width {
            break;
        }
        let (piece_width, keystroke) = match piece {
            LinePiece::Text(text, k) if text.starts_with(' ') => {
                (text.len() as f32 * space_width, k)
            }
            LinePiece::Text(text, k) => (text.chars().count() as f32 * char_width, k),
            LinePiece::Key(k) => {
                let repeat = if k.count > 1 { small_key * 0.45 } else { 0.0 };
                (
                    parts_width(&k.keys, small_key, theme) + repeat + 2.0 * key_margin,
                    k,
                )
            }
        };
        // Wiping pieces shrink from the far side as they expire
        let shown = leaving(theme, keystroke, life);
        let piece_width = piece_width * shown;
        // 1.0 until the piece reaches the fade zone, down to 0.0 at the line's far end
        let edge = ((width - x - piece_width) / (width - fade_from)).clamp(0.0, 1.0);
        let age = key_opacity(theme, keystroke, life);
        rail_opacity = rail_opacity.max(age);
        let opacity = age * edge;
        x += piece_width;

        let element: Element<'a, M> = match piece {
            LinePiece::Text(text, _) if text.starts_with(' ') => widget::Space::new()
                .width(Length::Fixed(piece_width))
                .into(),
            LinePiece::Text(text, _) => text::Text::new(text.clone())
                .size(font_size)
                .font(theme.font())
                .wrapping(cosmic::iced::widget::text::Wrapping::None)
                .class(cosmic::theme::Text::Color(Color {
                    a: text_color.a * opacity,
                    ..text_color
                }))
                .into(),
            LinePiece::Key(k) => {
                widget::container(keystroke_widget(k, small_key, opacity, theme, icon_style))
                    .padding([0.0, key_margin])
                    .into()
            }
        };
        children.push(if shown < 1.0 {
            Reveal::new(element, shown, true).into()
        } else {
            element
        });
    }
    children.reverse();

    let row =
        widget::container(widget::row::with_children(children).align_y(iced::Alignment::Center))
            .height(Length::Fixed(key_size))
            .align_y(iced::alignment::Vertical::Center);
    let line: Element<'a, M> = match visibility {
        // Just around the text, up to the line's width
        Some(RailVisibility::Grow) => Reveal::new(row, 1.0, true).max_width(width).into(),
        // The full line. The text is laid out at its natural width and clipped at
        // the old end, so when typing outruns the line the newest text still shows
        // (a width-limited row would squeeze the newest pieces away instead).
        _ => widget::container(Reveal::new(row, 1.0, true).max_width(width))
            .width(Length::Fixed(width))
            .align_x(iced::alignment::Horizontal::Right)
            .into(),
    };

    match &theme.rail {
        Some(rail) => on_rail(line, rail, rail_opacity),
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text a typewriter line shows, with keys as `[Key+Key]`
    fn line(keystrokes: &[Keystroke]) -> String {
        line_pieces(keystrokes)
            .iter()
            .map(|piece| match piece {
                LinePiece::Text(text, _) => text.clone(),
                LinePiece::Key(k) => format!("[{}]", k.keys.join("+")),
            })
            .collect()
    }

    #[test]
    fn typewriter_joins_text() {
        let shift = KeyModifiers {
            shift: true,
            ..Default::default()
        };
        let ctrl = KeyModifiers {
            ctrl: true,
            ..Default::default()
        };
        let key = |k: &str| Keystroke::single(k, false);
        let repeated = |k: &str, count| Keystroke { count, ..key(k) };

        let typed = [
            Keystroke::combination(&shift, "H", false),
            key("e"),
            repeated("l", 2),
            key("o"),
            key(" "),
            key("w"),
            repeated("⌫", 3),
            Keystroke::combination(&ctrl, "S", false),
            key("↵"),
            Keystroke::combination(&shift, "@", false),
        ];
        // Backspace is shown as a key, not applied to the text
        assert_eq!(line(&typed), "Hello w[⌫][Ctrl+S][↵]@");
        assert_eq!(line(&[key("␣"), key("Tab"), key("⇧")]), " [Tab][⇧]");
    }

    #[test]
    fn keys_expire_the_way_the_theme_says() {
        use crate::config::BuiltinTheme;
        let fade = Lifetime {
            linger: 3.5,
            disappear: 1.5,
        };
        let aged = |secs: f32| Keystroke {
            timestamp: Instant::now() - std::time::Duration::from_secs_f32(secs),
            ..Keystroke::single("a", false)
        };
        let mut theme = Theme::builtin(BuiltinTheme::Frosted);
        let (early, late) = (aged(1.0), aged(4.8));

        // Fade: fades out near the end, then closes up once it's invisible
        assert_eq!(key_opacity(&theme, &early, fade), 1.0);
        assert!(key_opacity(&theme, &aged(4.55), fade) < 0.5);
        assert_eq!(leaving(&theme, &aged(4.55), fade), 1.0);
        assert_eq!(key_opacity(&theme, &late, fade), 0.0);
        assert!(leaving(&theme, &late, fade) < 1.0);

        // Wipe: fully visible, shrinking over the last part of its time
        theme.key.expire = Expiry::Wipe;
        assert_eq!(key_opacity(&theme, &late, fade), 1.0);
        assert_eq!(leaving(&theme, &early, fade), 1.0);
        assert!(leaving(&theme, &aged(4.25), fade) < 0.6);

        // Vanish: fully visible and full size until it's gone
        theme.key.expire = Expiry::Vanish;
        assert_eq!(key_opacity(&theme, &late, fade), 1.0);
        assert_eq!(leaving(&theme, &late, fade), 1.0);
        assert_eq!(key_opacity(&theme, &aged(5.1), fade), 0.0);
    }

    #[test]
    fn room_opens_before_the_key_arrives() {
        assert_eq!(slide_phases(None, false), (1.0, 1.0));
        let (opening, arriving) = slide_phases(Some(Instant::now()), false);
        assert!(opening < 0.1 && arriving == 0.0);
        // Halfway through the opening step, the key hasn't started arriving
        let at =
            Instant::now() - std::time::Duration::from_secs_f32(SLIDE_SECS * OPENING_SHARE / 2.0);
        let (opening, arriving) = slide_phases(Some(at), false);
        assert!(opening > 0.5 && opening < 1.0 && arriving == 0.0);
        let done = Instant::now() - std::time::Duration::from_secs_f32(SLIDE_SECS);
        assert_eq!(slide_phases(Some(done), false), (1.0, 1.0));
        // A rushed key is visible right away
        let (_, arriving) = slide_phases(Some(Instant::now()), true);
        assert!(arriving >= 0.6);
    }

    #[test]
    fn every_icon_key_has_an_icon() {
        for key in ICON_KEYS {
            assert!(
                get_icon_for_key_with_style(key, IconStyle::Symbol).is_some(),
                "{key}"
            );
        }
    }

    #[test]
    fn caps_are_as_wide_as_their_shape() {
        use crate::config::BuiltinTheme;
        let keys = |names: &[&str]| names.iter().map(|k| k.to_string()).collect::<Vec<_>>();
        let plain = Theme::builtin(BuiltinTheme::Dark);
        assert_eq!(
            parts_width(&keys(&["Ctrl", "⇧"]), 50.0, &plain),
            100.0 + PLUS_WIDTH
        );

        let mechanical = Theme::builtin(BuiltinTheme::Mechanical);
        let letter = parts_width(&keys(&["a"]), 50.0, &mechanical);
        let shift = parts_width(&keys(&["⇧"]), 50.0, &mechanical);
        assert!(shift > 2.0 * letter, "{shift} vs {letter}");
        // A mouse button has no cap of its own, so it's on the mouse blank
        let mouse = mechanical.caps["_mouse"].aspect * cap_height(50.0);
        assert_eq!(parts_width(&keys(&["LClick"]), 50.0, &mechanical), mouse);
        assert!(mouse < letter);
        assert_eq!(
            parts_width(&keys(&["⇧", "a"]), 50.0, &mechanical),
            shift + letter + PLUS_WIDTH
        );
    }

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
