//! Kiwi keystroke visualizer - unified app with settings, tray icon, and overlay

mod capture;
mod color_picker;
mod config;
mod cosmic_xkb;
mod customize;
mod input;
mod keystroke;
mod overlay;
mod settings;
mod theme;
mod tray;
mod widgets;

use std::any::TypeId;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::core::event::wayland::OutputEvent;
use cosmic::iced::event::listen_with;
use cosmic::iced::futures::{SinkExt, StreamExt};
use cosmic::iced::Size;
use cosmic::iced::{window, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::about::About;
use crossbeam_channel::{Receiver as CbReceiver, Sender as CbSender};
use wayland_client::protocol::wl_output::WlOutput;

use config::{Config, APP_ID};
use overlay::{
    create_layer_surface_for_output, destroy_surface, view_overlay, OutputState, SharedState,
};
use theme::{Theme, ThemeChoice};

const REPOSITORY: &str = "https://github.com/hojjatabdollahi/kiwi";
const APP_ICON: &[u8] = include_bytes!("../data/icons/kiwi-on.svg");

/// Context pages for the context drawer
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
    Customize,
}

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (tray_tx, tray_rx) = crossbeam_channel::unbounded::<tray::TrayAction>();

    let settings = cosmic::app::Settings::default()
        .no_main_window(true) // Start with no window - tray only
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(350.0)
                .min_height(400.0),
        )
        .size(Size::new(440.0, 640.0))
        .exit_on_close(false); // Don't exit when window closes - tray stays

    cosmic::app::run::<KiwiApp>(settings, Flags { tray_tx, tray_rx })
}

#[derive(Clone)]
struct Flags {
    tray_tx: CbSender<tray::TrayAction>,
    tray_rx: CbReceiver<tray::TrayAction>,
}

struct KiwiApp {
    core: cosmic::Core,
    config: Config,
    #[allow(dead_code)]
    config_handler: Option<cosmic_config::Config>,
    pending_save: bool,
    /// Crossbeam receiver for tray actions
    tray_rx: CbReceiver<tray::TrayAction>,
    /// Handle to keep tray alive
    #[allow(dead_code)]
    tray_handle: Option<tray::TrayHandle>,
    /// Shared state for overlay (keystrokes, modifiers, etc.)
    shared_state: Arc<Mutex<SharedState>>,
    /// Sends COSMIC Comp XKB config changes to the input thread.
    xkb_config_tx: CbSender<cosmic_xkb::XkbConfig>,
    /// Wayland outputs with layer surfaces
    outputs: Vec<OutputState>,
    /// Current context page for context drawer
    context_page: ContextPage,
    /// About page widget
    about: About,
    /// Every theme the settings window offers, loaded when it opens
    themes: Vec<(ThemeChoice, Arc<Theme>)>,
    /// Result of the last theme import/export, shown under the theme cards
    theme_message: Option<String>,
    /// Set while the overlay is being arranged on screen
    arranging: Option<Arranging>,
    /// The theme being edited in the Customize drawer
    draft: Option<customize::Draft>,
    /// Segmented buttons in the settings window
    display_mode_model: widget::segmented_button::SingleSelectModel,
    icon_style_model: widget::segmented_button::SingleSelectModel,
}

/// An arrange-on-screen session
struct Arranging {
    /// Config from before arranging, restored by Cancel
    before: Config,
    /// Last time the user did something; arranging ends after a minute without input
    last_input: std::time::Instant,
}

