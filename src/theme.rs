//! Overlay themes - how keystrokes are drawn.
//!
//! Built-in themes are defined here in code. User themes use the same struct,
//! read from a `theme.ron` file, where any missing field falls back to the default theme.
//!
//! A user theme is a folder in [`themes_dir`]:
//! ```text
//! my-theme/
//!   theme.ron
//!   icons/Enter.svg, icons/LClick.svg, ...   (optional, see `icon_file_stem`)
//!   icons/caps/_blank.svg, icons/caps/Shift.svg, ...   (optional keycaps, see `Theme::cap`)
//! ```
//!
//! An icon is drawn inside the key's box. A cap *is* the key: it's drawn instead
//! of the box, at the key's height and as wide as the SVG's shape, and never tinted.
//! Keys without their own cap use a blank cap with their usual label on top:
//! `_mouse.svg`, `_touchpad.svg` or `_pen.svg` for input from those devices
//! (tablet pad buttons count as pen), and `_blank.svg` for everything else.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use cosmic::iced::Color;
use cosmic::widget::svg;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::config::BuiltinTheme;

/// A color, written in theme files as "#rrggbb" or "#rrggbbaa"
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hex(pub Color);

impl Hex {
    /// Parse "#rrggbb" or "#rrggbbaa"
    pub fn parse(s: &str) -> Option<Self> {
        parse_hex(s).map(Hex)
    }

    /// Always "#rrggbbaa"
    pub fn to_text(self) -> String {
        let [r, g, b, a] = self.0.into_rgba8();
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

impl Serialize for Hex {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_text())
    }
}

impl<'de> Deserialize<'de> for Hex {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        parse_hex(&s).map(Hex).ok_or_else(|| {
            de::Error::custom(format!(
                "invalid color {s:?}, expected #rrggbb or #rrggbbaa"
            ))
        })
    }
}

fn parse_hex(s: &str) -> Option<Color> {
    let digits = s.strip_prefix('#')?;
    if !digits.is_ascii() || !matches!(digits.len(), 6 | 8) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    let alpha = if digits.len() == 8 { byte(6)? } else { 255 };
    Some(Color::from_rgba8(
        byte(0)?,
        byte(2)?,
        byte(4)?,
        alpha as f32 / 255.0,
    ))
}

/// A background: one color, or a gradient of two at an angle in degrees
/// (0 runs bottom to top, 90 left to right; 45 is the default)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fill {
    Solid(Hex),
    Gradient(Stop, Stop, f32),
}

/// The angle gradients have unless a theme picks another
pub const DEFAULT_GRADIENT_ANGLE: f32 = 45.0;

impl Fill {
    /// A gradient from `start` to `end` at the default angle, spread over its whole length
    pub fn gradient(start: Hex, end: Hex) -> Self {
        Self::Gradient(Stop::new(start), Stop::new(end), DEFAULT_GRADIENT_ANGLE)
    }
}

/// How a fill is written in a theme file: a color, `(start, end)` for a gradient
/// at the default angle, or `(start, end, degrees)`
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum FillRepr {
    Solid(Hex),
    Angled(Stop, Stop, f32),
    Gradient(Stop, Stop),
}

impl Serialize for Fill {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Fill::Solid(color) => FillRepr::Solid(color),
            Fill::Gradient(start, end, angle) if (angle - DEFAULT_GRADIENT_ANGLE).abs() < 0.01 => {
                FillRepr::Gradient(start, end)
            }
            Fill::Gradient(start, end, angle) => FillRepr::Angled(start, end, angle),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Fill {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match FillRepr::deserialize(deserializer)? {
            FillRepr::Solid(color) => Fill::Solid(color),
            FillRepr::Angled(start, end, angle) => Fill::Gradient(start, end, angle),
            FillRepr::Gradient(start, end) => Fill::Gradient(start, end, DEFAULT_GRADIENT_ANGLE),
        })
    }
}

/// One color of a gradient and where it sits along it, from 0 to 1. Before the
/// start color's spot the gradient is all start color, and past the end color's
/// spot all end color. Without a spot, a color sits at its end of the gradient.
///
/// Written as just the color when it's at its end, or as `("#rrggbbaa", 0.3)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop {
    pub color: Hex,
    pub at: Option<f32>,
}

impl Stop {
    pub fn new(color: Hex) -> Self {
        Self { color, at: None }
    }
}

/// Where a gradient's two colors sit, from 0 to 1, in order
pub fn stop_positions(start: &Stop, end: &Stop) -> (f32, f32) {
    let start_at = start.at.unwrap_or(0.0).clamp(0.0, 1.0);
    let end_at = end.at.unwrap_or(1.0).clamp(0.0, 1.0);
    (start_at.min(end_at), end_at.max(start_at))
}

/// How a stop is written in a theme file
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum StopRepr {
    At(Hex, f32),
    Plain(Hex),
}

