//! The Customize drawer: edit the current theme's fields and save the result as
//! a theme of your own.
//!
//! Edits go to a draft that the overlay shows live. Built-in themes are never
//! changed, and saved themes only change when the draft is saved over them.

use std::path::PathBuf;
use std::sync::Arc;

use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{self, segmented_button, settings};
use cosmic::{Element, Task};

use crate::color_picker::{color_picker, gradient_editor, Hsva};
use crate::config::{IconStyle, OverlayPosition};
use crate::keystroke::{keystrokes_row, KeyModifiers, Keystroke, ICON_KEYS};
use crate::settings::{segmented_model, select_segment};
use crate::theme::{
    self, icon_file_stem, Expiry, Fill, Hex, Layout, RailStyle, RailVisibility, Repeats, Stop,
    Theme, ThemeChoice,
};
use crate::widgets::{fill_chip, stepper};
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
    /// Backgrounds and borders can be gradients
    fn allows_gradient(self) -> bool {
        matches!(
            self,
            Self::KeyBackground | Self::KeyBorder | Self::RailBackground | Self::RailBorder
        )
    }

    fn get(self, theme: &Theme) -> Option<Fill> {
        let rail = theme.rail.as_ref();
        match self {
            Self::KeyBackground => Some(theme.key.background),
            Self::KeyPressed => Some(Fill::Solid(theme.key.pressed)),
            Self::KeyText => Some(Fill::Solid(theme.key.text)),
            Self::KeyBorder => Some(theme.key.border.color),
            Self::RailBackground => rail.map(|r| r.background),
            Self::RailBorder => rail.map(|r| r.border.color),
            Self::RailDivider => rail.and_then(|r| r.divider).map(Fill::Solid),
        }
    }

    /// One color of the field: the only one, or the start (0) or end (1) of a gradient
    fn stop(self, theme: &Theme, stop: usize) -> Option<Hex> {
        match (self.get(theme)?, stop) {
            (Fill::Solid(color), 0) => Some(color),
            (Fill::Gradient(start, _), 0) => Some(start.color),
            (Fill::Gradient(_, end), 1) => Some(end.color),
            _ => None,
        }
    }

    fn set_stop(self, theme: &mut Theme, stop: usize, color: Hex) -> bool {
        let fill = match (self.get(theme), stop) {
            // A new color keeps its spot along the gradient
            (Some(Fill::Gradient(start, end)), 0) => Fill::Gradient(Stop { color, ..start }, end),
            (Some(Fill::Gradient(start, end)), 1) => Fill::Gradient(start, Stop { color, ..end }),
            (_, 0) => Fill::Solid(color),
            _ => return false,
        };
        self.set(theme, Some(fill))
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
            (Self::KeyBorder, Some(fill)) => theme.key.border.color = fill,
            (Self::RailBorder, Some(fill)) => match &mut theme.rail {
                Some(rail) => rail.border.color = fill,
                None => return false,
            },
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
    LayoutTab(segmented_button::Entity),
    SetDefaultSize(f32),
    SetRail(bool),
    /// Open the picker for a field (at the given gradient end), or close it if it's open
    PickColor(ColorField, usize),
    /// Edit the start (0) or end (1) of the gradient being picked
    PickStop(usize),
    /// Move the start (0) or end (1) color along the gradient being picked, 0 to 1
    MoveStop(usize, f32),
    PickerChanged(Hsva),
    PickerHex(String),
    PickerDone,
    PickerCancel,
    /// Switch a background or border between one color and a gradient
    ToggleGradient(ColorField),
    /// Remove an optional color (the divider)
    ClearColor(ColorField),
    Nudge(NumberField, f32),
    RepeatsTab(segmented_button::Entity),
    RailVisibilityTab(segmented_button::Entity),
    ExpiryTab(segmented_button::Entity),
    /// Open the font list, or close it
    ToggleFontPicker,
    /// Text typed in the font list's search field
    FontQuery(String),
    /// Use a font family (`None` is Kiwi's own)
    PickFont(Option<String>),
    SetRecolorIcons(bool),
    ChooseIcons,
    IconsChosen(PathBuf),
    ToggleMissingIcons,
    /// Show the name field for saving as a new theme, or hide it again
    StartNaming,
    StopNaming,
    SetSaveName(String),
    /// Save under the name in the name field
    Save,
    /// Save over the user theme the edits started from
    SaveInPlace,
    Discard,
}

