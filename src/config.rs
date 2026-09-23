//! Configuration types and palettes for Kiwi keystroke visualizer.

use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic_config::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

/// The APP_ID used for cosmic-config
pub const APP_ID: &str = "io.github.hojjatabdollahi.kiwi";
/// Version pulled from Cargo.toml at compile time
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Themes that ship with Kiwi (see `theme::Theme::builtin`)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum BuiltinTheme {
    #[default]
    Dark,
    Light,
    Frosted,
    Kiwi,
    Ribbon,
    Tape,
    Typewriter,
}

/// What the theme previews in settings are drawn on
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum PreviewBackground {
    /// A gradient that looks like a desktop wallpaper
    #[default]
    Desktop,
    /// A checkerboard, to show exactly how transparent a theme is
    Checkered,
}

impl PreviewBackground {
    /// The other background
    pub fn toggled(self) -> Self {
        match self {
            Self::Desktop => Self::Checkered,
            Self::Checkered => Self::Desktop,
        }
    }
}

/// Position of the overlay on screen
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum OverlayPosition {
    TopLeft,
    TopCenter,
    TopRight,
    #[default]
    BottomLeft,
    BottomRight,
    BottomCenter,
}

/// Icon style preference - symbols vs text labels
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum IconStyle {
    /// Prefer symbol icons (e.g., ⌫ for backspace)
    #[default]
    Symbol,
    /// Prefer text icons (e.g., "Backspace")
    Text,
}

impl IconStyle {
    pub const ALL: &'static [IconStyle] = &[IconStyle::Symbol, IconStyle::Text];

    pub fn name(&self) -> &'static str {
        match self {
            IconStyle::Symbol => "Symbols",
            IconStyle::Text => "Text",
        }
    }
}

/// Key display mode - typed character vs physical key
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyDisplayMode {
    /// Show the typed character (e.g., "Shift+@")
    #[default]
    TypedCharacter,
    /// Show the physical key name (e.g., "Shift+2")
    PhysicalKey,
}

impl KeyDisplayMode {
    pub const ALL: &'static [KeyDisplayMode] =
        &[KeyDisplayMode::TypedCharacter, KeyDisplayMode::PhysicalKey];

    pub fn name(&self) -> &'static str {
        match self {
            KeyDisplayMode::TypedCharacter => "Typed Character",
            KeyDisplayMode::PhysicalKey => "Physical Key",
        }
    }

    pub fn example(&self) -> &'static str {
        match self {
            KeyDisplayMode::TypedCharacter => "Shift+@",
            KeyDisplayMode::PhysicalKey => "Shift+2",
        }
    }
}

impl OverlayPosition {
    /// The corner an anchor point belongs to: keys grow from it toward the
    /// middle of the screen, and the repeat badge sits on the inside
    pub fn from_anchor((x, y): (f32, f32)) -> Self {
        match (x > 0.5, y > 0.5) {
            (false, false) => Self::TopLeft,
            (true, false) => Self::TopRight,
            (false, true) => Self::BottomLeft,
            (true, true) => Self::BottomRight,
        }
    }

    pub const ALL: &'static [OverlayPosition] = &[
        OverlayPosition::TopLeft,
        OverlayPosition::TopCenter,
        OverlayPosition::TopRight,
        OverlayPosition::BottomLeft,
        OverlayPosition::BottomCenter,
        OverlayPosition::BottomRight,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            OverlayPosition::TopLeft => "Top left",
            OverlayPosition::TopCenter => "Top center",
            OverlayPosition::TopRight => "Top right",
            OverlayPosition::BottomLeft => "Bottom left",
            OverlayPosition::BottomCenter => "Bottom center",
            OverlayPosition::BottomRight => "Bottom right",
        }
    }
}

impl BuiltinTheme {
    pub const ALL: &'static [BuiltinTheme] = &[
        BuiltinTheme::Dark,
        BuiltinTheme::Light,
        BuiltinTheme::Frosted,
        BuiltinTheme::Kiwi,
        BuiltinTheme::Ribbon,
        BuiltinTheme::Tape,
        BuiltinTheme::Typewriter,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            BuiltinTheme::Dark => "Dark",
            BuiltinTheme::Light => "Light",
            BuiltinTheme::Frosted => "Frosted",
            BuiltinTheme::Kiwi => "Kiwi",
            BuiltinTheme::Ribbon => "Ribbon",
            BuiltinTheme::Tape => "Tape",
            BuiltinTheme::Typewriter => "Typewriter",
        }
    }
}