impl Serialize for Stop {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.at {
            Some(at) => StopRepr::At(self.color, at),
            None => StopRepr::Plain(self.color),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Stop {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match StopRepr::deserialize(deserializer)? {
            StopRepr::At(color, at) => Self {
                color,
                at: Some(at),
            },
            StopRepr::Plain(color) => Self::new(color),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    /// One color, or two for a gradient (written as a plain "#hex" string when solid)
    pub color: Fill,
    pub width: f32,
}

/// How each key (or key combination) is drawn
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyStyle {
    /// Background of a released key
    pub background: Fill,
    /// Background while the key is held
    pub pressed: Hex,
    pub border: Stroke,
    pub radius: f32,
    /// Space between keys
    pub gap: f32,
    pub text: Hex,
    /// Color of the "+" between the parts of a combination
    pub separator: Hex,
    /// Repeat count badge ("x2")
    pub badge_text: Hex,
    pub badge_background: Hex,
    /// Background of a released combination (like "Ctrl + C"); `None` uses `background`
    pub combo_background: Option<Fill>,
    /// How a key pressed several times in a row is shown
    pub repeats: Repeats,
    /// How keys go away when they expire
    pub expire: Expiry,
}

/// How keys go away when they expire
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expiry {
    /// Fade out, then close up
    #[default]
    Fade,
    /// Stay fully visible, then wipe off from the far side
    Wipe,
    /// Stay fully visible, then go away at once
    Vanish,
}

/// When the rail shows, and how long it is
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RailVisibility {
    /// Always on screen at its full length, even with no keys
    Always,
    /// At its full length whenever any key shows
    WithKeys,
    /// Just around the keys, growing and shrinking with them
    #[default]
    Grow,
}

/// How a key pressed several times in a row is shown
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeats {
    /// "x3" in a small badge above or below the key
    #[default]
    Badge,
    /// "×3" inside the key, after its label
    Inline,
    /// Not shown
    Hidden,
}

/// How the input history is laid out
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    /// One key (or combination) after another
    #[default]
    Keys,
    /// A line of text: typed letters join into words, anything else is a small key
    Text,
}

/// A single strip drawn behind all the keys. Themes without one draw keys on their own.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RailStyle {
    pub background: Fill,
    pub border: Stroke,
    /// Corner radius; anything past half the height gives round ends
    pub radius: f32,
    /// Space between the rail's edge and the keys
    pub padding: f32,
    /// A thin line of this color between keys
    pub divider: Option<Hex>,
    /// When the rail shows, and how long it is
    pub visibility: RailVisibility,
}

impl Default for RailStyle {
    fn default() -> Self {
        let frosted = Theme::keys(BuiltinTheme::Frosted);
        Self {
            background: frosted.background,
            border: frosted.border,
            radius: 8.0,
            padding: 0.0,
            divider: None,
            visibility: RailVisibility::Grow,
        }
    }
}

impl Default for KeyStyle {
    fn default() -> Self {
        Theme::default().key
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    /// Key size until it's resized on screen, and what Reset goes back to
    pub default_size: f32,
    pub layout: Layout,
    /// Width of the line in the `Text` layout; older text fades out and scrolls off its far end
    pub line_width: f32,
    pub key: KeyStyle,
    pub rail: Option<RailStyle>,
    /// Font family for key labels and typed text; `None` is Kiwi's own (Gemunu Libre)
    pub font: Option<String>,
    /// Tint single-color icons with the key text color. Turn off for full-color icons.
    pub recolor_icons: bool,
    /// Icons from the theme's `icons/` folder, by file name without `.svg`
    #[serde(skip)]
    pub icons: HashMap<String, svg::Handle>,
    /// Keycaps from the theme's `icons/caps/` folder, by file name without `.svg`
    #[serde(skip)]
    pub caps: HashMap<String, Cap>,
    /// How far down the blank cap its label sits, as a share of the cap's height
    /// (0.5 is the middle; caps with a thick front edge want it higher)
    pub cap_label: f32,
    /// Where the theme's artwork comes from, shown in settings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub credits: Vec<Credit>,
}

/// A keycap SVG and its shape
#[derive(Debug, Clone, PartialEq)]
pub struct Cap {
    pub svg: svg::Handle,
    /// Width over height; a 1u key is about 1, Shift about 2.25
    pub aspect: f32,
}

impl Cap {
    fn new(bytes: impl Into<std::borrow::Cow<'static, [u8]>>) -> Self {
        let bytes = bytes.into();
        Self {
            aspect: svg_aspect(&bytes),
            svg: svg::Handle::from_memory(bytes),
        }
    }
}

/// Artwork a theme uses: what it is, who made it, and under which license
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Credit {
    pub work: String,
    pub author: String,
    pub license: String,
    pub url: String,
}

impl Credit {
    fn new(work: &str, author: &str, license: &str, url: &str) -> Self {
        Self {
            work: work.into(),
            author: author.into(),
            license: license.into(),
            url: url.into(),
        }
    }
}

/// A built-in theme's SVGs, embedded at build time, by icon name
macro_rules! svgs {
    ($dir:literal: $($name:literal)*) => {
        &[$(($name, include_bytes!(concat!("../data/themes/", $dir, "/", $name, ".svg")) as &[u8])),*]
    };
}

type Svgs = &'static [(&'static str, &'static [u8])];

const MECHANICAL_ICONS: Svgs = svgs!("mechanical/icons":
    "Esc" "Home" "End" "PgUp" "PgDn" "Ins" "Del" "ScrLk" "PrtSc" "Pause" "Compose" "Numlock");
const MECHANICAL_CAPS: Svgs = svgs!("mechanical/icons/caps":
    "_blank" "_mouse" "_touchpad" "_pen"
    "Space" "Shift" "Caps" "Tab" "Backspace" "Enter" "Ctrl" "Alt" "Super");
const MAC_CAPS: Svgs = svgs!("mac/icons/caps":
    "_blank" "_mouse" "_touchpad" "_pen"
    "Space" "Shift" "Caps" "Tab" "Backspace" "Enter" "Ctrl" "Alt" "Super" "Esc"
    "Left" "Right" "Up" "Down"
    "A" "B" "C" "D" "E" "F" "G" "H" "I" "J" "K" "L" "M"
    "N" "O" "P" "Q" "R" "S" "T" "U" "V" "W" "X" "Y" "Z"
    "0" "1" "2" "3" "4" "5" "6" "7" "8" "9"
    "Comma" "Period" "Semicolon" "Quote" "Slash" "BracketLeft" "BracketRight"
    "Backslash" "Minus" "Equal" "Grave");

impl Theme {
    pub fn builtin(builtin: BuiltinTheme) -> Self {
        let keys = Self::keys(builtin);
        let clear = Fill::Solid(Hex(Color::TRANSPARENT));
        let no_border = Stroke {
            color: clear,
            width: 0.0,
        };

        let (key, rail) = match builtin {
            // Bare labels on one round-ended strip; combinations sit on a faint pill
            BuiltinTheme::Ribbon => (
                KeyStyle {
                    background: clear,
                    combo_background: Some(Fill::Solid(Hex(Color::from_rgba(1.0, 1.0, 1.0, 0.1)))),
                    border: no_border,
                    radius: 999.0,
                    gap: 6.0,
                    repeats: Repeats::Inline,
                    ..keys
                },
                Some(RailStyle {
                    radius: 999.0,
                    padding: 6.0,
                    ..RailStyle::default()
                }),
            ),
            // One strip split into cells by thin lines
            BuiltinTheme::Tape => (
                KeyStyle {
                    background: clear,
                    border: no_border,
                    radius: 0.0,
                    gap: 0.0,
                    repeats: Repeats::Inline,
                    ..keys
                },
                Some(RailStyle {
                    divider: Some(Hex(Color::from_rgba(1.0, 1.0, 1.0, 0.14))),
                    ..RailStyle::default()
                }),
            ),
            // Typed text as a line, shortcuts as small keys inside it
            BuiltinTheme::Typewriter => (
                KeyStyle {
                    background: Fill::Solid(Hex(Color::from_rgba(1.0, 1.0, 1.0, 0.1))),
                    radius: 5.0,
                    repeats: Repeats::Inline,
                    ..keys
                },
                Some(RailStyle {
                    radius: 10.0,
                    padding: 8.0,
                    // The typewriter line keeps its full length while typing
                    visibility: RailVisibility::WithKeys,
                    ..RailStyle::default()
                }),
            ),
            BuiltinTheme::Dark
            | BuiltinTheme::Light
            | BuiltinTheme::Frosted
            | BuiltinTheme::Kiwi
            | BuiltinTheme::Mechanical
            | BuiltinTheme::Mac => (keys, None),
        };

        let layout = match builtin {
            BuiltinTheme::Typewriter => Layout::Text,
            _ => Layout::Keys,
        };

        let (icons, caps, credits): (Svgs, Svgs, _) = match builtin {
            BuiltinTheme::Mechanical => (
                MECHANICAL_ICONS,
                MECHANICAL_CAPS,
                vec![
                    Credit::new(
                        "Keycaps adapted from Free Keyboard Graphics",
                        "q2apro, after Mysid and Incnis Mrsi",
                        "Public domain",
                        "https://github.com/q2apro/keyboard-keys-speedflips",
                    ),
                    Credit::new(
                        "Key legends adapted from Misonocons",
                        "MisonoWorks",
                        "CC BY 4.0",
                        "https://github.com/misonoworks/misonocons",
                    ),
                ],
            ),
            BuiltinTheme::Mac => (
                &[],
                MAC_CAPS,
                vec![Credit::new(
                    "Keys adapted from SVG Keyboard Icons",
                    "George Black",
                    "MIT",
                    "https://github.com/georgemblack/svg-keyboard-icons",
                )],
            ),
            _ => (&[], &[], Vec::new()),
        };

        Self {
            default_size: 64.0,
            layout,
            line_width: 460.0,
            key,
            rail,
            font: None,
            recolor_icons: true,
            icons: icons
                .iter()
                .map(|(name, bytes)| (name.to_string(), svg::Handle::from_memory(*bytes)))
                .collect(),
            caps: caps
                .iter()
                .map(|(name, bytes)| (name.to_string(), Cap::new(*bytes)))
                .collect(),
            // The mechanical caps' top face sits above their thick front edge
            cap_label: if builtin == BuiltinTheme::Mechanical {
                0.4
            } else {
                0.5
            },
            credits,
        }
    }