const LAYOUTS: &[(&str, Layout)] = &[("Each key", Layout::Keys), ("Typed text", Layout::Text)];
const RAIL_VISIBILITY: &[(&str, RailVisibility)] = &[
    ("Always", RailVisibility::Always),
    ("With keys", RailVisibility::WithKeys),
    ("Grow", RailVisibility::Grow),
];
const EXPIRY: &[(&str, Expiry)] = &[
    ("Fade", Expiry::Fade),
    ("Wipe", Expiry::Wipe),
    ("Vanish", Expiry::Vanish),
];
const REPEATS: &[(&str, Repeats)] = &[
    ("Badge", Repeats::Badge),
    ("Inline", Repeats::Inline),
    ("Hide", Repeats::Hidden),
];

fn font_search_id() -> widget::Id {
    widget::Id::new("kiwi-font-search")
}

/// How Kiwi's own font is listed
const KIWI_FONT: &str = "Kiwi (Gemunu Libre)";
/// The font list shows at most this many matches, so a short query stays quick
const MAX_FONT_MATCHES: usize = 80;

/// The theme being edited
pub struct Draft {
    /// The theme the edits started from
    pub base: ThemeChoice,
    pub theme: Theme,
    /// Whether anything changed since it was opened, saved or discarded
    pub edited: bool,
    /// Where the icons come from (copied into the theme folder on save)
    pub icons_dir: Option<PathBuf>,
    /// The color being picked, if the picker is open
    picking: Option<Picking>,
    /// The rail's settings while it's switched off, to bring them back
    rail_backup: RailStyle,
    save_name: String,
    /// Whether the footer is asking for a name to save under
    naming: bool,
    show_missing_icons: bool,
    /// The font list's search text, while the list is open
    font_query: Option<String>,
    layout_model: segmented_button::SingleSelectModel,
    repeats_model: segmented_button::SingleSelectModel,
    rail_visibility_model: segmented_button::SingleSelectModel,
    expiry_model: segmented_button::SingleSelectModel,
}