/// How long arrange mode waits for input before finishing by itself
const ARRANGE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Message {
    // Window actions
    WindowClosed(window::Id),
    WindowOpened(window::Id),
    // Tray actions (from subscription)
    TrayAction(tray::TrayAction),
    // Tray actions (received from tray menu)
    TrayShowSettings,
    TrayToggleActive,
    TrayQuit,
    // Settings
    ToggleActive(bool),
    SetFadeDuration(f32),
    SetDisappearDuration(f32),
    SelectTheme(ThemeChoice),
    PreviewTheme(ThemeChoice),
    OpenThemesFolder,
    ImportTheme,
    ImportThemeFrom(std::path::PathBuf),
    ExportTheme,
    ExportThemeTo(std::path::PathBuf),
    ThemeMessage(String),
    Customize(customize::CustomizeMessage),
    DisplayModeTab(widget::segmented_button::Entity),
    IconStyleTab(widget::segmented_button::Entity),
    /// Switch what the theme previews are drawn on
    TogglePreviewBackground,
    /// Open the Customize drawer for a theme, selecting it first
    CustomizeTheme(ThemeChoice),
    // Arrange mode
    StartArranging,
    /// Put the newest key here, as fractions of the screen (dragging in arrange mode)
    MoveKeys(f32, f32),
    /// Switch which way the keys grow from their anchor
    FlipGrowth,
    FinishArranging,
    CancelArranging,
    ResetArrangement,
    NudgeSize(f32),
    NudgeLength(i32),
    SetShowKeyboard(bool),
    SetShowMouse(bool),
    SetShowGestures(bool),
    SetShowTouch(bool),
    SetShowTablet(bool),
    SaveConfig,
    ConfigChanged(Config),
    CosmicCompConfigChanged(cosmic_xkb::CosmicCompConfig),
    // Context drawer
    ToggleContextPage(ContextPage),
    LaunchUrl(String),
    // Overlay
    OutputEvent(OutputEvent, WlOutput),
    Tick,
}

impl cosmic::Application for KiwiApp {
    type Executor = cosmic::executor::Default;
    type Flags = Flags;
    type Message = Message;

    const APP_ID: &'static str = "io.github.hojjatabdollahi.kiwi";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        mut core: cosmic::Core,
        flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Don't auto-apply a corner radius to layer surfaces. Our overlay is a
        // full-screen layer surface; libcosmic otherwise sends a corner-radius
        // request for it (auto_corner_radius includes `System`), which cosmic-comp
        // rejects with a "corner radius too large" protocol error, killing the
        // Wayland connection. Window/popup corner radii are left untouched.
        core.set_auto_corner_radius(cosmic::core::Auto::Window | cosmic::core::Auto::Popup);
        // The header bar draws this at the standard COSMIC title size
        core.set_header_title("Kiwi".to_string());

        // Load config
        let config_handler = cosmic_config::Config::new(APP_ID, Config::VERSION).ok();
        // Keys missing from an older config get their defaults instead of
        // resetting the whole config
        let config = config_handler
            .as_ref()
            .map(|h| Config::get_entry(h).unwrap_or_else(|(_, config)| config))
            .unwrap_or_default();

        // Create shared state for overlay (this also loads the theme)
        let mut state = SharedState::default();
        state.update_from_config(&config);
        let shared_state = Arc::new(Mutex::new(state));

        let initial_xkb_config = cosmic_xkb::load_current_config();
        let (xkb_config_tx, xkb_config_rx) =
            crossbeam_channel::unbounded::<cosmic_xkb::XkbConfig>();

        // Always start input capture (it checks enabled state internally)
        // Pass a clone of the tray sender so input thread can trigger deactivation
        input::spawn_input_capture(
            shared_state.clone(),
            flags.tray_tx.clone(),
            initial_xkb_config,
            xkb_config_rx,
        );

        // Create tray icon
        let tray_handle = tray::create_tray(config.enabled, flags.tray_tx);

        // Create the about widget
        let about = About::default()
            .name("Kiwi")
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .author("Hojjat Abdollahi")
            // The built-in themes' artwork, with its license
            .links(
                std::iter::once(("Repository".to_string(), REPOSITORY.to_string())).chain(
                    config::BuiltinTheme::ALL
                        .iter()
                        .flat_map(|builtin| Theme::builtin(*builtin).credits)
                        .map(|c| (format!("{} by {} · {}", c.work, c.author, c.license), c.url)),
                ),
            )
            .license("GPL-3.0");

        let app = Self {
            core,
            config_handler,
            pending_save: false,
            tray_rx: flags.tray_rx,
            tray_handle: Some(tray_handle),
            shared_state,
            xkb_config_tx,
            outputs: Vec::new(),
            context_page: ContextPage::default(),
            about,
            themes: Vec::new(),
            theme_message: None,
            arranging: None,
            draft: None,
            display_mode_model: settings::segmented_model(
                settings::DISPLAY_MODES,
                config.key_display_mode,
            ),
            icon_style_model: settings::segmented_model(settings::ICON_STYLES, config.icon_style),
            config,
        };