    /// Colors and shapes of separate keycaps, which the rail themes then adjust
    fn keys(builtin: BuiltinTheme) -> KeyStyle {
        let rgb = |r, g, b| Hex(Color::from_rgb(r, g, b));
        let rgba = |r, g, b, a| Hex(Color::from_rgba(r, g, b, a));
        let stroke = |color| Stroke {
            color: Fill::Solid(color),
            width: 1.0,
        };

        match builtin {
            // Classic dark with a subtle blue pressed state
            BuiltinTheme::Dark => KeyStyle {
                background: Fill::Solid(rgba(0.0, 0.0, 0.0, 0.4)),
                pressed: rgba(0.2, 0.2, 0.5, 0.5),
                border: stroke(rgba(1.0, 1.0, 1.0, 0.25)),
                radius: 6.0,
                gap: 4.0,
                text: rgb(1.0, 1.0, 1.0),
                separator: rgba(1.0, 1.0, 1.0, 0.5),
                badge_text: rgba(1.0, 1.0, 1.0, 1.0),
                badge_background: rgba(0.0, 0.0, 0.0, 0.6),
                combo_background: None,
                repeats: Repeats::Badge,
                expire: Expiry::Fade,
            },
            // Bright with dark text
            BuiltinTheme::Light => KeyStyle {
                background: Fill::Solid(rgba(0.95, 0.95, 0.97, 0.45)),
                pressed: rgba(0.6, 0.65, 0.85, 0.5),
                border: stroke(rgba(0.3, 0.3, 0.4, 0.3)),
                radius: 6.0,
                gap: 4.0,
                text: rgb(0.1, 0.1, 0.15),
                separator: rgba(0.2, 0.2, 0.3, 0.6),
                badge_text: rgba(0.1, 0.1, 0.15, 1.0),
                badge_background: rgba(1.0, 1.0, 1.0, 0.7),
                combo_background: None,
                repeats: Repeats::Badge,
                expire: Expiry::Fade,
            },
            // Translucent glass with a gradient (the rail themes use these colors too)
            BuiltinTheme::Frosted
            | BuiltinTheme::Ribbon
            | BuiltinTheme::Tape
            | BuiltinTheme::Typewriter => KeyStyle {
                background: Fill::gradient(rgba(0.3, 0.35, 0.45, 0.5), rgba(0.2, 0.25, 0.35, 0.4)),
                pressed: rgba(0.4, 0.5, 0.7, 0.7),
                border: stroke(rgba(1.0, 1.0, 1.0, 0.2)),
                radius: 6.0,
                gap: 4.0,
                text: rgb(1.0, 1.0, 1.0),
                separator: rgba(1.0, 1.0, 1.0, 0.6),
                badge_text: rgba(1.0, 1.0, 1.0, 1.0),
                badge_background: rgba(0.1, 0.15, 0.25, 0.7),
                combo_background: None,
                repeats: Repeats::Badge,
                expire: Expiry::Fade,
            },
            // Kiwi green flesh, brown skin border, cream and seed-colored badge
            BuiltinTheme::Kiwi => KeyStyle {
                background: Fill::gradient(
                    rgba(0.55, 0.75, 0.25, 0.55),
                    rgba(0.7, 0.82, 0.45, 0.45),
                ),
                pressed: rgba(0.35, 0.55, 0.18, 0.75),
                border: stroke(rgba(0.45, 0.32, 0.2, 0.5)),
                radius: 6.0,
                gap: 4.0,
                text: rgb(0.98, 0.97, 0.92),
                separator: rgba(0.85, 0.9, 0.75, 0.8),
                badge_text: rgba(0.15, 0.12, 0.08, 1.0),
                badge_background: rgba(0.95, 0.93, 0.85, 0.85),
                combo_background: None,
                repeats: Repeats::Badge,
                expire: Expiry::Fade,
            },
            // Keycap themes: the box only shows for a key with no cap. The labels
            // are on the caps, and the "+" and badge are on the desktop.
            BuiltinTheme::Mechanical | BuiltinTheme::Mac => {
                let mac = builtin == BuiltinTheme::Mac;
                let label = if mac {
                    rgb(0.11, 0.11, 0.12)
                } else {
                    rgb(0.17, 0.17, 0.17)
                };
                KeyStyle {
                    background: Fill::Solid(if mac {
                        rgb(0.98, 0.98, 0.99)
                    } else {
                        rgb(0.85, 0.85, 0.85)
                    }),
                    pressed: rgb(0.75, 0.75, 0.77),
                    border: stroke(rgba(0.0, 0.0, 0.0, 0.25)),
                    radius: if mac { 10.0 } else { 4.0 },
                    gap: 4.0,
                    text: label,
                    separator: rgba(1.0, 1.0, 1.0, 0.8),
                    badge_text: label,
                    badge_background: rgba(0.97, 0.97, 0.97, 0.9),
                    combo_background: None,
                    repeats: Repeats::Badge,
                    expire: Expiry::Fade,
                }
            }
        }
    }