impl Draft {
    pub fn new(base: ThemeChoice, theme: Theme) -> Self {
        let save_name = match &base {
            ThemeChoice::Builtin(builtin) => format!("My {}", builtin.name()),
            ThemeChoice::User(name) => name.clone(),
        };
        Self {
            icons_dir: base.icons_dir(&theme::themes_dir()),
            rail_backup: theme.rail.unwrap_or_default(),
            layout_model: segmented_model(LAYOUTS, theme.layout),
            repeats_model: segmented_model(REPEATS, theme.key.repeats),
            rail_visibility_model: segmented_model(
                RAIL_VISIBILITY,
                theme.rail.map(|rail| rail.visibility).unwrap_or_default(),
            ),
            expiry_model: segmented_model(EXPIRY, theme.key.expire),
            naming: false,
            base,
            theme,
            edited: false,
            picking: None,
            save_name,
            show_missing_icons: false,
            font_query: None,
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

/// The color the picker is open for
struct Picking {
    field: ColorField,
    stop: usize,
    hsva: Hsva,
    /// The hex field's text, kept while it's being typed
    hex: String,
    /// The field before picking started, for Cancel
    before: Option<Fill>,
}

/// The last few folders of a path, short enough to sit next to a button
fn short_path(path: &std::path::Path) -> String {
    let parts: Vec<_> = path.iter().map(|p| p.to_string_lossy()).collect();
    match parts.len() {
        0..=3 => path.display().to_string(),
        n => format!("…/{}", parts[n - 3..].join("/")),
    }
}

/// A new divider starts as a faint white line
const NEW_DIVIDER: Hex = Hex(cosmic::iced::Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.14,
});

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
            M::LayoutTab(entity) => {
                draft.layout_model.activate(entity);
                let Some(&layout) = draft.layout_model.data::<Layout>(entity) else {
                    return Task::none();
                };
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
                    select_segment(&mut draft.layout_model, Layout::Keys);
                }
                draft.picking = None;
            }
            M::PickColor(field, stop) => {
                // Clicking the field's chip again closes the picker, whichever end is open
                let already_open = draft.picking.as_ref().is_some_and(|p| p.field == field);
                draft.picking = if already_open {
                    None
                } else {
                    let color = field.stop(&draft.theme, stop).unwrap_or(NEW_DIVIDER);
                    Some(Picking {
                        field,
                        stop,
                        hsva: Hsva::from_color(color.0),
                        hex: color.to_text(),
                        before: field.get(&draft.theme),
                    })
                };
                return Task::none();
            }
            M::PickStop(stop) => {
                let Some(picking) = &mut draft.picking else {
                    return Task::none();
                };
                if let Some(color) = picking.field.stop(&draft.theme, stop) {
                    picking.stop = stop;
                    picking.hsva = Hsva::from_color(color.0);
                    picking.hex = color.to_text();
                }
                return Task::none();
            }
            M::MoveStop(stop, at) => {
                let Some(picking) = &draft.picking else {
                    return Task::none();
                };
                let field = picking.field;
                let Some(Fill::Gradient(mut start, mut end)) = field.get(&draft.theme) else {
                    return Task::none();
                };
                // The colors can meet (a hard edge) but not pass each other
                let (start_at, end_at) = theme::stop_positions(&start, &end);
                if stop == 0 {
                    start.at = Some(at.min(end_at));
                } else {
                    end.at = Some(at.max(start_at));
                }
                field.set(&mut draft.theme, Some(Fill::Gradient(start, end)));
            }
            M::PickerChanged(hsva) => {
                let Some(picking) = &mut draft.picking else {
                    return Task::none();
                };
                let color = Hex(hsva.to_color());
                picking.hsva = hsva;
                picking.hex = color.to_text();
                picking
                    .field
                    .set_stop(&mut draft.theme, picking.stop, color);
            }
            M::PickerHex(text) => {
                let Some(picking) = &mut draft.picking else {
                    return Task::none();
                };
                let parsed = Hex::parse(text.trim());
                picking.hex = text;
                // Keep what was typed, but only apply it once it's a whole color
                let Some(color) = parsed else {
                    return Task::none();
                };
                picking.hsva = Hsva::from_color(color.0);
                picking
                    .field
                    .set_stop(&mut draft.theme, picking.stop, color);
            }
            M::PickerDone => {
                draft.picking = None;
                return Task::none();
            }
            M::PickerCancel => {
                let Some(picking) = draft.picking.take() else {
                    return Task::none();
                };
                picking.field.set(&mut draft.theme, picking.before);
            }
            M::ToggleGradient(field) => {
                let fill = match field.get(&draft.theme) {
                    Some(Fill::Solid(color)) => Fill::gradient(color, color),
                    Some(Fill::Gradient(start, _)) => Fill::Solid(start.color),
                    None => return Task::none(),
                };
                field.set(&mut draft.theme, Some(fill));
                // Keep the picker open; after switching to one color it edits that color
                if let Some(picking) = draft.picking.as_mut().filter(|p| p.field == field) {
                    if matches!(fill, Fill::Solid(_)) && picking.stop == 1 {
                        let color = field.stop(&draft.theme, 0).unwrap_or(NEW_DIVIDER);
                        picking.stop = 0;
                        picking.hsva = Hsva::from_color(color.0);
                        picking.hex = color.to_text();
                    }
                }
            }
            M::ClearColor(field) => {
                field.set(&mut draft.theme, None);
                draft.picking = None;
            }
            M::Nudge(field, delta) => field.nudge(&mut draft.theme, delta),
            M::RailVisibilityTab(entity) => {
                draft.rail_visibility_model.activate(entity);
                let Some(&visibility) = draft.rail_visibility_model.data::<RailVisibility>(entity)
                else {
                    return Task::none();
                };
                if let Some(rail) = &mut draft.theme.rail {
                    rail.visibility = visibility;
                }
            }
            M::ExpiryTab(entity) => {
                draft.expiry_model.activate(entity);
                let Some(&expire) = draft.expiry_model.data::<Expiry>(entity) else {
                    return Task::none();
                };
                draft.theme.key.expire = expire;
            }
            M::RepeatsTab(entity) => {
                draft.repeats_model.activate(entity);
                let Some(&repeats) = draft.repeats_model.data::<Repeats>(entity) else {
                    return Task::none();
                };
                draft.theme.key.repeats = repeats;
            }
            M::ToggleFontPicker => {
                draft.font_query = match draft.font_query {
                    Some(_) => None,
                    None => Some(String::new()),
                };
                // Start typing right away
                return widget::text_input::focus(font_search_id());
            }
            M::FontQuery(query) => {
                draft.font_query = Some(query);
                return Task::none();
            }
            M::PickFont(font) => {
                draft.theme.font = font;
                draft.font_query = None;
            }
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
            M::StartNaming | M::StopNaming => {
                draft.naming = matches!(message, M::StartNaming);
                return Task::none();
            }
            M::SetSaveName(name) => {
                draft.save_name = name;
                return Task::none();
            }
            M::Save | M::SaveInPlace => {
                if let (M::SaveInPlace, ThemeChoice::User(name)) = (&message, &draft.base) {
                    draft.save_name = name.clone();
                }
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
    icon_style: IconStyle,
    background: crate::config::PreviewBackground,
) -> (Element<'a, Message>, Element<'a, Message>) {
    let theme = &draft.theme;
    let send = |m: CustomizeMessage| Message::Customize(m);

    // The field's chip (a color, or a gradient), then its extra action
    let chips = move |field: ColorField| {
        let fill = field.get(theme);
        let mut row = widget::Row::new().spacing(6).align_y(Alignment::Center);
        if let Some(fill) = fill {
            let (start, end) = match fill {
                Fill::Solid(color) => (color.0, None),
                Fill::Gradient(start, end) => (
                    start.color.0,
                    Some((end.color.0, theme::stop_positions(&start, &end))),
                ),
            };
            let open = draft.picking.as_ref().is_some_and(|p| p.field == field);
            row = row.push(fill_chip(
                start,
                end,
                open,
                send(CustomizeMessage::PickColor(field, 0)),
            ));
        }
        if field == ColorField::RailDivider {
            row = row.push(if fill.is_none() {
                widget::button::link("Add").on_press(send(CustomizeMessage::PickColor(field, 0)))
            } else {
                widget::button::link("Remove").on_press(send(CustomizeMessage::ClearColor(field)))
            });
        }
        row
    };

    // The picker, when it's open for this field
    let picker = move |field: ColorField| -> Option<Element<'a, Message>> {
        let picking = draft.picking.as_ref().filter(|p| p.field == field)?;
        let gradient = matches!(field.get(theme), Some(Fill::Gradient(..)));
        let mut panel = widget::Column::new().spacing(8);
        if field.allows_gradient() {
            let hint = if gradient {
                "Click a marker to edit its color, drag it to move it"
            } else {
                ""
            };
            panel = panel.push(
                widget::Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(widget::text::caption(hint).width(Length::Fill))
                    .push(widget::text::body("Gradient"))
                    .push(
                        widget::toggler(gradient)
                            .on_toggle(move |_| send(CustomizeMessage::ToggleGradient(field))),
                    ),
            );
        }
        if let Some(Fill::Gradient(start, end)) = field.get(theme) {
            let (start_at, end_at) = theme::stop_positions(&start, &end);
            panel = panel.push(gradient_editor(
                [(start_at, start.color.0), (end_at, end.color.0)],
                picking.stop,
                move |stop| send(CustomizeMessage::PickStop(stop)),
                move |stop, at| send(CustomizeMessage::MoveStop(stop, at)),
            ));
        }
        Some(
            panel
                .push(color_picker(
                    picking.hsva,
                    &picking.hex,
                    move |hsva| send(CustomizeMessage::PickerChanged(hsva)),
                    move |hex| send(CustomizeMessage::PickerHex(hex)),
                ))
                .push(
                    widget::Row::new()
                        .spacing(8)
                        .push(widget::Space::new().width(Length::Fill))
                        .push(
                            widget::button::standard("Cancel")
                                .on_press(send(CustomizeMessage::PickerCancel)),
                        )
                        .push(
                            widget::button::suggested("Done")
                                .on_press(send(CustomizeMessage::PickerDone)),
                        ),
                )
                .into(),
        )
    };

