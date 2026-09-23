//! The Customize drawer: edit the current theme's fields and save the result as
//! a theme of your own.
//!
//! Edits go to a draft that the overlay shows live. Built-in themes are never
//! changed, and saved themes only change when the draft is saved over them.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, settings};
use cosmic::{Element, Task};

use crate::keystroke::ICON_KEYS;
use crate::theme::{
    self, icon_file_stem, Fill, Hex, Layout, RailStyle, Repeats, Theme, ThemeChoice,
};
use crate::{KiwiApp, Message};

/// A color (or background) the drawer edits as hex text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorField {
    KeyBackground,
    KeyPressed,
    KeyText,
    KeyBorder,
    RailBackground,
    RailBorder,
    RailDivider,
}

impl ColorField {
    const ALL: [Self; 7] = [
        Self::KeyBackground,
        Self::KeyPressed,
        Self::KeyText,
        Self::KeyBorder,
        Self::RailBackground,
        Self::RailBorder,
        Self::RailDivider,
    ];

    /// Only backgrounds can be gradients
    fn allows_gradient(self) -> bool {
        matches!(self, Self::KeyBackground | Self::RailBackground)
    }

    fn get(self, theme: &Theme) -> Option<Fill> {
        let rail = theme.rail.as_ref();
        match self {
            Self::KeyBackground => Some(theme.key.background),
            Self::KeyPressed => Some(Fill::Solid(theme.key.pressed)),
            Self::KeyText => Some(Fill::Solid(theme.key.text)),
            Self::KeyBorder => Some(Fill::Solid(theme.key.border.color)),
            Self::RailBackground => rail.map(|r| r.background),
            Self::RailBorder => rail.map(|r| Fill::Solid(r.border.color)),
            Self::RailDivider => rail.and_then(|r| r.divider).map(Fill::Solid),
        }
    }

    /// Store a value; returns false if this field can't take it
    fn set(self, theme: &mut Theme, value: Option<Fill>) -> bool {
        let solid = match value {
            Some(Fill::Solid(color)) => Some(color),
            Some(Fill::Gradient(..)) if !self.allows_gradient() => return false,
            _ => None,
        };
        match (self, value) {
            (Self::KeyBackground, Some(fill)) => theme.key.background = fill,
            (Self::RailBackground, Some(fill)) => match &mut theme.rail {
                Some(rail) => rail.background = fill,
                None => return false,
            },
            // Leaving the divider empty removes it
            (Self::RailDivider, _) => match &mut theme.rail {
                Some(rail) => rail.divider = solid,
                None => return false,
            },
            (field, _) => {
                let Some(color) = solid else {
                    return false;
                };
                match field {
                    Self::KeyPressed => theme.key.pressed = color,
                    Self::KeyText => theme.key.text = color,
                    Self::KeyBorder => theme.key.border.color = color,
                    Self::RailBorder => match &mut theme.rail {
                        Some(rail) => rail.border.color = color,
                        None => return false,
                    },
                    _ => return false,
                }
            }
        }
        true
    }
}

/// A pixel value the drawer edits with −/+ buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberField {
    KeyBorderWidth,
    KeyRadius,
    KeyGap,
    RailBorderWidth,
    RailRadius,
    RailPadding,
}

impl NumberField {
    fn value(self, theme: &Theme) -> f32 {
        let rail = theme.rail.unwrap_or_default();
        match self {
            Self::KeyBorderWidth => theme.key.border.width,
            Self::KeyRadius => theme.key.radius,
            Self::KeyGap => theme.key.gap,
            Self::RailBorderWidth => rail.border.width,
            Self::RailRadius => rail.radius,
            Self::RailPadding => rail.padding,
        }
    }