    /// Read a user theme folder: `theme.ron` plus any `icons/*.svg` and `icons/caps/*.svg`
    pub fn load(dir: &Path) -> Result<Self, String> {
        let file = dir.join(THEME_FILE);
        let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let mut theme: Theme =
            ron::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
        theme.load_icons(&dir.join(ICONS_DIR));
        Ok(theme)
    }

    /// Replace the theme's icons with the SVG files in `dir`, and its caps with the ones in `dir/caps`
    pub fn load_icons(&mut self, dir: &Path) {
        self.icons = read_svgs(dir)
            .map(|(stem, bytes)| (stem, svg::Handle::from_memory(bytes)))
            .collect();
        self.caps = read_svgs(&dir.join(CAPS_DIR))
            .map(|(stem, bytes)| (stem, Cap::new(bytes)))
            .collect();
    }

    /// The theme's icons and caps as files in its icon folder (`Enter.svg`,
    /// `caps/Shift.svg`, ...), sorted
    pub fn files(&self) -> Vec<(String, &[u8])> {
        fn bytes(handle: &svg::Handle) -> Option<&[u8]> {
            match handle.data() {
                cosmic::iced::advanced::svg::Data::Bytes(bytes) => Some(bytes.as_ref()),
                // Theme SVGs are always read into memory
                cosmic::iced::advanced::svg::Data::Path(_) => None,
            }
        }
        let icons = self
            .icons
            .iter()
            .filter_map(|(stem, handle)| Some((format!("{stem}.svg"), bytes(handle)?)));
        let caps = self
            .caps
            .iter()
            .filter_map(|(stem, cap)| Some((format!("{CAPS_DIR}/{stem}.svg"), bytes(&cap.svg)?)));
        let mut files: Vec<_> = icons.chain(caps).collect();
        files.sort();
        files
    }

    /// The theme as `theme.ron` text
    pub fn to_ron(&self) -> String {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .expect("a theme always serializes")
    }

    /// The bold font key labels and typed text are drawn in
    pub fn font(&self) -> cosmic::iced::Font {
        let family = self
            .font
            .as_deref()
            .map_or(crate::keystroke::FONT_NAME, intern);
        cosmic::iced::Font {
            family: cosmic::iced::font::Family::Name(family),
            weight: cosmic::iced::font::Weight::Bold,
            ..Default::default()
        }
    }

    /// The theme's own icon for `key`, if its icon folder has one
    pub fn icon(&self, key: &str) -> Option<&svg::Handle> {
        self.icons.get(icon_file_stem(key))
    }

    /// The theme's own cap for `key`. Letters match either case, since it's the
    /// same key, and a drag falls back to the cap of the button being dragged.
    pub fn cap(&self, key: &str) -> Option<&Cap> {
        let stem = icon_file_stem(key);
        // A full-length space bar takes far too much room on screen, so Space is
        // drawn on the blank cap with the space icon, as wide as a letter
        if stem == "Space" {
            return None;
        }
        self.caps
            .get(stem)
            .or_else(|| self.caps.get(&stem.to_uppercase()))
            .or_else(|| self.cap(crate::keystroke::dragged_button(key)?))
    }