        // Load bundled font
        let font_task = keystroke::load_font().map(|result| {
            if let Err(e) = result {
                log::warn!("Failed to load bundled font: {:?}", e);
            }
            cosmic::Action::None
        });

        (app, font_task)
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            // The master switch, the same one the tray toggles
            widget::tooltip(
                widget::toggler(self.config.enabled).on_toggle(Message::ToggleActive),
                "Show keystrokes",
                widget::tooltip::Position::Bottom,
            )
            .into(),
            widget::button::icon(widget::icon::from_name("help-about-symbolic"))
                .on_press(Message::ToggleContextPage(ContextPage::About))
                .into(),
        ]
    }

    fn context_drawer(&self) -> Option<cosmic::app::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => cosmic::app::context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            ),
            ContextPage::Customize => {
                let draft = self.draft.as_ref()?;
                let (content, footer) = customize::view(
                    draft,
                    self.theme_message.as_deref(),
                    self.config.icon_style,
                    self.config.preview_background,
                );
                cosmic::app::context_drawer::context_drawer(
                    content,
                    Message::ToggleContextPage(ContextPage::Customize),
                )
                .title("Customize")
                .footer(footer)
            }
        })
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::WindowClosed(id))
    }

    // Always use a transparent app background (the surface clear color).
    //
    // libcosmic's default `style()` switches the clear color to an opaque
    // `bg_color` when `window.is_maximized`. With this libcosmic + wayland/wgpu
    // setup, that opaque-background path renders the maximized window as a blank
    // fill with no content drawn (the widgets are still there and interactive,
    // just not painted). Forcing the same transparent clear color used for
    // non-maximized windows fixes it: the content container paints its own
    // background (opaque when frosted glass is off, translucent-over-blur when
    // on), so maximized renders identically to windowed.
    //
    // `is_maximized` is global, so this also keeps the transparent overlay
    // transparent while the settings window is maximized.
    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        let theme = cosmic::theme::active();
        Some(cosmic::iced::theme::Style {
            background_color: cosmic::iced::Color::TRANSPARENT,
            icon_color: theme.cosmic().on_bg_color().into(),
            text_color: theme.cosmic().on_bg_color().into(),
        })
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // This is for the settings window (main window when open)
        settings::settings_view(self)
    }

    fn view_window(&self, id: window::Id) -> Element<'_, Self::Message> {
        // Check if this is an overlay (layer surface)
        if self.outputs.iter().any(|o| o.surface_id == id) {
            view_overlay(&self.shared_state, self.is_touch_surface(id))
        } else {
            // Settings window
            settings::settings_view(self)
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        use cosmic::iced::time;
        use std::time::Duration;

        let mut subs = vec![
            // Watch for config changes
            self.core()
                .watch_config::<Config>(APP_ID)
                .map(|update| Message::ConfigChanged(update.config)),
            // Watch COSMIC Comp's input-source config. The input-source applet
            // moves the active source to the first layout/variant pair.
            self.core()
                .watch_config::<cosmic_xkb::CosmicCompConfig>(cosmic_xkb::COSMIC_COMP_APP_ID)
                .map(|update| {
                    if !update.errors.is_empty() {
                        log::warn!(
                            "Errors loading COSMIC Comp config {:?}: {:?}",
                            update.keys,
                            update.errors
                        );
                    }
                    Message::CosmicCompConfigChanged(update.config)
                }),
            // Tray actions subscription
            tray_subscription(self.tray_rx.clone()),
            // Wayland output events
            listen_with(|event, _, _| {
                if let cosmic::iced::core::Event::PlatformSpecific(
                    cosmic::iced::core::event::PlatformSpecific::Wayland(
                        cosmic::iced::core::event::wayland::Event::Output(output_event, wl_output),
                    ),
                ) = event
                {
                    Some(Message::OutputEvent(output_event, wl_output))
                } else {
                    None
                }
            }),
            // Periodic tick to update overlay and clean up expired keystrokes
            time::every(Duration::from_millis(50)).map(|_| Message::Tick),
        ];

        // Touch markers track the finger and keys slide, so both need a smoother
        // refresh than the 50ms tick, but only while it's happening
        if self
            .shared_state
            .lock()
            .map(|s| s.has_touches() || s.is_sliding())
            .unwrap_or(false)
        {
            subs.push(time::every(Duration::from_millis(16)).map(|_| Message::Tick));
        }

        // Esc cancels arranging (the overlay has keyboard focus meanwhile)
        if self.arranging.is_some() {
            subs.push(listen_with(|event, _, _| {
                use cosmic::iced::keyboard::{key::Named, Event, Key};
                match event {
                    cosmic::iced::core::Event::Keyboard(Event::KeyPressed {
                        key: Key::Named(Named::Escape),
                        ..
                    }) => Some(Message::CancelArranging),
                    _ => None,
                }
            }));
        }

        // Debounce timer for config save
        if self.pending_save {
            subs.push(time::every(Duration::from_millis(300)).map(|_| Message::SaveConfig));
        }

        Subscription::batch(subs)
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::WindowClosed(id) => {
                // Check if this is the settings window
                if self.core.main_window_id() == Some(id) {
                    self.core_mut().set_main_window_id(None);
                    log::info!("Settings window closed, app continues in tray");
                    return cosmic::iced::window::close(id);
                }
                // Otherwise might be a layer surface being destroyed
            }
            Message::TrayShowSettings => {
                // If window already exists, close it (toggle behavior)
                if let Some(id) = self.core.main_window_id() {
                    log::info!("Window already open ({:?}), closing", id);
                    self.core_mut().set_main_window_id(None);
                    return cosmic::iced::window::close(id);
                }

                // No window exists, open a new one
                self.reload_themes();
                let settings = window::Settings {
                    size: Size::new(440.0, 640.0),
                    decorations: false, // libcosmic provides its own header bar
                    // The window is drawn translucent (see `style`, and frosted glass).
                    // An opaque window leaves those pixels undefined, so the
                    // background flickers between old frames.
                    transparent: true,
                    ..Default::default()
                };
                let (id, task) = cosmic::iced::window::open(settings);
                self.core_mut().set_main_window_id(Some(id));
                log::info!("Opening new window with id: {:?}", id);
                return task.map(|id| cosmic::Action::App(Message::WindowOpened(id)));
            }
            Message::TrayToggleActive => {
                let new_active = !self.config.enabled;
                return self.update(Message::ToggleActive(new_active));
            }
            Message::TrayQuit => {
                std::process::exit(0);
            }
            Message::ToggleActive(active) => {
                self.config.enabled = active;
                self.save_config();
                self.update_tray_state();

                // Update shared state
                if let Ok(mut state) = self.shared_state.lock() {
                    state.enabled = active;

                    // When disabling, clear all input state to prevent stale modifiers
                    if !active {
                        state.modifiers = keystroke::KeyModifiers::default();
                        state.peak_modifiers = keystroke::KeyModifiers::default();
                        state.current_key = None;
                        state.current_mouse = None;
                        state.key_pressed_with_modifiers = false;
                        state.history.clear();
                        state.touches.clear();
                    }
                }

                // If enabling and input capture isn't running, start it
                if active {
                    // Input capture is already spawned at init - just enable in state
                    log::info!("Keystrokes enabled");
                } else {
                    log::info!("Keystrokes disabled");
                }
            }
            Message::SetFadeDuration(duration) => {
                self.config.fade_duration = duration;
                self.pending_save = true;

                // Update shared state
                if let Ok(mut state) = self.shared_state.lock() {
                    state.life.linger = duration;
                }
            }
            Message::SetDisappearDuration(duration) => {
                self.config.disappear_duration = duration;
                self.pending_save = true;
                if let Ok(mut state) = self.shared_state.lock() {
                    state.life.disappear = duration;
                }
            }
            Message::SelectTheme(choice) => self.select_theme(choice),
            Message::PreviewTheme(choice) => {
                let theme = self
                    .themes
                    .iter()
                    .find(|(c, _)| *c == choice)
                    .map(|(_, theme)| theme.clone())
                    .unwrap_or_else(|| Arc::new(choice.load(&theme::themes_dir())));
                if let Ok(mut state) = self.shared_state.lock() {
                    state.preview = Some(overlay::Preview::new(theme));
                }
            }
            Message::OpenThemesFolder => {
                let dir = theme::themes_dir();
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    log::error!("Can't create {}: {}", dir.display(), e);
                }
                if let Err(e) = open::that_detached(&dir) {
                    log::error!("Failed to open {}: {}", dir.display(), e);
                }
            }
            Message::ImportTheme => {
                use cosmic::dialog::file_chooser::{open, FileFilter};
                let dialog = open::Dialog::new()
                    .title("Import a Kiwi theme".to_string())
                    .filter(FileFilter::new("Kiwi theme").glob("*.zip"));
                return cosmic::task::future(async move {
                    match dialog.open_file().await {
                        Ok(response) => match response.url().to_file_path() {
                            Ok(path) => Message::ImportThemeFrom(path).into(),
                            Err(()) => {
                                Message::ThemeMessage("Only local files can be imported".into())
                                    .into()
                            }
                        },
                        Err(e) => dialog_failed(e),
                    }
                });
            }
            Message::ImportThemeFrom(path) => match theme::import(&path, &theme::themes_dir()) {
                Ok(name) => {
                    self.reload_themes();
                    self.theme_message = Some(format!("Imported “{name}”"));
                    self.select_theme(ThemeChoice::User(name));
                }
                Err(e) => self.theme_message = Some(format!("Can't import: {e}")),
            },
            Message::ExportTheme => {
                use cosmic::dialog::file_chooser::save;
                let file_name = format!("{}.zip", ThemeChoice::from_config(&self.config).name());
                let dialog = save::Dialog::new()
                    .title("Export theme".to_string())
                    .file_name(file_name);
                return cosmic::task::future(async move {
                    match dialog.save_file().await {
                        Ok(response) => match response.url().map(|url| url.to_file_path()) {
                            Some(Ok(path)) => Message::ExportThemeTo(path).into(),
                            _ => {
                                Message::ThemeMessage("Only local files can be saved".into()).into()
                            }
                        },
                        Err(e) => dialog_failed(e),
                    }
                });
            }
            Message::ExportThemeTo(path) => {
                let dir = theme::themes_dir();
                // Unsaved edits are exported as they are
                let source = match &self.draft {
                    Some(draft) if draft.edited => Ok((draft.theme.to_ron(), draft.theme.clone())),
                    _ => {
                        let choice = ThemeChoice::from_config(&self.config);
                        choice.file_text(&dir).map(|text| (text, choice.load(&dir)))
                    }
                };
                let result =
                    source.and_then(|(text, theme)| theme::export(&text, &theme.files(), &path));
                self.theme_message = Some(match result {
                    Ok(()) => format!("Exported to {}", path.display()),
                    Err(e) => format!("Can't export: {e}"),
                });
            }
            Message::ThemeMessage(message) => self.theme_message = Some(message),
            Message::Customize(message) => return self.update_customize(message),
            Message::FlipGrowth => {
                self.config.grow_left = Some(!self.config.grows_left());
                self.apply_arrangement();
            }
            Message::MoveKeys(x, y) => {
                self.config.anchor = Some((x, y));
                self.apply_arrangement();
            }
            Message::StartArranging => {
                if self.arranging.is_none() {
                    self.arranging = Some(Arranging {
                        before: self.config.clone(),
                        last_input: std::time::Instant::now(),
                    });
                    if let Ok(mut state) = self.shared_state.lock() {
                        state.arranging = true;
                    }
                    return self.set_overlays_interactive(true);
                }
            }
            Message::FinishArranging => return self.stop_arranging(true),
            Message::CancelArranging => return self.stop_arranging(false),
            Message::ResetArrangement => {
                let (default_size, layout) = self
                    .shared_state
                    .lock()
                    .map(|s| (s.theme.default_size, s.theme.layout))
                    .unwrap_or((64.0, theme::Layout::Keys));
                let defaults = Config::default();
                self.config.key_size = default_size;
                self.config.position = defaults.position;
                self.config.anchor = defaults.anchor;
                self.config.grow_left = defaults.grow_left;
                self.config.line_width = None;
                if layout == theme::Layout::Keys {
                    self.config.history_count = defaults.history_count;
                }
                self.apply_arrangement();
            }
            Message::NudgeSize(delta) => {
                self.config.key_size = (self.config.key_size + delta).clamp(32.0, 160.0);
                self.apply_arrangement();
            }
            Message::NudgeLength(steps) => {
                let Ok(theme) = self.shared_state.lock().map(|s| s.theme.clone()) else {
                    return Task::none();
                };
                match theme.layout {
                    // Keys: how many keystrokes stay on screen
                    theme::Layout::Keys => {
                        self.config.history_count =
                            (self.config.history_count as i32 + steps).clamp(1, 10) as u8;
                    }
                    // Text: how wide the line is
                    theme::Layout::Text => {
                        let width = self.config.line_width.unwrap_or(theme.line_width);
                        self.config.line_width =
                            Some((width + steps as f32 * 40.0).clamp(160.0, 1600.0));
                    }
                }
                self.apply_arrangement();
            }
            Message::DisplayModeTab(entity) => {
                self.display_mode_model.activate(entity);
                let Some(&mode) = self
                    .display_mode_model
                    .data::<config::KeyDisplayMode>(entity)
                else {
                    return Task::none();
                };
                self.config.key_display_mode = mode;
                self.save_config();

                // Update shared state
                if let Ok(mut state) = self.shared_state.lock() {
                    state.key_display_mode = mode;
                }
            }
            Message::TogglePreviewBackground => {
                self.config.preview_background = self.config.preview_background.toggled();
                self.save_config();
            }
            Message::CustomizeTheme(choice) => {
                if choice != ThemeChoice::from_config(&self.config) {
                    self.select_theme(choice);
                }
                return self.update_customize(customize::CustomizeMessage::Open);
            }
            Message::IconStyleTab(entity) => {
                self.icon_style_model.activate(entity);
                let Some(&style) = self.icon_style_model.data::<config::IconStyle>(entity) else {
                    return Task::none();
                };
                self.config.icon_style = style;
                self.save_config();

                // Update shared state
                if let Ok(mut state) = self.shared_state.lock() {
                    state.icon_style = style;
                }
            }
            Message::SetShowKeyboard(show) => {
                self.config.show_keyboard = show;
                self.save_config();

                if let Ok(mut state) = self.shared_state.lock() {
                    state.show_keyboard = show;
                }
            }
            Message::SetShowMouse(show) => {
                self.config.show_mouse = show;
                self.save_config();

                if let Ok(mut state) = self.shared_state.lock() {
                    state.show_mouse = show;
                }
            }
            Message::SetShowGestures(show) => {
                self.config.show_gestures = show;
                self.save_config();

                if let Ok(mut state) = self.shared_state.lock() {
                    state.show_gestures = show;
                }
            }
            Message::SetShowTouch(show) => {
                self.config.show_touch = show;
                self.save_config();

                if let Ok(mut state) = self.shared_state.lock() {
                    state.show_touch = show;
                    if !show {
                        state.touches.clear();
                    }
                }
            }
            Message::SetShowTablet(show) => {
                self.config.show_tablet = show;
                self.save_config();

                if let Ok(mut state) = self.shared_state.lock() {
                    state.show_tablet = show;
                }
            }
            Message::SaveConfig => {
                if self.pending_save {
                    self.pending_save = false;
                    self.save_config();
                }
            }
            Message::ConfigChanged(config) => {
                log::info!("Config changed externally: enabled={}", config.enabled);
                self.config = config.clone();
                self.update_tray_state();
                settings::select_segment(&mut self.display_mode_model, config.key_display_mode);
                settings::select_segment(&mut self.icon_style_model, config.icon_style);

                // Update shared state - layout positioning is handled in view
                if let Ok(mut state) = self.shared_state.lock() {
                    state.update_from_config(&config);
                }
            }
            Message::CosmicCompConfigChanged(config) => {
                let source = config.xkb_config.active_source();
                log::info!(
                    "COSMIC input source changed: layout='{}' variant='{}'",
                    source.layout,
                    source.variant
                );
                if let Err(e) = self.xkb_config_tx.send(config.xkb_config) {
                    log::warn!("Failed to send XKB config update to input thread: {}", e);
                }
            }
            Message::WindowOpened(id) => {
                log::info!("WindowOpened: {:?}", id);
                self.core_mut().set_main_window_id(Some(id));
                return cosmic::iced::window::gain_focus(id).map(|_: ()| cosmic::Action::None);
            }
            Message::OutputEvent(event, wl_output) => {
                return self.handle_output_event(event, wl_output);
            }
            Message::Tick => {
                // Clean up expired keystrokes
                if let Ok(mut state) = self.shared_state.lock() {
                    state.cleanup_expired();
                }
                // Never leave the overlay holding input if arranging was abandoned
                if self
                    .arranging
                    .as_ref()
                    .is_some_and(|a| a.last_input.elapsed() > ARRANGE_TIMEOUT)
                {
                    log::info!("No input while arranging, finishing");
                    return self.stop_arranging(true);
                }
            }
            Message::TrayAction(action) => match action {
                tray::TrayAction::ShowSettings => return self.update(Message::TrayShowSettings),
                tray::TrayAction::ToggleActive => return self.update(Message::TrayToggleActive),
                tray::TrayAction::Arrange => return self.update(Message::StartArranging),
                tray::TrayAction::Quit => return self.update(Message::TrayQuit),
            },
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }
            Message::LaunchUrl(url) => {
                if let Err(e) = open::that_detached(&url) {
                    log::error!("Failed to open URL {}: {}", url, e);
                }
            }
        }
        Task::none()
    }
}