    fn nudge(self, theme: &mut Theme, delta: f32) {
        // Radii stop at 999 ("fully round"), everything else at 200
        let max = match self {
            Self::KeyRadius | Self::RailRadius => 999.0,
            _ => 200.0,
        };
        let value = (self.value(theme) + delta).clamp(0.0, max);
        let rail = theme.rail.as_mut();
        match (self, rail) {
            (Self::KeyBorderWidth, _) => theme.key.border.width = value,
            (Self::KeyRadius, _) => theme.key.radius = value,
            (Self::KeyGap, _) => theme.key.gap = value,
            (Self::RailBorderWidth, Some(rail)) => rail.border.width = value,
            (Self::RailRadius, Some(rail)) => rail.radius = value,
            (Self::RailPadding, Some(rail)) => rail.padding = value,
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
pub enum CustomizeMessage {
    Open,
    SetLayout(Layout),
    SetDefaultSize(f32),
    SetRail(bool),
    SetColor(ColorField, String),
    Nudge(NumberField, f32),
    SetRepeats(Repeats),
    SetRecolorIcons(bool),
    ChooseIcons,
    IconsChosen(PathBuf),
    ToggleMissingIcons,
    SetSaveName(String),
    Save,
    Discard,
}

/// The theme being edited
pub struct Draft {
    /// The theme the edits started from
    pub base: ThemeChoice,
    pub theme: Theme,
    /// Whether anything changed since it was opened, saved or discarded
    pub edited: bool,
    /// Where the icons come from (copied into the theme folder on save)
    pub icons_dir: Option<PathBuf>,
    /// Color fields as typed, so half-typed hex isn't lost
    colors: HashMap<ColorField, String>,
    /// The rail's settings while it's switched off, to bring them back
    rail_backup: RailStyle,
    save_name: String,
    show_missing_icons: bool,
}

impl Draft {
    pub fn new(base: ThemeChoice, theme: Theme) -> Self {
        let save_name = match &base {
            ThemeChoice::Builtin(builtin) => format!("My {}", builtin.name()),
            ThemeChoice::User(name) => name.clone(),
        };
        let colors = ColorField::ALL
            .into_iter()
            .map(|field| (field, fill_text(field.get(&theme))))
            .collect();
        Self {
            icons_dir: base.icons_dir(&theme::themes_dir()),
            rail_backup: theme.rail.unwrap_or_default(),
            base,
            theme,
            edited: false,
            colors,
            save_name,
            show_missing_icons: false,
        }
    }

    /// The name shown for the theme, marking unsaved edits
    pub fn name(&self) -> String {
        if self.edited {
            format!("{} (edited)", self.base.name())
        } else {
            self.base.name().to_string()
        }
    }

    fn missing_icons(&self) -> Vec<&'static str> {
        ICON_KEYS
            .iter()
            .map(|key| icon_file_stem(key))
            .filter(|stem| !self.theme.icons.contains_key(*stem))
            .collect()
    }
}

/// "#rrggbbaa", "#rrggbbaa, #rrggbbaa" for a gradient, or nothing
fn fill_text(fill: Option<Fill>) -> String {
    match fill {
        None => String::new(),
        Some(Fill::Solid(color)) => color.to_text(),
        Some(Fill::Gradient(start, end)) => format!("{}, {}", start.to_text(), end.to_text()),
    }
}

/// The reverse of `fill_text`: `Some(None)` for empty text, `None` if it doesn't parse
fn parse_fill(text: &str) -> Option<Option<Fill>> {
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    match parts.as_slice() {
        [""] => Some(None),
        [one] => Some(Some(Fill::Solid(Hex::parse(one)?))),
        [start, end] => Some(Some(Fill::Gradient(Hex::parse(start)?, Hex::parse(end)?))),
        _ => None,
    }
}

impl KiwiApp {
    pub(crate) fn update_customize(
        &mut self,
        message: CustomizeMessage,
    ) -> Task<cosmic::Action<Message>> {
        use CustomizeMessage as M;

        let current = ThemeChoice::from_config(&self.config);
        if matches!(message, M::Open) {
            if self.draft.as_ref().is_none_or(|d| d.base != current) {
                let theme = self.loaded_theme(&current);
                self.draft = Some(Draft::new(current, theme));
            }
            self.context_page = crate::ContextPage::Customize;
            self.core.window.show_context = true;
            return Task::none();
        }

        let Some(draft) = &mut self.draft else {
            return Task::none();
        };
        match message {
            // Handled above
            M::Open => return Task::none(),
            M::SetLayout(layout) => {
                draft.theme.layout = layout;
                // The typewriter line needs a rail to sit on
                if layout == Layout::Text && draft.theme.rail.is_none() {
                    draft.theme.rail = Some(draft.rail_backup);
                }
            }
            M::SetDefaultSize(size) => {
                draft.theme.default_size = size;
                // Show it right away; resizing on screen overrides it again
                self.config.key_size = size;
                self.pending_save = true;
                if let Ok(mut state) = self.shared_state.lock() {
                    state.key_size = size;
                }
            }
            M::SetRail(on) => {
                if let Some(rail) = draft.theme.rail {
                    draft.rail_backup = rail;
                }
                draft.theme.rail = on.then_some(draft.rail_backup);
                if !on && draft.theme.layout == Layout::Text {
                    draft.theme.layout = Layout::Keys;
                }
                for field in [
                    ColorField::RailBackground,
                    ColorField::RailBorder,
                    ColorField::RailDivider,
                ] {
                    draft
                        .colors
                        .insert(field, fill_text(field.get(&draft.theme)));
                }
            }
            M::SetColor(field, text) => {
                let applied =
                    parse_fill(&text).is_some_and(|fill| field.set(&mut draft.theme, fill));
                draft.colors.insert(field, text);
                if !applied {
                    // Keep what was typed, but don't mark anything changed yet
                    return Task::none();
                }
            }
            M::Nudge(field, delta) => field.nudge(&mut draft.theme, delta),
            M::SetRepeats(repeats) => draft.theme.key.repeats = repeats,
            M::SetRecolorIcons(recolor) => draft.theme.recolor_icons = recolor,
            M::ChooseIcons => {
                use cosmic::dialog::file_chooser::open;
                let dialog = open::Dialog::new().title("Choose an icon folder".to_string());
                return cosmic::task::future(async move {
                    match dialog.open_folder().await {
                        Ok(response) => match response.url().to_file_path() {
                            Ok(path) => Message::Customize(M::IconsChosen(path)).into(),
                            Err(()) => {
                                Message::ThemeMessage("Only local folders can be used".into())
                                    .into()
                            }
                        },
                        Err(e) => crate::dialog_failed(e),
                    }
                });
            }
            M::IconsChosen(dir) => {
                draft.theme.load_icons(&dir);
                draft.icons_dir = Some(dir);
            }
            M::ToggleMissingIcons => {
                draft.show_missing_icons = !draft.show_missing_icons;
                return Task::none();
            }
            M::SetSaveName(name) => {
                draft.save_name = name;
                return Task::none();
            }
            M::Save => {
                let text = draft.theme.to_ron();
                let icons = draft.icons_dir.clone();
                let result = theme::save(
                    &text,
                    icons.as_deref(),
                    &theme::themes_dir(),
                    &draft.save_name,
                );
                match result {
                    Ok(name) => {
                        self.theme_message = Some(format!("Saved “{name}”"));
                        self.reload_themes();
                        let choice = ThemeChoice::User(name);
                        self.select_theme(choice.clone());
                        let theme = self.loaded_theme(&choice);
                        self.draft = Some(Draft::new(choice, theme));
                    }
                    Err(e) => self.theme_message = Some(e),
                }
                return Task::none();
            }
            M::Discard => {
                let base = draft.base.clone();
                let theme = self.loaded_theme(&base);
                self.draft = Some(Draft::new(base, theme.clone()));
                self.show_theme(theme);
                return Task::none();
            }
        }

        draft.edited = true;
        let theme = draft.theme.clone();
        self.show_theme(theme);
        Task::none()
    }

    /// The saved version of a theme, from the gallery when it's there
    fn loaded_theme(&self, choice: &ThemeChoice) -> Theme {
        self.themes
            .iter()
            .find(|(c, _)| c == choice)
            .map(|(_, theme)| Theme::clone(theme))
            .unwrap_or_else(|| choice.load(&theme::themes_dir()))
    }

    /// Put a theme on the overlay without changing which theme is selected
    fn show_theme(&self, theme: Theme) {
        if let Ok(mut state) = self.shared_state.lock() {
            state.theme = Arc::new(theme);
        }
    }
}

/// The drawer's content and footer
pub fn view<'a>(
    draft: &'a Draft,
    message: Option<&'a str>,
) -> (Element<'a, Message>, Element<'a, Message>) {
    let theme = &draft.theme;
    let send = |m: CustomizeMessage| Message::Customize(m);

    let color = |label: &'static str, field: ColorField| {
        let text = draft.colors.get(&field).map(String::as_str).unwrap_or("");
        let placeholder = if field.allows_gradient() {
            "#rrggbbaa or two for a gradient"
        } else if field == ColorField::RailDivider {
            "none"
        } else {
            "#rrggbbaa"
        };
        settings::item(
            label,
            widget::text_input(placeholder, text)
                .on_input(move |text| send(CustomizeMessage::SetColor(field, text)))
                .width(Length::Fixed(170.0)),
        )
    };
    let number = |label: &'static str, field: NumberField| {
        let step = match field {
            NumberField::KeyBorderWidth | NumberField::RailBorderWidth => 1.0,
            _ => 2.0,
        };
        settings::item(
            label,
            widget::Row::new()
                .spacing(4)
                .align_y(Alignment::Center)
                .push(
                    widget::button::standard("−")
                        .on_press(send(CustomizeMessage::Nudge(field, -step))),
                )
                .push(
                    widget::text::body(format!("{:.0}", field.value(theme)))
                        .width(Length::Fixed(36.0))
                        .align_x(cosmic::iced::alignment::Horizontal::Center),
                )
                .push(
                    widget::button::standard("+")
                        .on_press(send(CustomizeMessage::Nudge(field, step))),
                ),
        )
    };
    let choice = |label: &'static str, selected: bool, message: CustomizeMessage| {
        widget::radio(label, true, Some(selected), move |_| send(message.clone()))
    };

    let banner = widget::text::caption(if draft.edited {
        format!(
            "{}: built-in and saved themes stay as they are until you save.",
            draft.name()
        )
    } else {
        format!(
            "Editing {}. Changes show on the overlay right away.",
            draft.name()
        )
    });

    let layout = settings::section()
        .title("Layout")
        .add(settings::item(
            "Show",
            widget::Row::new()
                .spacing(12)
                .push(choice(
                    "Each key",
                    theme.layout == Layout::Keys,
                    CustomizeMessage::SetLayout(Layout::Keys),
                ))
                .push(choice(
                    "Typed text",
                    theme.layout == Layout::Text,
                    CustomizeMessage::SetLayout(Layout::Text),
                )),
        ))
        .add(settings::item(
            format!("Default size: {:.0}", theme.default_size),
            widget::slider(32.0..=160.0, theme.default_size, move |v| {
                send(CustomizeMessage::SetDefaultSize(v))
            })
            .width(Length::Fixed(150.0)),
        ));

    let mut rail = settings::section().title("Rail").add(
        settings::item::builder("Draw one strip behind the keys")
            .toggler(theme.rail.is_some(), move |on| {
                send(CustomizeMessage::SetRail(on))
            }),
    );
    if theme.rail.is_some() {
        rail = rail
            .add(color("Background", ColorField::RailBackground))
            .add(color("Border", ColorField::RailBorder))
            .add(number("Border width", NumberField::RailBorderWidth))
            .add(number("Corner radius", NumberField::RailRadius))
            .add(number("Padding", NumberField::RailPadding))
            .add(color("Divider between keys", ColorField::RailDivider));
    }

    let keys = settings::section()
        .title("Keys")
        .add(color("Background", ColorField::KeyBackground))
        .add(color("While held", ColorField::KeyPressed))
        .add(color("Text", ColorField::KeyText))
        .add(color("Border", ColorField::KeyBorder))
        .add(number("Border width", NumberField::KeyBorderWidth))
        .add(number("Corner radius", NumberField::KeyRadius))
        .add(number("Gap between keys", NumberField::KeyGap))
        .add(settings::item(
            "Repeats",
            widget::Row::new()
                .spacing(12)
                .push(choice(
                    "Badge",
                    theme.key.repeats == Repeats::Badge,
                    CustomizeMessage::SetRepeats(Repeats::Badge),
                ))
                .push(choice(
                    "Inline",
                    theme.key.repeats == Repeats::Inline,
                    CustomizeMessage::SetRepeats(Repeats::Inline),
                ))
                .push(choice(
                    "Hide",
                    theme.key.repeats == Repeats::Hidden,
                    CustomizeMessage::SetRepeats(Repeats::Hidden),
                )),
        ));

    let missing = draft.missing_icons();
    let folder = draft
        .icons_dir
        .as_ref()
        .map(|dir| dir.display().to_string())
        .unwrap_or_else(|| "No icon folder".to_string());
    let mut icons = settings::section()
        .title("Icons")
        .add(settings::item(
            folder,
            widget::button::standard("Choose…").on_press(send(CustomizeMessage::ChooseIcons)),
        ))
        .add(
            settings::item::builder(format!(
                "{} of {} icons found",
                ICON_KEYS.len() - missing.len(),
                ICON_KEYS.len()
            ))
            .description("The rest use Kiwi's built-in icons")
            .control(
                widget::button::link(if draft.show_missing_icons {
                    "Hide list"
                } else {
                    "Show missing"
                })
                .on_press(send(CustomizeMessage::ToggleMissingIcons)),
            ),
        );
    if draft.show_missing_icons {
        icons = icons.add(widget::text::caption(
            missing
                .iter()
                .map(|stem| format!("{stem}.svg"))
                .collect::<Vec<_>>()
                .join("  "),
        ));
    }
    icons = icons.add(
        settings::item::builder("Tint icons with the text color")
            .description("Turn off for full-color icons")
            .toggler(theme.recolor_icons, move |on| {
                send(CustomizeMessage::SetRecolorIcons(on))
            }),
    );

    let mut content = widget::Column::new()
        .spacing(16)
        .push(banner)
        .push(layout)
        .push(rail)
        .push(keys)
        .push(icons);
    if let Some(message) = message {
        content = content.push(widget::text::caption(message));
    }

    let footer = widget::Column::new()
        .spacing(8)
        .push(
            widget::Row::new()
                .spacing(8)
                .align_y(Alignment::Center)
                .push(
                    widget::text_input("Theme name", &draft.save_name)
                        .on_input(move |name| send(CustomizeMessage::SetSaveName(name)))
                        .on_submit(move |_| send(CustomizeMessage::Save))
                        .width(Length::Fill),
                )
                .push(
                    widget::button::suggested("Save as theme")
                        .on_press(send(CustomizeMessage::Save)),
                ),
        )
        .push(
            widget::Row::new()
                .spacing(8)
                .push(
                    widget::button::text("Discard changes")
                        .on_press_maybe(draft.edited.then(|| send(CustomizeMessage::Discard))),
                )
                .push(widget::Space::new().width(Length::Fill))
                .push(widget::button::standard("Export…").on_press(Message::ExportTheme)),
        );

    (content.into(), footer.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BuiltinTheme;

    #[test]
    fn color_text_round_trips() {
        let theme = Theme::builtin(BuiltinTheme::Tape);
        for field in ColorField::ALL {
            // Colors go through 8-bit hex, so compare the text
            let text = fill_text(field.get(&theme));
            assert_eq!(parse_fill(&text).map(fill_text), Some(text), "{field:?}");
        }
        assert_eq!(parse_fill("  "), Some(None));
        assert_eq!(parse_fill("#fff"), None);
        assert_eq!(parse_fill("#000000, #ffffff, #000000"), None);
    }

    #[test]
    fn fields_only_take_what_fits() {
        let mut theme = Theme::builtin(BuiltinTheme::Frosted);
        let gradient = parse_fill("#000000, #ffffff").unwrap();
        assert!(!ColorField::KeyText.set(&mut theme, gradient));
        assert!(ColorField::KeyBackground.set(&mut theme, gradient));
        // No rail yet, so rail fields can't be set
        assert!(!ColorField::RailBorder.set(&mut theme, parse_fill("#ffffff").unwrap()));

        let mut tape = Theme::builtin(BuiltinTheme::Tape);
        assert!(ColorField::RailDivider.set(&mut tape, None));
        assert_eq!(tape.rail.unwrap().divider, None);

        NumberField::KeyGap.nudge(&mut theme, -100.0);
        assert_eq!(theme.key.gap, 0.0);
    }
}
