//! Configuration types and palettes for Kiwi keystroke visualizer.

use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic_config::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

/// Re-export iced Color type for palette colors
pub use cosmic::iced::Color;

/// The APP_ID used for cosmic-config
pub const APP_ID: &str = "dev.hojjat.kiwi";
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

/// Color palette for keystroke visualization
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// Text color (base, before opacity)
    pub text: Color,
    /// Background when key is pressed
    pub bg_pressed: Color,
    /// Background when key is released (can be gradient start)
    pub bg_released: Color,
    /// Optional gradient end color for released state
    pub bg_gradient_end: Option<Color>,
    /// Border color
    pub border: Color,
    /// Plus sign color
    pub plus: Color,
    /// Count badge text color
    pub count: Color,
    /// Count badge background color (for the oval)
    pub count_bg: Color,
}

impl Palette {
    /// Dark theme - classic dark with subtle blue pressed state (more transparent)
    pub fn dark() -> Self {
        Self {
            text: Color::from_rgb(1.0, 1.0, 1.0),
            bg_pressed: Color::from_rgba(0.2, 0.2, 0.5, 0.5),
            bg_released: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
            bg_gradient_end: None,
            border: Color::from_rgba(1.0, 1.0, 1.0, 0.25),
            plus: Color::from_rgba(1.0, 1.0, 1.0, 0.5),
            count: Color::from_rgba(1.0, 1.0, 1.0, 1.0),
            count_bg: Color::from_rgba(0.0, 0.0, 0.0, 0.6),
        }
    }

    /// Light theme - bright with dark text (more transparent)
    pub fn light() -> Self {
        Self {
            text: Color::from_rgb(0.1, 0.1, 0.15),
            bg_pressed: Color::from_rgba(0.6, 0.65, 0.85, 0.5),
            bg_released: Color::from_rgba(0.95, 0.95, 0.97, 0.45),
            bg_gradient_end: None,
            border: Color::from_rgba(0.3, 0.3, 0.4, 0.3),
            plus: Color::from_rgba(0.2, 0.2, 0.3, 0.6),
            count: Color::from_rgba(0.1, 0.1, 0.15, 1.0),
            count_bg: Color::from_rgba(1.0, 1.0, 1.0, 0.7),
        }
    }

    /// Frosted glass - translucent with blur-like gradient
    pub fn frosted() -> Self {
        Self {
            text: Color::from_rgb(1.0, 1.0, 1.0),
            bg_pressed: Color::from_rgba(0.4, 0.5, 0.7, 0.7),
            bg_released: Color::from_rgba(0.3, 0.35, 0.45, 0.5),
            bg_gradient_end: Some(Color::from_rgba(0.2, 0.25, 0.35, 0.4)),
            border: Color::from_rgba(1.0, 1.0, 1.0, 0.2),
            plus: Color::from_rgba(1.0, 1.0, 1.0, 0.6),
            count: Color::from_rgba(1.0, 1.0, 1.0, 1.0),
            count_bg: Color::from_rgba(0.1, 0.15, 0.25, 0.7),
        }
    }

    /// Kiwi theme - vibrant green with brown accents like the fruit
    pub fn kiwi() -> Self {
        Self {
            // Cream/white text for contrast on green
            text: Color::from_rgb(0.98, 0.97, 0.92),
            // Pressed: darker kiwi green
            bg_pressed: Color::from_rgba(0.35, 0.55, 0.18, 0.75),
            // Released: kiwi flesh green with gradient to lighter center
            bg_released: Color::from_rgba(0.55, 0.75, 0.25, 0.55),
            bg_gradient_end: Some(Color::from_rgba(0.7, 0.82, 0.45, 0.45)),
            // Brown border like kiwi skin
            border: Color::from_rgba(0.45, 0.32, 0.2, 0.5),
            // Lighter green for plus
            plus: Color::from_rgba(0.85, 0.9, 0.75, 0.8),
            // Dark seeds color for count
            count: Color::from_rgba(0.15, 0.12, 0.08, 1.0),
            // Cream background for count badge
            count_bg: Color::from_rgba(0.95, 0.93, 0.85, 0.85),
        }
    }

    /// Get palette from type
    pub fn from_type(palette_type: PaletteType) -> Self {
        match palette_type {
            PaletteType::Dark => Self::dark(),
            PaletteType::Light => Self::light(),
            PaletteType::Frosted => Self::frosted(),
            PaletteType::Kiwi => Self::kiwi(),
        }
    }

    /// Apply opacity to all colors
    pub fn with_opacity(&self, opacity: f32) -> Self {
        Self {
            text: color_with_opacity(self.text, opacity),
            bg_pressed: color_with_opacity(self.bg_pressed, opacity),
            bg_released: color_with_opacity(self.bg_released, opacity),
            bg_gradient_end: self.bg_gradient_end.map(|c| color_with_opacity(c, opacity)),
            border: color_with_opacity(self.border, opacity),
            plus: color_with_opacity(self.plus, opacity),
            count: color_with_opacity(self.count, opacity),
            count_bg: color_with_opacity(self.count_bg, opacity),
        }
    }
}

/// Helper to apply opacity multiplier to a color
fn color_with_opacity(color: Color, opacity: f32) -> Color {
    Color::from_rgba(color.r, color.g, color.b, color.a * opacity)
}

/// User configuration - persisted via cosmic-config
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, CosmicConfigEntry)]
#[version = 7]
pub struct Config {
    /// Whether keystroke visualization is enabled
    pub enabled: bool,
    /// Size of keystroke widgets (32-256 pixels)
    pub key_size: f32,
    /// How long keystrokes stay visible (in seconds)
    pub fade_duration: f32,
    /// Color palette
    pub palette: PaletteType,
    /// Position of the overlay on screen
    pub position: OverlayPosition,
    /// Key display mode - typed character or physical key
    pub key_display_mode: KeyDisplayMode,
    /// Maximum number of keystroke widgets to show (1-10)
    pub history_count: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            key_size: 36.0,
            fade_duration: 5.0,
            palette: PaletteType::Dark,
            position: OverlayPosition::BottomLeft,
            key_display_mode: KeyDisplayMode::TypedCharacter,
            history_count: 5,
        }
    }
}