    let number = move |field: NumberField| {
        let step = match field {
            NumberField::KeyBorderWidth | NumberField::RailBorderWidth => 1.0,
            _ => 2.0,
        };
        let value = field.value(theme);
        stepper(
            format!("{value:.0}"),
            36.0,
            (value > 0.0).then(|| send(CustomizeMessage::Nudge(field, -step))),
            Some(send(CustomizeMessage::Nudge(field, step))),
        )
    };

    // A color row, with the picker under it while it's open
    let add_color =
        move |section: settings::Section<'a, Message>, label: &'static str, field: ColorField| {
            let section = section.add(settings::item(label, chips(field)));
            match picker(field) {
                Some(picker) => section.add(picker),
                None => section,
            }
        };
    // Border color, then border width on its own row (two gradient chips leave no room)
    let add_border =
        move |section: settings::Section<'a, Message>, field: ColorField, width: NumberField| {
            let section = section.add(settings::item("Border", chips(field)));
            let section = match picker(field) {
                Some(picker) => section.add(picker),
                None => section,
            };
            section.add(settings::item("Border width", number(width)))
        };

    let layout = settings::section()
        .title("Layout")
        .add(settings::item(
            "Show",
            widget::segmented_control::horizontal(&draft.layout_model)
                .on_activate(move |e| send(CustomizeMessage::LayoutTab(e)))
                .width(Length::Shrink),
        ))
        .add(
            settings::item::builder("Default size")
                .description("Used until you resize it on screen")
                .control(
                    widget::Row::new()
                        .spacing(12)
                        .align_y(Alignment::Center)
                        .push(
                            widget::slider(32.0..=160.0, theme.default_size, move |v| {
                                send(CustomizeMessage::SetDefaultSize(v))
                            })
                            .width(Length::Fixed(120.0)),
                        )
                        .push(
                            widget::text::body(format!("{:.0} px", theme.default_size))
                                .width(Length::Fixed(48.0))
                                .align_x(cosmic::iced::alignment::Horizontal::Right),
                        ),
                ),
        );

    // The rail's on/off switch sits in its section header
    let mut rail = settings::section().header(
        widget::Row::new()
            .align_y(Alignment::Center)
            .push(widget::text::heading("Rail"))
            .push(widget::Space::new().width(Length::Fill))
            .push(
                widget::toggler(theme.rail.is_some())
                    .on_toggle(move |on| send(CustomizeMessage::SetRail(on))),
            ),
    );
    if theme.rail.is_some() {
        rail = add_color(rail, "Background", ColorField::RailBackground);
        rail = add_border(rail, ColorField::RailBorder, NumberField::RailBorderWidth);
        rail = rail
            .add(settings::item(
                "Corner radius",
                number(NumberField::RailRadius),
            ))
            .add(settings::item("Padding", number(NumberField::RailPadding)))
            .add(
                settings::item::builder("Show the rail")
                    .description("Always, at full length while keys show, or just around the keys")
                    .control(
                        widget::segmented_control::horizontal(&draft.rail_visibility_model)
                            .on_activate(move |e| send(CustomizeMessage::RailVisibilityTab(e)))
                            .width(Length::Shrink),
                    ),
            );
        rail = add_color(rail, "Divider between keys", ColorField::RailDivider);
    } else {
        rail = rail.add(widget::text::caption(
            "Off: keys are drawn on their own. Typed text needs the rail.",
        ));
    }

    let mut keys = settings::section().title("Keys");
    keys = add_color(keys, "Background", ColorField::KeyBackground);
    keys = add_color(keys, "While held", ColorField::KeyPressed);
    keys = add_color(keys, "Text", ColorField::KeyText);
    keys = keys.add(settings::item(
        "Font",
        widget::button::standard(format!("{} ▾", theme.font.as_deref().unwrap_or(KIWI_FONT)))
            .on_press(send(CustomizeMessage::ToggleFontPicker)),
    ));
    if let Some(query) = &draft.font_query {
        keys = keys.add(font_picker(query, theme.font.as_deref()));
    }
    keys = add_border(keys, ColorField::KeyBorder, NumberField::KeyBorderWidth);
    let keys = keys
        .add(settings::item(
            "Corner radius",
            number(NumberField::KeyRadius),
        ))
        .add(settings::item(
            "Gap between keys",
            number(NumberField::KeyGap),
        ))
        .add(settings::item(
            "Repeats",
            widget::segmented_control::horizontal(&draft.repeats_model)
                .on_activate(move |e| send(CustomizeMessage::RepeatsTab(e)))
                .width(Length::Shrink),
        ))
        .add(settings::item(
            "Expired keys",
            widget::segmented_control::horizontal(&draft.expiry_model)
                .on_activate(move |e| send(CustomizeMessage::ExpiryTab(e)))
                .width(Length::Shrink),
        ));

    let missing = draft.missing_icons();
    let folder = draft
        .icons_dir
        .as_deref()
        .map(short_path)
        .unwrap_or_else(|| "No folder, using Kiwi's icons".to_string());
    let mut icons = settings::section()
        .title("Icons")
        .add(settings::item_row(vec![
            widget::container(widget::text::caption(folder).font(cosmic::font::mono()))
                .padding([6, 12])
                .width(Length::Fill)
                .class(cosmic::theme::Container::custom(|theme| {
                    let cosmic = theme.cosmic();
                    widget::container::Style {
                        background: Some(Color::from(cosmic.button.base).into()),
                        border: cosmic::iced::Border {
                            radius: cosmic.radius_xl().into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }))
                .into(),
            widget::button::standard("Choose…")
                .on_press(send(CustomizeMessage::ChooseIcons))
                .into(),
        ]))
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
                    "Show list"
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
        .push(preview(theme, icon_style, background))
        .push(banner(draft))
        .push(layout)
        .push(rail)
        .push(keys)
        .push(icons);
    if let Some(message) = message {
        content = content.push(widget::text::caption(message));
    }

    let own_theme = matches!(draft.base, ThemeChoice::User(_));
    let footer: Element<'a, Message> = if draft.naming {
        widget::Row::new()
            .spacing(8)
            .align_y(Alignment::Center)
            .push(
                widget::text_input("Theme name", &draft.save_name)
                    .on_input(move |name| send(CustomizeMessage::SetSaveName(name)))
                    .on_submit(move |_| send(CustomizeMessage::Save))
                    .width(Length::Fill),
            )
            .push(widget::button::standard("Cancel").on_press(send(CustomizeMessage::StopNaming)))
            .push(widget::button::suggested("Save").on_press(send(CustomizeMessage::Save)))
            .into()
    } else {
        let mut row = widget::Row::new()
            .spacing(8)
            .align_y(Alignment::Center)
            .push(
                widget::button::text("Discard changes")
                    .on_press_maybe(draft.edited.then(|| send(CustomizeMessage::Discard))),
            )
            .push(widget::Space::new().width(Length::Fill))
            .push(widget::button::standard("Export…").on_press(Message::ExportTheme));
        row = if own_theme {
            row.push(
                widget::button::standard("Save as…").on_press(send(CustomizeMessage::StartNaming)),
            )
            .push(
                widget::button::suggested("Save")
                    .on_press_maybe(draft.edited.then(|| send(CustomizeMessage::SaveInPlace))),
            )
        } else {
            row.push(
                widget::button::suggested("Save as theme…")
                    .on_press(send(CustomizeMessage::StartNaming)),
            )
        };
        row.into()
    };

    (content.into(), footer)
}

