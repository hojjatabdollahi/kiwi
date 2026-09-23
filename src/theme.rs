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
//! ```

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

/// A background: one color, or two for a 45° gradient
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Fill {
    Solid(Hex),
    Gradient(Hex, Hex),
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
}

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
                    ..RailStyle::default()
                }),
            ),
            BuiltinTheme::Dark
            | BuiltinTheme::Light
            | BuiltinTheme::Frosted
            | BuiltinTheme::Kiwi => (keys, None),
        };

        let layout = match builtin {
            BuiltinTheme::Typewriter => Layout::Text,
            _ => Layout::Keys,
        };

        Self {
            default_size: 64.0,
            layout,
            line_width: 460.0,
            key,
            rail,
            font: None,
            recolor_icons: true,
            icons: HashMap::new(),
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
            },
            // Translucent glass with a gradient (the rail themes use these colors too)
            BuiltinTheme::Frosted
            | BuiltinTheme::Ribbon
            | BuiltinTheme::Tape
            | BuiltinTheme::Typewriter => KeyStyle {
                background: Fill::Gradient(rgba(0.3, 0.35, 0.45, 0.5), rgba(0.2, 0.25, 0.35, 0.4)),
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
            },
            // Kiwi green flesh, brown skin border, cream and seed-colored badge
            BuiltinTheme::Kiwi => KeyStyle {
                background: Fill::Gradient(
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
            },
        }
    }

    /// Read a user theme folder: `theme.ron` plus any `icons/*.svg`
    pub fn load(dir: &Path) -> Result<Self, String> {
        let file = dir.join(THEME_FILE);
        let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let mut theme: Theme =
            ron::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
        theme.load_icons(&dir.join(ICONS_DIR));
        Ok(theme)
    }

    /// Replace the theme's icons with the SVG files in `dir`
    pub fn load_icons(&mut self, dir: &Path) {
        self.icons.clear();
        for path in svg_files(dir) {
            let Some(stem) = svg_stem(&path) else {
                continue;
            };
            match fs::read(&path) {
                Ok(bytes) => {
                    self.icons
                        .insert(stem.to_string(), svg::Handle::from_memory(bytes));
                }
                Err(e) => log::warn!("Skipping icon {}: {e}", path.display()),
            }
        }
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
}

impl Default for Theme {
    fn default() -> Self {
        Self::builtin(BuiltinTheme::Frosted)
    }
}

const THEME_FILE: &str = "theme.ron";
const ICONS_DIR: &str = "icons";

/// The icon file name (without `.svg`) for a key. Most keys use the name Kiwi
/// already gives them ("LClick", "PgUp", "2Up", "Pad3"); the ones shown as a
/// symbol get a readable name instead.
pub fn icon_file_stem(key: &str) -> &str {
    match key {
        "↵" => "Enter",
        "⇧" => "Shift",
        "⌫" => "Backspace",
        "␣" => "Space",
        other => other,
    }
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
/// that file and `icons/*.svg` next to it are copied; everything else is ignored.
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
        for (path, bytes) in &files {
            let in_icons = path.parent() == Some(&root.join(ICONS_DIR));
            if let (true, Some(file_name)) =
                (in_icons && svg_stem(path).is_some(), path.file_name())
            {
                fs::write(dest.join(ICONS_DIR).join(file_name), bytes)?;
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

/// Write a theme to a zip: `theme.ron` at the top and the icons from `icons_dir` under `icons/`
pub fn export(theme_text: &str, icons_dir: Option<&Path>, dest: &Path) -> Result<(), String> {
    let write = || -> zip::result::ZipResult<()> {
        let options = zip::write::SimpleFileOptions::default();
        let mut zip = zip::ZipWriter::new(fs::File::create(dest)?);
        zip.start_file(THEME_FILE, options)?;
        zip.write_all(theme_text.as_bytes())?;

        for path in icons_dir.map(svg_files).unwrap_or_default() {
            let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            zip.start_file(format!("{ICONS_DIR}/{file_name}"), options)?;
            zip.write_all(&fs::read(&path)?)?;
        }
        zip.finish()?;
        Ok(())
    };
    write().map_err(|e| format!("Can't write {}: {e}", dest.display()))
}

/// Save a theme as a folder in `themes_dir` and return the folder name. Icons are
/// copied from `icons_from` unless they're already in that folder. An existing
/// theme with the same name is replaced.
pub fn save(
    theme_text: &str,
    icons_from: Option<&Path>,
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
        let same_folder = icons_from
            .is_some_and(|from| fs::canonicalize(from).ok() == fs::canonicalize(&icons_dest).ok());
        if !same_folder {
            for path in icons_from.map(svg_files).unwrap_or_default() {
                if let Some(file_name) = path.file_name() {
                    fs::copy(&path, icons_dest.join(file_name))?;
                }
            }
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
            Fill::Gradient(Hex(Color::BLACK), Hex(Color::WHITE))
        );
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
        fs::create_dir_all(user.join("icons")).unwrap();
        fs::write(user.join("theme.ron"), "// hand-written\n(key: (gap: 9.0))").unwrap();
        fs::write(user.join("icons/Enter.svg"), SVG).unwrap();

        let zip = dir.join("Mine.zip");
        let mine = ThemeChoice::User("Mine".into());
        let text = mine.file_text(&themes).unwrap();
        export(&text, mine.icons_dir(&themes).as_deref(), &zip).unwrap();
        // The name is taken, so the import gets a number
        assert_eq!(import(&zip, &themes).unwrap(), "Mine 2");

        let theme = Theme::load(&themes.join("Mine 2")).unwrap();
        assert_eq!(theme.key.gap, 9.0);
        assert!(theme.icon("↵").is_some());
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
        export(&kiwi.file_text(&themes).unwrap(), None, &builtin).unwrap();
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

        let theme = Theme::builtin(BuiltinTheme::Tape);
        assert_eq!(
            save(&theme.to_ron(), Some(&icons), &themes, "Talk/mode!").unwrap(),
            "Talkmode"
        );
        let saved = Theme::load(&themes.join("Talkmode")).unwrap();
        assert!(saved.icon("Tab").is_some());
        assert_eq!(saved.to_ron(), theme.to_ron());

        // Saving over itself keeps its icons
        let own_icons = ThemeChoice::User("Talkmode".into()).icons_dir(&themes);
        save(
            "(key: (gap: 3.0))",
            own_icons.as_deref(),
            &themes,
            "Talkmode",
        )
        .unwrap();
        let saved = Theme::load(&themes.join("Talkmode")).unwrap();
        assert_eq!(saved.key.gap, 3.0);
        assert!(saved.icon("Tab").is_some());

        assert!(save("()", None, &themes, " ! ").is_err());
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