    /// The cap `key` uses when it has none of its own, with its label drawn on
    /// top: the blank for its device if the theme has one, or else `_blank`
    pub fn blank_cap(&self, key: &str) -> Option<&Cap> {
        device_blank(key)
            .and_then(|name| self.caps.get(name))
            .or_else(|| self.caps.get(BLANK_CAP))
    }
}

/// The blank cap for input from a pointing device, by its label
fn device_blank(key: &str) -> Option<&'static str> {
    // Swipes and multi-finger taps: "2Up", "3Tap", "4Down", ...
    let fingers = key
        .strip_prefix(['2', '3', '4'])
        .is_some_and(|rest| matches!(rest, "Tap" | "Up" | "Down" | "Left" | "Right"));
    let pad_button = key
        .strip_prefix("Pad")
        .is_some_and(|n| n.parse::<u8>().is_ok());
    match key {
        "LClick" | "RClick" | "MClick" | "LDrag" | "ScrollUp" | "ScrollDown" => Some("_mouse"),
        "Tap" | "TapDrag" => Some("_touchpad"),
        _ if fingers => Some("_touchpad"),
        "PenTap" | "PenDrag" | "Pen1" | "Pen2" | "Pen3" | "Eraser" | "EraserDrag" => Some("_pen"),
        _ if pad_button => Some("_pen"),
        _ => None,
    }
}

/// Width over height of an SVG, from its `viewBox` or else its `width` and
/// `height`; 1 when neither can be read
fn svg_aspect(bytes: &[u8]) -> f32 {
    let text = String::from_utf8_lossy(bytes);
    let Some(tag) = text.find("<svg").map(|start| &text[start..]) else {
        return 1.0;
    };
    let tag = &tag[..tag.find('>').unwrap_or(tag.len())];
    // An attribute's value, skipping longer names that end the same (like `stroke-width`)
    let attr = |name: &str| {
        tag.match_indices(name).find_map(|(i, _)| {
            if !tag[..i].ends_with(char::is_whitespace) {
                return None;
            }
            let rest = tag[i + name.len()..]
                .trim_start()
                .strip_prefix('=')?
                .trim_start();
            let quote = rest.chars().next().filter(|c| matches!(c, '"' | '\''))?;
            let value = &rest[1..];
            Some(&value[..value.find(quote)?])
        })
    };
    // "120", "120px" and "32mm" all give 120 or 32; percentages give nothing
    let length = |value: &str| {
        let end = value
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(value.len());
        (!value[end..].starts_with('%'))
            .then(|| value[..end].parse::<f32>().ok())
            .flatten()
    };
    let from_view_box = attr("viewBox").and_then(|view_box| {
        let numbers: Vec<f32> = view_box
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|n| !n.is_empty())
            .map(|n| n.parse().ok())
            .collect::<Option<_>>()?;
        match numbers[..] {
            [_, _, w, h] => Some((w, h)),
            _ => None,
        }
    });
    let size = from_view_box.or_else(|| Some((length(attr("width")?)?, length(attr("height")?)?)));
    match size {
        Some((w, h)) if w > 0.0 && h > 0.0 => (w / h).clamp(0.2, 10.0),
        _ => 1.0,
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::builtin(BuiltinTheme::Frosted)
    }
}

const THEME_FILE: &str = "theme.ron";
const ICONS_DIR: &str = "icons";
/// Inside the icons folder
const CAPS_DIR: &str = "caps";
const BLANK_CAP: &str = "_blank";

/// The icon file name (without `.svg`) for a key. Most keys use the name Kiwi
/// already gives them ("LClick", "PgUp", "2Up", "Pad3", "A"); the ones shown as a
/// symbol get a readable name instead.
pub fn icon_file_stem(key: &str) -> &str {
    match key {
        "↵" => "Enter",
        "⇧" => "Shift",
        "⌫" => "Backspace",
        "␣" => "Space",
        "←" => "Left",
        "→" => "Right",
        "↑" => "Up",
        "↓" => "Down",
        "," => "Comma",
        "." => "Period",
        ";" => "Semicolon",
        "'" => "Quote",
        "/" => "Slash",
        "\\" => "Backslash",
        "[" => "BracketLeft",
        "]" => "BracketRight",
        "-" => "Minus",
        "=" => "Equal",
        "`" => "Grave",
        other => other,
    }
}

/// The SVG files in a folder as (file name without `.svg`, contents)
fn read_svgs(dir: &Path) -> impl Iterator<Item = (String, Vec<u8>)> {
    svg_files(dir).into_iter().filter_map(|path| {
        let stem = svg_stem(&path)?.to_string();
        fs::read(&path)
            .map_err(|e| log::warn!("Skipping icon {}: {e}", path.display()))
            .ok()
            .map(|bytes| (stem, bytes))
    })
}

/// File name without `.svg`, for SVG files only
fn svg_stem(path: &Path) -> Option<&str> {
    if path.extension()? != "svg" {
        return None;
    }
    path.file_stem()?.to_str()
}

/// A `'static` copy of a font family name, which is what iced wants. Each name is
/// kept once, so this only grows by the number of different fonts picked.
fn intern(name: &str) -> &'static str {
    static NAMES: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(known) = names.iter().find(|known| **known == name) {
        return known;
    }
    let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
    names.push(leaked);
    leaked
}

/// A regular-weight font by family name, e.g. to show a font's name in that font
pub fn font_named(family: &str) -> cosmic::iced::Font {
    cosmic::iced::Font {
        family: cosmic::iced::font::Family::Name(intern(family)),
        ..Default::default()
    }
}

/// The font families installed on the system, sorted, read once
pub fn font_families() -> &'static [String] {
    static FAMILIES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    FAMILIES.get_or_init(|| {
        let mut font_system = cosmic::iced::advanced::graphics::text::font_system()
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let mut families: Vec<String> = font_system
            .raw()
            .db()
            .faces()
            .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
            .collect();
        families.sort_by_key(|name| name.to_lowercase());
        families.dedup();
        families
    })
}

/// The SVG files in a folder, sorted (none if it can't be read)
fn svg_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| svg_stem(path).is_some())
        .collect();
    files.sort();
    files
}

/// Where user themes live: `$XDG_CONFIG_HOME/kiwi/themes`, usually `~/.config/kiwi/themes`
pub fn themes_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_default()
        .join("kiwi")
        .join("themes")
}

/// Which theme is in use: a built-in palette or a user theme folder
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeChoice {
    Builtin(BuiltinTheme),
    User(String),
}

impl ThemeChoice {
    pub fn from_config(config: &crate::config::Config) -> Self {
        match &config.user_theme {
            Some(name) => Self::User(name.clone()),
            None => Self::Builtin(config.palette),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Builtin(palette) => palette.name(),
            Self::User(name) => name,
        }
    }

    /// Built-in themes first, then the user's theme folders by name
    pub fn all(themes_dir: &Path) -> Vec<Self> {
        let mut user: Vec<String> = fs::read_dir(themes_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.path().join(THEME_FILE).is_file())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        user.sort();

        BuiltinTheme::ALL
            .iter()
            .map(|palette| Self::Builtin(*palette))
            .chain(user.into_iter().map(Self::User))
            .collect()
    }

    /// The folder a user theme's icons live in
    pub fn icons_dir(&self, themes_dir: &Path) -> Option<PathBuf> {
        match self {
            Self::Builtin(_) => None,
            Self::User(name) => Some(themes_dir.join(name).join(ICONS_DIR)),
        }
    }

