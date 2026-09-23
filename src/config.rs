//! Configuration types and palettes for Kiwi keystroke visualizer.

use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic_config::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

/// The APP_ID used for cosmic-config
pub const APP_ID: &str = "io.github.hojjatabdollahi.kiwi";
/// Version pulled from Cargo.toml at compile time
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Color palette preset for keystroke visualization
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaletteType {
    #[default]
    Dark,
    Light,
    Frosted,
    Kiwi,
}

/// Position of the overlay on screen
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum OverlayPosition {
    TopLeft,
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
    pub const ALL: &'static [OverlayPosition] = &[
        OverlayPosition::TopLeft,
        OverlayPosition::TopRight,
        OverlayPosition::BottomLeft,
        OverlayPosition::BottomRight,
        OverlayPosition::BottomCenter,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            OverlayPosition::TopLeft => "Top Left",
            OverlayPosition::TopRight => "Top Right",
            OverlayPosition::BottomLeft => "Bottom Left",
            OverlayPosition::BottomRight => "Bottom Right",
            OverlayPosition::BottomCenter => "Bottom Center",
        }
    }
}

impl PaletteType {
    pub const ALL: &'static [PaletteType] = &[
        PaletteType::Dark,
        PaletteType::Light,
        PaletteType::Frosted,
        PaletteType::Kiwi,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            PaletteType::Dark => "Dark",
            PaletteType::Light => "Light",
            PaletteType::Frosted => "Frosted",
            PaletteType::Kiwi => "Kiwi",
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
    pub palette: PaletteType,
    /// Folder name of the user theme in use (see `theme::themes_dir`)
    pub user_theme: Option<String>,
    /// Position of the overlay on screen
    pub position: OverlayPosition,
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            key_size: 64.0,
            fade_duration: 5.0,
            palette: PaletteType::Frosted,
            user_theme: None,
            position: OverlayPosition::TopRight,
            key_display_mode: KeyDisplayMode::TypedCharacter,
            icon_style: IconStyle::Symbol,
            history_count: 5,
            show_keyboard: true,
            show_mouse: true,
            show_gestures: true,
            show_touch: true,
        }
    }
}