/// Connector names used by built-in panels (eDP-1, LVDS-1, DSI-1, ...)
fn is_internal_output(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    ["edp", "lvds", "dsi"].iter().any(|p| name.starts_with(p))
}

fn tray_subscription(rx: CbReceiver<tray::TrayAction>) -> Subscription<Message> {
    use cosmic::iced::Subscription;

    struct TraySub;
    struct TrayRx(CbReceiver<tray::TrayAction>);

    impl Hash for TrayRx {
        fn hash<H: Hasher>(&self, state: &mut H) {
            TypeId::of::<TraySub>().hash(state);
        }
    }

    Subscription::run_with(TrayRx(rx), |TrayRx(rx)| {
        let rx = rx.clone();
        cosmic::iced::stream::channel(
            10,
            move |mut output: cosmic::iced::futures::channel::mpsc::Sender<Message>| async move {
                // Bridge the blocking crossbeam receiver into an async stream.
                let (mut tx, mut async_rx) =
                    cosmic::iced::futures::channel::mpsc::channel::<tray::TrayAction>(10);

                std::thread::spawn(move || {
                    for action in rx.iter() {
                        let _ = tx.try_send(action);
                    }
                });

                while let Some(action) = async_rx.next().await {
                    if output.send(Message::TrayAction(action)).await.is_err() {
                        break;
                    }
                }
            },
        )
    })
}