    /// The theme's `theme.ron`: a user theme's file as written (comments included),
    /// or a built-in theme written out from code
    pub fn file_text(&self, themes_dir: &Path) -> Result<String, String> {
        match self {
            Self::Builtin(builtin) => Ok(Theme::builtin(*builtin).to_ron()),
            Self::User(name) => fs::read_to_string(themes_dir.join(name).join(THEME_FILE))
                .map_err(|e| e.to_string()),
        }
    }

    /// Load the theme. A user theme that can't be read falls back to the default.
    pub fn load(&self, themes_dir: &Path) -> Theme {
        match self {
            Self::Builtin(palette) => Theme::builtin(*palette),
            Self::User(name) => Theme::load(&themes_dir.join(name)).unwrap_or_else(|e| {
                log::warn!("Can't load theme {name:?}, using the default: {e}");
                Theme::default()
            }),
        }
    }
}

/// Largest single file accepted from a theme zip
const MAX_IMPORT_FILE: u64 = 2 * 1024 * 1024;
/// Most files accepted from a theme zip
const MAX_IMPORT_FILES: usize = 1000;

/// Unpack a theme zip into `themes_dir` and return the new theme's name.
///
/// The zip needs a `theme.ron`, either at the top or inside one folder. Only
/// that file and `icons/*.svg` and `icons/caps/*.svg` next to it are copied;
/// everything else is ignored.
/// The theme is named after the zip file; an existing theme with that name is kept
/// and the new one gets a number added.
pub fn import(zip_path: &Path, themes_dir: &Path) -> Result<String, String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Not a zip file: {e}"))?;
    if archive.len() > MAX_IMPORT_FILES {
        return Err(format!("Too many files (more than {MAX_IMPORT_FILES})"));
    }

    // Read the files a theme can contain, with paths made safe by the zip crate
    let mut files: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(path) = entry.enclosed_name().filter(|_| entry.is_file()) else {
            continue;
        };
        if path.file_name() != Some(THEME_FILE.as_ref()) && svg_stem(&path).is_none() {
            continue;
        }
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(MAX_IMPORT_FILE + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if bytes.len() as u64 > MAX_IMPORT_FILE {
            return Err(format!("{} is larger than 2 MB", path.display()));
        }
        files.push((path, bytes));
    }

    // theme.ron marks the root; zipping a folder puts everything one level down
    let root = files
        .iter()
        .map(|(path, _)| path)
        .filter(|path| path.file_name() == Some(THEME_FILE.as_ref()))
        .min_by_key(|path| path.components().count())
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .ok_or("The zip has no theme.ron")?;
    let theme_text = files
        .iter()
        .find(|(path, _)| *path == root.join(THEME_FILE))
        .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
        .unwrap_or_default();
    ron::from_str::<Theme>(&theme_text).map_err(|e| format!("theme.ron: {e}"))?;

    let base = zip_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(folder_name)
        .unwrap_or_default();
    let base = if base.is_empty() {
        "Imported theme".to_string()
    } else {
        base
    };
    let name = (1..)
        .map(|n| {
            if n == 1 {
                base.clone()
            } else {
                format!("{base} {n}")
            }
        })
        .find(|name| !themes_dir.join(name).exists())
        .unwrap_or(base);

    let dest = themes_dir.join(&name);
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(dest.join(ICONS_DIR))?;
        fs::write(dest.join(THEME_FILE), &theme_text)?;
        let icons = root.join(ICONS_DIR);
        let caps = icons.join(CAPS_DIR);
        for (path, bytes) in &files {
            let (Some(parent), Some(file_name)) = (path.parent(), path.file_name()) else {
                continue;
            };
            let folder = if parent == icons {
                dest.join(ICONS_DIR)
            } else if parent == caps {
                dest.join(ICONS_DIR).join(CAPS_DIR)
            } else {
                continue;
            };
            if svg_stem(path).is_some() {
                fs::create_dir_all(&folder)?;
                fs::write(folder.join(file_name), bytes)?;
            }
        }
        Ok(())
    };
    write().map_err(|e| {
        let _ = fs::remove_dir_all(&dest);
        format!("Can't write {}: {e}", dest.display())
    })?;
    Ok(name)
}

/// Keep a name usable as a folder: letters, digits, spaces, `-` and `_`
fn folder_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .collect::<String>()
        .trim()
        .to_string()
}

/// Write a theme to a zip: `theme.ron` at the top and `files` (see [`Theme::files`]) under `icons/`
pub fn export(theme_text: &str, files: &[(String, &[u8])], dest: &Path) -> Result<(), String> {
    let write = || -> zip::result::ZipResult<()> {
        let options = zip::write::SimpleFileOptions::default();
        let mut zip = zip::ZipWriter::new(fs::File::create(dest)?);
        zip.start_file(THEME_FILE, options)?;
        zip.write_all(theme_text.as_bytes())?;

        for (file, bytes) in files {
            zip.start_file(format!("{ICONS_DIR}/{file}"), options)?;
            zip.write_all(bytes)?;
        }
        zip.finish()?;
        Ok(())
    };
    write().map_err(|e| format!("Can't write {}: {e}", dest.display()))
}