/// A search field over the installed fonts, each listed in its own font
fn font_picker<'a>(query: &'a str, current: Option<&str>) -> Element<'a, Message> {
    let needle = query.trim().to_lowercase();
    let matches = |name: &str| name.to_lowercase().contains(&needle);

    let row = |label: &str, font: cosmic::iced::Font, value: Option<String>| {
        let selected = value.as_deref() == current;
        widget::button::custom(widget::text::body(label.to_string()).font(font))
            .class(cosmic::theme::Button::MenuItem)
            .selected(selected)
            .width(Length::Fill)
            .on_press(Message::Customize(CustomizeMessage::PickFont(value)))
    };
    let mut list = widget::Column::new().spacing(2);
    if matches(KIWI_FONT) {
        list = list.push(row(KIWI_FONT, Theme::default().font(), None));
    }
    let found: Vec<&String> = theme::font_families()
        .iter()
        .filter(|name| matches(name))
        .collect();
    for name in found.iter().take(MAX_FONT_MATCHES) {
        list = list.push(row(name, theme::font_named(name), Some(name.to_string())));
    }
    if found.len() > MAX_FONT_MATCHES {
        list = list.push(widget::text::caption(format!(
            "{} more; type to narrow the list",
            found.len() - MAX_FONT_MATCHES
        )));
    } else if found.is_empty() && !matches(KIWI_FONT) {
        list = list.push(widget::text::caption("No installed font matches"));
    }

    widget::Column::new()
        .spacing(8)
        .push(
            widget::search_input("Search fonts", query)
                .id(font_search_id())
                .on_input(|q| Message::Customize(CustomizeMessage::FontQuery(q)))
                .on_clear(Message::Customize(CustomizeMessage::FontQuery(
                    String::new(),
                ))),
        )
        .push(widget::scrollable(list).height(Length::Fixed(220.0)))
        .into()
}