/// Turn a failed file dialog into a message, staying quiet when it was just cancelled
fn dialog_failed(error: cosmic::dialog::file_chooser::Error) -> cosmic::Action<Message> {
    match error {
        cosmic::dialog::file_chooser::Error::Cancelled => cosmic::Action::None,
        e => Message::ThemeMessage(format!("File dialog failed: {e}")).into(),
    }
}

impl KiwiApp {
    /// Re-read the list of themes (built-in and the themes folder)
    fn reload_themes(&mut self) {
        let dir = theme::themes_dir();
        self.themes = ThemeChoice::all(&dir)
            .into_iter()
            .map(|choice| {
                let theme = Arc::new(choice.load(&dir));
                (choice, theme)
            })
            .collect();
    }

    fn select_theme(&mut self, choice: ThemeChoice) {
        match &choice {
            ThemeChoice::Builtin(palette) => {
                self.config.palette = *palette;
                self.config.user_theme = None;
            }
            ThemeChoice::User(name) => self.config.user_theme = Some(name.clone()),
        }
        self.save_config();

        let theme = self
            .themes
            .iter()
            .find(|(c, _)| *c == choice)
            .map(|(_, theme)| theme.clone())
            .unwrap_or_else(|| Arc::new(choice.load(&theme::themes_dir())));
        // Picking another theme while customizing starts over from that theme
        if self.draft.is_some() {
            self.draft = Some(customize::Draft::new(choice.clone(), Theme::clone(&theme)));
        }
        if let Ok(mut state) = self.shared_state.lock() {
            state.theme_choice = choice;
            state.theme = theme;
        }
    }