/// User configuration - persisted via cosmic-config
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, CosmicConfigEntry)]
#[version = 9]
pub struct Config {
    /// Whether keystroke visualization is enabled
    pub enabled: bool,
    /// Size of keystroke widgets (32-256 pixels)
    pub key_size: f32,
    /// How long keystrokes stay visible (in seconds)
    pub fade_duration: f32,
    /// Built-in theme, used when `user_theme` is not set
    pub palette: BuiltinTheme,
    /// Folder name of the user theme in use (see `theme::themes_dir`)
    pub user_theme: Option<String>,
    /// Position of the overlay on screen
    pub position: OverlayPosition,
    /// Distance between the keys and the screen edge (pixels). Only used to
    /// place configs from before free placement.
    pub margin: f32,
    /// Where the newest key sits, as fractions of the screen's width and height,
    /// set by dragging the keys on screen
    pub anchor: Option<(f32, f32)>,
    /// Width of the typewriter line, overriding the theme's
    pub line_width: Option<f32>,
    /// Key display mode - typed character or physical key
    pub key_display_mode: KeyDisplayMode,
    /// Icon style - symbols or text
    pub icon_style: IconStyle,
    /// Maximum number of keystroke widgets to show (1-10)
    pub history_count: u8,
    /// Show keyboard input
    pub show_keyboard: bool,
    /// Show mouse input (clicks, scroll)
    pub show_mouse: bool,
    /// Show touchpad gestures (taps, swipes, multi-finger)
    pub show_gestures: bool,
    /// Show touchscreen contacts (a marker wherever a finger touches)
    pub show_touch: bool,
    /// Show drawing tablet input (pen taps, eraser, pen and pad buttons)
    pub show_tablet: bool,
    /// What the theme previews in settings are drawn on
    pub preview_background: PreviewBackground,
}

impl Config {
    /// Where the newest key sits, as fractions of the screen's width and height.
    /// Configs from before free placement get the spot matching their old position.
    pub fn anchor(&self) -> (f32, f32) {
        self.anchor.unwrap_or(match self.position {
            OverlayPosition::TopLeft => (0.01, 0.02),
            // Just right of the middle, so the keys still end at the middle
            OverlayPosition::TopCenter => (0.501, 0.02),
            OverlayPosition::TopRight => (0.99, 0.02),
            OverlayPosition::BottomLeft => (0.01, 0.98),
            OverlayPosition::BottomCenter => (0.501, 0.98),
            OverlayPosition::BottomRight => (0.99, 0.98),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            key_size: 64.0,
            fade_duration: 5.0,
            palette: BuiltinTheme::Frosted,
            user_theme: None,
            position: OverlayPosition::TopRight,
            margin: 20.0,
            anchor: None,
            line_width: None,
            key_display_mode: KeyDisplayMode::TypedCharacter,
            icon_style: IconStyle::Symbol,
            history_count: 5,
            show_keyboard: true,
            show_mouse: true,
            show_gestures: true,
            show_touch: true,
            show_tablet: true,
            preview_background: PreviewBackground::Desktop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_positions_become_anchors() {
        let old = Config {
            position: OverlayPosition::BottomLeft,
            anchor: None,
            ..Config::default()
        };
        assert_eq!(old.anchor(), (0.01, 0.98));
        // Old center positions keep ending at the middle, growing leftward
        let center = Config {
            position: OverlayPosition::TopCenter,
            ..old.clone()
        };
        assert_eq!(
            OverlayPosition::from_anchor(center.anchor()),
            OverlayPosition::TopRight
        );
        // A dragged anchor wins over the old position
        let dragged = Config {
            anchor: Some((0.3, 0.7)),
            ..old
        };
        assert_eq!(dragged.anchor(), (0.3, 0.7));
        assert_eq!(
            OverlayPosition::from_anchor(dragged.anchor()),
            OverlayPosition::BottomLeft
        );
    }
}