/// Keys and rail drawn with the theme being edited, including a held key and a repeat
fn preview<'a>(
    theme: &Theme,
    icon_style: IconStyle,
    background: crate::config::PreviewBackground,
) -> Element<'a, Message> {
    let ctrl = KeyModifiers {
        ctrl: true,
        ..Default::default()
    };
    let key = |k: &str| Keystroke::single(k, false);
    let keys: Vec<Keystroke> = match theme.layout {
        Layout::Keys => vec![
            key("V"),
            Keystroke::combination(&ctrl, "C", false),
            Keystroke {
                count: 3,
                ..key("A")
            },
            key("LClick"),
            Keystroke::single("↵", true),
        ],
        Layout::Text => "git commit"
            .chars()
            .map(|c| key(&c.to_string()))
            .chain([key("LClick"), Keystroke::combination(&ctrl, "S", true)])
            .collect(),
    };
    let sample = keystrokes_row::<Message>(
        &keys,
        34.0,
        60.0, // long enough that the sample never fades
        theme,
        260.0,
        // Right-aligned so the order reads left to right, like typing
        OverlayPosition::TopRight,
        // The row's length is in key widths; leave room for all of the sample
        12,
        icon_style,
        crate::keystroke::Motion::default(),
    );
    widget::container(cosmic::iced::widget::stack![
        crate::settings::preview_backdrop(background),
        widget::container(sample)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center),
    ])
    .width(Length::Fill)
    .height(Length::Fixed(96.0))
    .clip(true)
    .into()
}