    fn current_theme_name(&self) -> String {
        match &self.draft {
            Some(draft) => draft.name(),
            None => ThemeChoice::from_config(&self.config).name().to_string(),
        }
    }

    fn save_config(&self) {
        if let Some(ref handler) = self.config_handler {
            if let Err(e) = self.config.write_entry(handler) {
                log::error!("Failed to save config: {}", e);
            }
        }
    }

    fn update_tray_state(&self) {
        if let Some(ref handle) = self.tray_handle {
            let is_active = self.config.enabled;
            handle.update(move |tray| {
                tray.set_active(is_active);
            });
        }
    }

    /// Whether `id` is the surface that should draw touchscreen contacts.
    ///
    /// libinput doesn't say which output a touch device is mapped to (that
    /// mapping lives in the compositor), so contacts go to the internal panel
    /// when there is one, and otherwise to the first output kiwi saw.
    fn is_touch_surface(&self, id: window::Id) -> bool {
        self.outputs
            .iter()
            .find(|o| o.name.as_deref().is_some_and(is_internal_output))
            .or_else(|| self.outputs.first())
            .is_some_and(|o| o.surface_id == id)
    }

    fn handle_output_event(
        &mut self,
        event: OutputEvent,
        wl_output: WlOutput,
    ) -> Task<cosmic::Action<Message>> {
        match event {
            OutputEvent::Created(info_opt) => {
                let name = info_opt.as_ref().and_then(|i| i.name.clone());
                log::info!("Output created: {:?}", name);

                let surface_id = window::Id::unique();
                let task = create_layer_surface_for_output(&wl_output, surface_id);
                self.outputs.push(OutputState {
                    output: wl_output.clone(),
                    surface_id,
                    name,
                });

                return task;
            }
            OutputEvent::Removed => {
                if let Some(idx) = self.outputs.iter().position(|o| o.output == wl_output) {
                    let removed = self.outputs.remove(idx);
                    log::info!("Output removed: {:?}", removed.name);
                    return destroy_surface(removed.surface_id);
                }
            }
            OutputEvent::InfoUpdate(info) => {
                if let Some(output_state) = self.outputs.iter_mut().find(|o| o.output == wl_output)
                {
                    output_state.name = info.name;
                }
            }
        }
        Task::none()
    }