/// Save a theme as a folder in `themes_dir` and return the folder name, with
/// `files` (see [`Theme::files`]) written into its icon folder. An existing theme
/// with the same name is replaced.
pub fn save(
    theme_text: &str,
    files: &[(String, &[u8])],
    themes_dir: &Path,
    name: &str,
) -> Result<String, String> {
    let name = folder_name(name);
    if name.is_empty() {
        return Err("Give the theme a name".into());
    }
    let dest = themes_dir.join(&name);
    let icons_dest = dest.join(ICONS_DIR);
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(&icons_dest)?;
        fs::write(dest.join(THEME_FILE), theme_text)?;
        for (file, bytes) in files {
            let path = icons_dest.join(file);
            if let Some(folder) = path.parent() {
                fs::create_dir_all(folder)?;
            }
            fs::write(path, bytes)?;
        }
        Ok(())
    };
    write().map_err(|e| format!("Can't save {}: {e}", dest.display()))?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors_parse() {
        assert_eq!(
            parse_hex("#ff000080"),
            Some(Color::from_rgba8(255, 0, 0, 128.0 / 255.0))
        );
        assert_eq!(parse_hex("#00ff00"), Some(Color::from_rgb8(0, 255, 0)));
        for bad in ["ff0000", "#ff00", "#gg0000", "#ff00000", "#ÿÿÿ"] {
            assert_eq!(parse_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn missing_fields_fall_back_to_default() {
        let theme: Theme =
            ron::from_str(r##"(key: (background: ["#000000", "#ffffff"], gap: 0))"##).unwrap();
        assert_eq!(
            theme.key.background,
            Fill::gradient(Hex(Color::BLACK), Hex(Color::WHITE))
        );
        // A color moved along the gradient keeps its spot
        let moved: Theme =
            ron::from_str(r##"(key: (background: (("#000000", 0.25), "#ffffff")))"##).unwrap();
        let Fill::Gradient(start, end, _) = moved.key.background else {
            panic!("expected a gradient");
        };
        assert_eq!((start.at, end.at), (Some(0.25), None));
        assert_eq!(stop_positions(&start, &end), (0.25, 1.0));
        assert!(ron::to_string(&moved.key.background)
            .unwrap()
            .contains("0.25"));
        assert_eq!(theme.key.gap, 0.0);
        assert_eq!(theme.key.radius, Theme::default().key.radius);

        // Borders written as one color (the only kind before gradients) still load
        let old: Theme =
            ron::from_str(r##"(key: (border: (color: "#ffffff33", width: 2.0)))"##).unwrap();
        assert_eq!(
            old.key.border.color,
            Fill::Solid(Hex::parse("#ffffff33").unwrap())
        );
        let gradient: Theme =
            ron::from_str(r##"(key: (border: (color: ("#ff0000", "#0000ff"), width: 2.0)))"##)
                .unwrap();
        assert!(matches!(gradient.key.border.color, Fill::Gradient(..)));
        // An angle other than the default is kept
        let angled: Theme =
            ron::from_str(r##"(key: (background: ("#000000", "#ffffff", 120.0)))"##).unwrap();
        assert!(matches!(angled.key.background, Fill::Gradient(_, _, a) if a == 120.0));
        assert!(ron::to_string(&angled.key.background)
            .unwrap()
            .contains("120"));

        // Rail visibility and key expiry have defaults, and read back when set
        let set: Theme =
            ron::from_str("(rail: Some((visibility: Always)), key: (expire: Wipe))").unwrap();
        assert_eq!(set.rail.unwrap().visibility, RailVisibility::Always);
        assert_eq!(set.key.expire, Expiry::Wipe);
        assert_eq!(Theme::default().key.expire, Expiry::Fade);
        assert_eq!(
            Theme::builtin(BuiltinTheme::Typewriter)
                .rail
                .unwrap()
                .visibility,
            RailVisibility::WithKeys
        );

        let empty: Theme = ron::from_str("()").unwrap();
        assert_eq!(empty, Theme::default());
        assert_eq!(empty.rail, None);

        let railed: Theme = ron::from_str(
            r##"(rail: Some((divider: Some("#ffffff24"))), key: (repeats: Inline))"##,
        )
        .unwrap();
        let rail = railed.rail.unwrap();
        assert!(rail.divider.is_some());
        assert_eq!(rail.radius, RailStyle::default().radius);
        assert_eq!(railed.key.repeats, Repeats::Inline);
    }

    /// A fresh, empty folder for one test
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kiwi-theme-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_zip(path: &Path, files: &[(&str, &str)]) {
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, body) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    const SVG: &str = "<svg xmlns='http://www.w3.org/2000/svg'/>";

    #[test]
    fn export_then_import_keeps_theme_and_icons() {
        let dir = scratch("roundtrip");
        let themes = dir.join("themes");
        let user = themes.join("Mine");
        fs::create_dir_all(user.join("icons/caps")).unwrap();
        fs::write(user.join("theme.ron"), "// hand-written\n(key: (gap: 9.0))").unwrap();
        fs::write(user.join("icons/Enter.svg"), SVG).unwrap();
        fs::write(user.join("icons/caps/Shift.svg"), WIDE).unwrap();

        let zip = dir.join("Mine.zip");
        let mine = ThemeChoice::User("Mine".into());
        let text = mine.file_text(&themes).unwrap();
        export(&text, &mine.load(&themes).files(), &zip).unwrap();
        // The name is taken, so the import gets a number
        assert_eq!(import(&zip, &themes).unwrap(), "Mine 2");

        let theme = Theme::load(&themes.join("Mine 2")).unwrap();
        assert_eq!(theme.key.gap, 9.0);
        assert!(theme.icon("↵").is_some());
        assert_eq!(theme.cap("⇧").map(|cap| cap.aspect), Some(2.0));
        assert!(fs::read_to_string(themes.join("Mine 2/theme.ron"))
            .unwrap()
            .starts_with("// hand-written"));
        assert_eq!(
            ThemeChoice::all(&themes)[BuiltinTheme::ALL.len()..],
            [
                ThemeChoice::User("Mine".into()),
                ThemeChoice::User("Mine 2".into())
            ]
        );

        let builtin = dir.join("Kiwi.zip");
        let kiwi = ThemeChoice::Builtin(BuiltinTheme::Kiwi);
        export(&kiwi.file_text(&themes).unwrap(), &[], &builtin).unwrap();
        let name = import(&builtin, &themes).unwrap();
        // Colors go through 8-bit hex, so compare what the file holds
        let border = |theme: Theme| ron::to_string(&theme.key.border).unwrap();
        assert_eq!(
            border(Theme::load(&themes.join(name)).unwrap()),
            border(Theme::builtin(BuiltinTheme::Kiwi))
        );
    }

    #[test]
    fn save_copies_icons_and_replaces() {
        let dir = scratch("save");
        let themes = dir.join("themes");
        let icons = dir.join("my icons");
        fs::create_dir_all(&icons).unwrap();
        fs::write(icons.join("Tab.svg"), SVG).unwrap();

        let mut theme = Theme::builtin(BuiltinTheme::Tape);
        theme.load_icons(&icons);
        assert_eq!(
            save(&theme.to_ron(), &theme.files(), &themes, "Talk/mode!").unwrap(),
            "Talkmode"
        );
        let saved = Theme::load(&themes.join("Talkmode")).unwrap();
        assert!(saved.icon("Tab").is_some());
        assert_eq!(saved.to_ron(), theme.to_ron());

        // Saving over itself keeps its icons
        save("(key: (gap: 3.0))", &saved.files(), &themes, "Talkmode").unwrap();
        let saved = Theme::load(&themes.join("Talkmode")).unwrap();
        assert_eq!(saved.key.gap, 3.0);
        assert!(saved.icon("Tab").is_some());

        // A customized built-in keycap theme keeps its built-in caps and icons
        let mechanical = Theme::builtin(BuiltinTheme::Mechanical);
        save(&mechanical.to_ron(), &mechanical.files(), &themes, "Clacky").unwrap();
        let saved = Theme::load(&themes.join("Clacky")).unwrap();
        assert_eq!(saved.caps, mechanical.caps);
        assert_eq!(saved.icons.len(), mechanical.icons.len());
        assert_eq!(saved.cap_label, 0.4);
        assert_eq!(saved.credits, mechanical.credits);

        assert!(save("()", &[], &themes, " ! ").is_err());
    }

    /// Two wide, one high
    const WIDE: &str = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 120 60'/>";

    #[test]
    fn caps_are_measured_and_found() {
        assert_eq!(svg_aspect(WIDE.as_bytes()), 2.0);
        assert_eq!(
            svg_aspect(br#"<svg stroke-width="9" height="53.5" width="107mm">"#),
            2.0
        );
        assert_eq!(svg_aspect(br#"<svg width="100%" height="100%">"#), 1.0);
        assert_eq!(svg_aspect(b"not an svg"), 1.0);

        let mac = Theme::builtin(BuiltinTheme::Mac);
        let shift = mac.cap("⇧").unwrap().aspect;
        assert!(shift > 1.4 && shift < 2.0, "{shift}");
        // Typed lowercase letters use the letter's cap; symbols use their file name
        assert_eq!(mac.cap("a"), mac.cap("A"));
        assert!(mac.cap("a").is_some() && mac.cap("/").is_some());
        assert!(mac.cap("LClick").is_none());

        // A drag without its own cap uses the button's
        let mut theme = Theme::default();
        theme
            .caps
            .insert("LClick".into(), Cap::new(WIDE.as_bytes()));
        assert_eq!(theme.cap("LDrag"), theme.cap("LClick"));
        assert!(theme.cap("LDrag").is_some() && theme.cap("TapDrag").is_none());
        theme.caps.insert("LDrag".into(), Cap::new(SVG.as_bytes()));
        assert_eq!(theme.cap("LDrag").unwrap().aspect, 1.0);

        // Keys without a cap use their device's blank, or the plain one
        for builtin in [BuiltinTheme::Mechanical, BuiltinTheme::Mac] {
            let theme = Theme::builtin(builtin);
            let blank = |key| theme.blank_cap(key).map(|cap| cap.svg.id());
            let named = |name| theme.caps.get(name).map(|cap| cap.svg.id());
            assert!(named("_blank").is_some(), "{builtin:?}");
            for (key, name) in [
                ("A", "_blank"),
                ("F5", "_blank"),
                ("LClick", "_mouse"),
                ("ScrollDown", "_mouse"),
                ("Tap", "_touchpad"),
                ("3Up", "_touchpad"),
                ("2Right", "_touchpad"),
                ("PenDrag", "_pen"),
                ("Pad8", "_pen"),
            ] {
                assert_eq!(blank(key), named(name), "{builtin:?} {key}");
            }
        }
        // A theme without device blanks puts everything on `_blank`
        let mut theme = Theme::default();
        theme.caps.insert("_blank".into(), Cap::new(SVG.as_bytes()));
        assert_eq!(theme.blank_cap("LClick"), theme.blank_cap("A"));
        assert!(theme.blank_cap("A").is_some());
    }

    #[test]
    fn import_takes_only_theme_files() {
        let dir = scratch("unsafe");
        let themes = dir.join("themes");
        let zip = dir.join("../weird name!.zip");
        write_zip(
            &zip,
            &[
                ("folder/theme.ron", "()"),
                ("folder/icons/Tab.svg", SVG),
                ("folder/icons/run.sh", "echo hi"),
                ("../../escape.svg", SVG),
                ("folder/icons/nested/Deep.svg", SVG),
            ],
        );
        let name = import(&zip, &themes).unwrap();
        assert_eq!(name, "weird name");

        let mut files: Vec<_> = fs::read_dir(themes.join(&name).join("icons"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        files.sort();
        assert_eq!(files, ["Tab.svg"]);
        assert!(!dir.join("escape.svg").exists() && !themes.join("escape.svg").exists());
        fs::remove_file(zip).unwrap();

        let broken = dir.join("broken.zip");
        write_zip(&broken, &[("theme.ron", "(key: (gap: \"wide\"))")]);
        assert!(import(&broken, &themes).is_err());
        write_zip(&broken, &[("icons/Tab.svg", SVG)]);
        assert!(import(&broken, &themes).is_err());
        assert!(!themes.join("broken").exists());
    }

    #[test]
    fn font_falls_back_to_kiwis() {
        use cosmic::iced::font::Family;
        let family = |theme: &Theme| match theme.font().family {
            Family::Name(name) => name,
            other => panic!("{other:?}"),
        };
        let mut theme = Theme::default();
        assert_eq!(family(&theme), crate::keystroke::FONT_NAME);
        theme.font = Some("Fira Sans".into());
        let first = family(&theme);
        assert_eq!(first, "Fira Sans");
        // The same name is reused, not stored again
        assert!(std::ptr::eq(first, family(&theme)));
    }

    #[test]
    fn written_themes_read_back() {
        for palette in BuiltinTheme::ALL {
            let theme = Theme::builtin(*palette);
            let text = ron::to_string(&theme).unwrap();
            let read: Theme = ron::from_str(&text).unwrap();
            // Colors go through 8-bit hex, so compare what the file holds
            assert_eq!(ron::to_string(&read).unwrap(), text);
        }
    }
}
