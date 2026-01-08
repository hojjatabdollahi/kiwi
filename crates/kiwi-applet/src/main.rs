//! COSMIC panel applet for Kiwi keystroke visualizer.

use std::process::Command;

use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{window::Id, Length, Limits, Subscription};
use cosmic::iced_widget::svg::{self, Svg};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use kiwi_common::{Config, PaletteType, APP_ID};

const SERVICE_NAME: &str = "kiwi-daemon.service";

// Embedded eye icons
const ICON_EYE_CLOSED: &[u8] = include_bytes!("../../../data/icons/eye-closed.svg");
const ICON_EYE_OPEN: &[u8] = include_bytes!("../../../data/icons/eye-open.svg");

/// Static palette names for dropdown (must match PaletteType::ALL order)
const PALETTE_NAMES: &[&str] = &["Dark", "Light", "Frosted", "Kiwi"];

/// Check if the daemon service is running
fn is_daemon_running() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", SERVICE_NAME])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Start the daemon service
fn start_daemon() -> bool {
    Command::new("systemctl")
        .args(["--user", "start", SERVICE_NAME])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Stop the daemon service
fn stop_daemon() -> bool {
    Command::new("systemctl")
        .args(["--user", "stop", SERVICE_NAME])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    cosmic::applet::run::<KiwiApplet>(())
}

struct KiwiApplet {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,
    #[allow(dead_code)]
    config_handler: Option<cosmic_config::Config>,
    daemon_running: bool,
    /// Pending config save (for debouncing slider)
    pending_save: bool,
}

#[derive(Debug, Clone)]
enum Message {
    TogglePopup,
    PopupClosed(Id),
    ToggleActive(bool),  // Combined: starts/stops daemon and enables/disables
    SetKeySize(f32),
    SetFadeDuration(f32),
    SetPaletteIndex(usize),
    SaveConfig,
    ConfigChanged(Config),
    RefreshDaemonStatus,
}

impl cosmic::Application for KiwiApplet {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.hojjat.kiwi.applet";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Load config from cosmic-config
        let config_handler = cosmic_config::Config::new(APP_ID, Config::VERSION).ok();
        let config = config_handler
            .as_ref()
            .and_then(|h| Config::get_entry(h).ok())
            .unwrap_or_default();

        let daemon_running = is_daemon_running();
        log::info!("Loaded config: enabled={}, daemon_running={}", config.enabled, daemon_running);

        let app = Self {
            core,
            popup: None,
            config,
            config_handler,
            daemon_running,
            pending_save: false,
        };
        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let is_active = self.config.enabled && self.daemon_running;
        
        let icon_data = if is_active { ICON_EYE_OPEN } else { ICON_EYE_CLOSED };
        
        let handle = svg::Handle::from_memory(icon_data);
        let suggested = self.core.applet.suggested_size(true);
        let (major_padding, minor_padding) = self.core.applet.suggested_padding(true);
        let (horizontal_padding, vertical_padding) = if self.core.applet.is_horizontal() {
            (major_padding, minor_padding)
        } else {
            (minor_padding, major_padding)
        };
        
        // Apply theme color for currentColor in SVG
        // Note: eye-open has red fill that won't use currentColor, so it stays red
        let svg_icon = Svg::new(handle)
            .width(Length::Fixed(suggested.0 as f32))
            .height(Length::Fixed(suggested.1 as f32))
            .class(cosmic::theme::Svg::Custom(std::rc::Rc::new(|theme| {
                svg::Style {
                    color: Some(theme.cosmic().background.on.into()),
                }
            })));

        widget::button::custom(
            widget::layer_container(svg_icon).center(Length::Fill)
        )
        .width(Length::Fixed((suggested.0 + 2 * horizontal_padding) as f32))
        .height(Length::Fixed((suggested.1 + 2 * vertical_padding) as f32))
        .class(cosmic::theme::Button::AppletIcon)
        .on_press(Message::TogglePopup)
        .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        // Find current selection index
        let current_index = PaletteType::ALL
            .iter()
            .position(|p| *p == self.config.palette);

        // Active = daemon running AND enabled
        let is_active = self.daemon_running && self.config.enabled;
        
        let content_list = widget::list_column()
            .padding(5)
            .spacing(0)
            .add(widget::settings::item(
                "Active",
                widget::toggler(is_active).on_toggle(Message::ToggleActive),
            ))
            .add(widget::settings::item(
                format!("Size: {:.0}", self.config.key_size),
                widget::slider(32.0..=256.0, self.config.key_size, Message::SetKeySize)
                    .width(cosmic::iced::Length::Fixed(120.0)),
            ))
            .add(widget::settings::item(
                format!("Fade: {:.1}s", self.config.fade_duration),
                widget::slider(1.0..=10.0, self.config.fade_duration, Message::SetFadeDuration)
                    .width(cosmic::iced::Length::Fixed(120.0)),
            ))
            .add(widget::settings::item(
                "Theme",
                widget::dropdown(PALETTE_NAMES, current_index, Message::SetPaletteIndex),
            ));

        self.core.applet.popup_container(content_list).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        use cosmic::iced::time;
        use std::time::Duration;

        let mut subs = vec![
            // Watch for config changes from other processes
            self.core()
                .watch_config::<Config>(APP_ID)
                .map(|update| Message::ConfigChanged(update.config)),
            // Periodically check daemon status
            time::every(Duration::from_secs(5)).map(|_| Message::RefreshDaemonStatus),
        ];

        // Debounce timer for saving config
        if self.pending_save {
            subs.push(time::every(Duration::from_millis(300)).map(|_| Message::SaveConfig));
        }

        Subscription::batch(subs)
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(300.0)
                        .min_width(200.0)
                        .min_height(80.0)
                        .max_height(400.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::ToggleActive(active) => {
                if active {
                    // Turning ON: start daemon if needed, then enable
                    if !self.daemon_running {
                        log::info!("Starting daemon...");
                        if start_daemon() {
                            self.daemon_running = true;
                            log::info!("Daemon started");
                        } else {
                            log::error!("Failed to start daemon");
                            return cosmic::Task::none();
                        }
                    }
                    self.config.enabled = true;
                    log::info!("Keystrokes enabled");
                } else {
                    // Turning OFF: disable and stop daemon
                    self.config.enabled = false;
                    log::info!("Keystrokes disabled");
                    if self.daemon_running {
                        log::info!("Stopping daemon...");
                        if stop_daemon() {
                            self.daemon_running = false;
                            log::info!("Daemon stopped");
                        } else {
                            log::error!("Failed to stop daemon");
                        }
                    }
                }
                self.save_config();
            }
            Message::SetKeySize(size) => {
                self.config.key_size = size;
                self.pending_save = true;  // Debounce - don't save immediately
            }
            Message::SetFadeDuration(duration) => {
                self.config.fade_duration = duration;
                self.pending_save = true;  // Debounce - don't save immediately
            }
            Message::SetPaletteIndex(index) => {
                if let Some(palette) = PaletteType::ALL.get(index) {
                    self.config.palette = *palette;
                    self.save_config();  // Save immediately for dropdown
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
                self.config = config;
            }
            Message::RefreshDaemonStatus => {
                self.daemon_running = is_daemon_running();
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}

impl KiwiApplet {
    fn save_config(&self) {
        if let Some(ref handler) = self.config_handler {
            if let Err(e) = self.config.write_entry(handler) {
                log::error!("Failed to save config: {}", e);
            }
        }
    }
}