    /// Show the arranged config on the overlay. It's saved when arranging finishes.
    fn apply_arrangement(&mut self) {
        if let Some(arranging) = &mut self.arranging {
            arranging.last_input = std::time::Instant::now();
        } else {
            self.save_config();
        }
        if let Ok(mut state) = self.shared_state.lock() {
            state.update_from_config(&self.config);
        }
    }

    /// Leave arrange mode, keeping the new arrangement or going back to the old one
    fn stop_arranging(&mut self, keep: bool) -> Task<cosmic::Action<Message>> {
        let Some(arranging) = self.arranging.take() else {
            return Task::none();
        };
        if !keep {
            let before = arranging.before;
            self.config.position = before.position;
            self.config.anchor = before.anchor;
            self.config.grow_left = before.grow_left;
            self.config.key_size = before.key_size;
            self.config.line_width = before.line_width;
            self.config.history_count = before.history_count;
        }
        self.save_config();
        if let Ok(mut state) = self.shared_state.lock() {
            state.arranging = false;
            state.update_from_config(&self.config);
        }
        self.set_overlays_interactive(false)
    }

    fn set_overlays_interactive(&self, interactive: bool) -> Task<cosmic::Action<Message>> {
        Task::batch(
            self.outputs
                .iter()
                .map(|o| overlay::set_interactive(o.surface_id, interactive)),
        )
    }
}