/// Which theme is being edited, and whether saving is needed
fn banner<'a>(draft: &Draft) -> Element<'a, Message> {
    let line = match (&draft.base, draft.edited) {
        (_, false) => "Changes show on the overlay as you make them.",
        (ThemeChoice::Builtin(_), true) => {
            "Built-in themes aren't changed. Save to keep this as your own."
        }
        (ThemeChoice::User(_), true) => "Save to keep these changes.",
    };
    widget::container(
        widget::Column::new()
            .spacing(2)
            .push(widget::text::body(draft.name()).font(cosmic::font::semibold()))
            .push(widget::text::caption(line)),
    )
    .padding([8, 12])
    .width(Length::Fill)
    .class(cosmic::theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        let accent = Color::from(cosmic.accent_color());
        widget::container::Style {
            background: Some(Color { a: 0.1, ..accent }.into()),
            border: cosmic::iced::Border {
                color: Color { a: 0.35, ..accent },
                width: 1.0,
                radius: cosmic.radius_s().into(),
            },
            ..Default::default()
        }
    }))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BuiltinTheme;

    #[test]
    fn gradient_stops_edit_separately() {
        let mut theme = Theme::builtin(BuiltinTheme::Frosted);
        let red = Hex::parse("#ff0000").unwrap();
        let (start, _) = match theme.key.background {
            Fill::Gradient(start, end) => (start, end),
            Fill::Solid(_) => panic!("Frosted keys have a gradient"),
        };
        assert!(ColorField::KeyBackground.set_stop(&mut theme, 1, red));
        assert_eq!(theme.key.background, Fill::Gradient(start, Stop::new(red)));
        // A new color keeps its spot along the gradient
        let end = Stop {
            color: red,
            at: Some(0.6),
        };
        theme.key.background = Fill::Gradient(start, end);
        let blue = Hex::parse("#0000ff").unwrap();
        assert!(ColorField::KeyBackground.set_stop(&mut theme, 1, blue));
        assert_eq!(
            theme.key.background,
            Fill::Gradient(start, Stop { color: blue, ..end })
        );
        // Solid fields have no second stop
        assert!(!ColorField::KeyText.set_stop(&mut theme, 1, red));
        // Setting the first stop of an empty divider adds one, but only with a rail
        assert!(!ColorField::RailDivider.set_stop(&mut theme, 0, red));
        let mut ribbon = Theme::builtin(BuiltinTheme::Ribbon);
        assert!(ColorField::RailDivider.set_stop(&mut ribbon, 0, red));
        assert_eq!(ColorField::RailDivider.stop(&ribbon, 0), Some(red));
    }

    #[test]
    fn fields_only_take_what_fits() {
        let mut theme = Theme::builtin(BuiltinTheme::Frosted);
        let gradient = Some(Fill::gradient(
            Hex::parse("#000000").unwrap(),
            Hex::parse("#ffffff").unwrap(),
        ));
        assert!(!ColorField::KeyText.set(&mut theme, gradient));
        assert!(ColorField::KeyBackground.set(&mut theme, gradient));
        // No rail yet, so rail fields can't be set
        assert!(!ColorField::RailBorder.set(
            &mut theme,
            Some(Fill::Solid(Hex::parse("#ffffff").unwrap()))
        ));

        let mut tape = Theme::builtin(BuiltinTheme::Tape);
        assert!(ColorField::RailDivider.set(&mut tape, None));
        assert_eq!(tape.rail.unwrap().divider, None);

        NumberField::KeyGap.nudge(&mut theme, -100.0);
        assert_eq!(theme.key.gap, 0.0);
    }
}
