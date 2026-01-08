//! Kiwi keystroke visualizer - Settings app with tray icon

mod position_selector;
mod tray;

use crossbeam_channel::{Receiver as CbReceiver, Sender as CbSender};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{window, Length, Subscription};
use cosmic::iced::Size;
use cosmic::iced_widget::svg;
use cosmic::iced_widget::Svg;
use cosmic::iced_futures::futures::{SinkExt, StreamExt};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::scrollable;
use kiwi_common::{keystroke_widget, Config, Keystroke, OverlayPosition, PaletteType, APP_ID};
use position_selector::PositionSelector;
use std::any::TypeId;

const SERVICE_NAME: &str = "kiwi-daemon.service";

// Checkerboard pattern SVG for transparency preview
const CHECKERBOARD_SVG: &[u8] =
    b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\">\
  <rect width=\"8\" height=\"8\" fill=\"rgb(204,204,204)\"/>\
  <rect x=\"8\" width=\"8\" height=\"8\" fill=\"rgb(153,153,153)\"/>\
  <rect y=\"8\" width=\"8\" height=\"8\" fill=\"rgb(153,153,153)\"/>\
  <rect x=\"8\" y=\"8\" width=\"8\" height=\"8\" fill=\"rgb(204,204,204)\"/>\
</svg>";

/// Static palette names for dropdown
const PALETTE_NAMES: &[&str] = &["Dark", "Light", "Frosted", "Kiwi"];

/// Check if the daemon service is running (async)
async fn is_daemon_running_async() -> bool {
    tokio::process::Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", SERVICE_NAME])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Start the daemon service (async)
async fn start_daemon_async() -> bool {
    tokio::process::Command::new("systemctl")
        .args(["--user", "start", SERVICE_NAME])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Stop the daemon service (async)
async fn stop_daemon_async() -> bool {
    tokio::process::Command::new("systemctl")
        .args(["--user", "stop", SERVICE_NAME])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (tray_tx, tray_rx) = crossbeam_channel::unbounded::<tray::TrayAction>();

    let settings = cosmic::app::Settings::default()
        .size_limits(cosmic::iced::Limits::NONE.min_width(350.0).min_height(400.0))
        .size(cosmic::iced::Size::new(400.0, 550.0))
        .exit_on_close(false); // Don't exit when window closes - tray stays

    cosmic::app::run::<KiwiApp>(
        settings,
        Flags {
            tray_tx,
            tray_rx,
        },
    )
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
    daemon_running: bool,
    pending_save: bool,
    window_visible: bool,
    /// Crossbeam receiver for tray actions (works even if no window is visible)
    tray_rx: CbReceiver<tray::TrayAction>,
    /// Handle to keep tray alive
    #[allow(dead_code)]
    tray_handle: Option<ksni::Handle<tray::KiwiTray>>,
}

#[derive(Debug, Clone)]
enum Message {
    // Window actions
    WindowClosed(window::Id),
    WindowOpened(window::Id),
    // Tray actions (from subscription)
    TrayAction(tray::TrayAction),
    // Tray actions (received from tray menu)
    TrayShowSettings,
    TrayToggleActive,
    TrayQuit,
    // Daemon control
    ToggleActive(bool),
    DaemonStatusChecked(bool),
    DaemonStarted(bool),
    DaemonStopped(bool),
    RefreshDaemonStatus,
    // Settings
    SetKeySize(f32),
    SetFadeDuration(f32),
    SetPaletteIndex(usize),
    SetPosition(OverlayPosition),
    SaveConfig,
    ConfigChanged(Config),
}

impl cosmic::Application for KiwiApp {
    type Executor = cosmic::executor::Default;
    type Flags = Flags;
    type Message = Message;

    const APP_ID: &'static str = "dev.hojjat.kiwi";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Load config
        let config_handler = cosmic_config::Config::new(APP_ID, Config::VERSION).ok();
        let config = config_handler
            .as_ref()
            .and_then(|h| Config::get_entry(h).ok())
            .unwrap_or_default();

        // Create tray icon
        let tray_handle = tray::create_tray(config.enabled, flags.tray_tx);

        let app = Self {
            core,
            config,
            config_handler,
            daemon_running: false,
            pending_save: false,
            window_visible: false, // Start hidden
            tray_rx: flags.tray_rx,
            tray_handle: Some(tray_handle),
        };

        // Check daemon status async
        let task = cosmic::task::future(async {
            let running = is_daemon_running_async().await;
            cosmic::Action::App(Message::DaemonStatusChecked(running))
        });

        (app, task)
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![]
    }

    fn header_center(&self) -> Vec<Element<'_, Self::Message>> {
        vec![widget::text::title3("Kiwi Settings").into()]
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        // When user closes the window, we'll handle it to minimize instead
        Some(Message::WindowClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        self.settings_view()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        use cosmic::iced::time;
        use std::time::Duration;

        let mut subs = vec![
            // Watch for config changes
            self.core()
                .watch_config::<Config>(APP_ID)
                .map(|update| Message::ConfigChanged(update.config)),
            // Periodically check daemon status
            time::every(Duration::from_secs(5)).map(|_| Message::RefreshDaemonStatus),
            // Tray actions (no polling; block on receiver in a background thread)
            tray_subscription(self.tray_rx.clone()),
        ];

        // Debounce timer for config save
        if self.pending_save {
            subs.push(time::every(Duration::from_millis(300)).map(|_| Message::SaveConfig));
        }

        Subscription::batch(subs)
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::WindowClosed(id) => {
                // Let the window close, but app keeps running (tray stays active)
                self.window_visible = false;
                self.core_mut().set_main_window_id(None);
                log::info!("Window closed, app continues in tray");
                return cosmic::iced::window::close(id);
            }
            Message::TrayShowSettings => {
                log::info!("TrayShowSettings: Opening settings window");
                self.window_visible = true;
                
                // If window already exists, just focus it
                if let Some(id) = self.core.main_window_id() {
                    log::info!("Window already open ({:?}), focusing", id);
                    return cosmic::iced::window::gain_focus(id).map(|_: ()| cosmic::Action::None);
                }
                
                // No window exists, open a new one
                let settings = window::Settings {
                    size: Size::new(400.0, 550.0),
                    decorations: false, // libcosmic provides its own header bar
                    ..Default::default()
                };
                let (id, task) = cosmic::iced::window::open(settings);
                self.core_mut().set_main_window_id(Some(id));
                log::info!("Opening new window with id: {:?}", id);
                return task.map(|id| cosmic::Action::App(Message::WindowOpened(id)));
            }
            Message::TrayToggleActive => {
                let new_active = !(self.daemon_running && self.config.enabled);
                return self.update(Message::ToggleActive(new_active));
            }
            Message::TrayQuit => {
                // Actually quit the application
                std::process::exit(0);
            }
            Message::ToggleActive(active) => {
                if active {
                    self.config.enabled = true;
                    self.save_config();
                    self.update_tray_state();
                    log::info!("Keystrokes enabled");

                    if !self.daemon_running {
                        log::info!("Starting daemon...");
                        return cosmic::task::future(async {
                            let success = start_daemon_async().await;
                            cosmic::Action::App(Message::DaemonStarted(success))
                        });
                    }
                } else {
                    self.config.enabled = false;
                    self.save_config();
                    self.update_tray_state();
                    log::info!("Keystrokes disabled");

                    if self.daemon_running {
                        log::info!("Stopping daemon...");
                        return cosmic::task::future(async {
                            let success = stop_daemon_async().await;
                            cosmic::Action::App(Message::DaemonStopped(success))
                        });
                    }
                }
            }
            Message::DaemonStatusChecked(running) => {
                log::info!("Daemon status: {}", running);
                self.daemon_running = running;
                self.update_tray_state();
            }
            Message::DaemonStarted(success) => {
                if success {
                    self.daemon_running = true;
                    log::info!("Daemon started");
                } else {
                    log::error!("Failed to start daemon");
                }
                self.update_tray_state();
            }
            Message::DaemonStopped(success) => {
                if success {
                    self.daemon_running = false;
                    log::info!("Daemon stopped");
                } else {
                    log::error!("Failed to stop daemon");
                }
                self.update_tray_state();
            }
            Message::WindowOpened(id) => {
                log::info!("WindowOpened: {:?}", id);
                self.core_mut().set_main_window_id(Some(id));
                return cosmic::iced::window::gain_focus(id).map(|_: ()| cosmic::Action::None);
            }
            Message::SetKeySize(size) => {
                self.config.key_size = size;
                self.pending_save = true;
            }
            Message::SetFadeDuration(duration) => {
                self.config.fade_duration = duration;
                self.pending_save = true;
            }
            Message::SetPaletteIndex(index) => {
                if let Some(palette) = PaletteType::ALL.get(index) {
                    self.config.palette = *palette;
                    self.save_config();
                }
            }
            Message::SetPosition(position) => {
                self.config.position = position;
                self.save_config();
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
                self.update_tray_state();
            }
            Message::RefreshDaemonStatus => {
                return cosmic::task::future(async {
                    let running = is_daemon_running_async().await;
                    cosmic::Action::App(Message::DaemonStatusChecked(running))
                });
            }
            Message::TrayAction(action) => match action {
                tray::TrayAction::ShowSettings => return self.update(Message::TrayShowSettings),
                tray::TrayAction::ToggleActive => return self.update(Message::TrayToggleActive),
                tray::TrayAction::Quit => return self.update(Message::TrayQuit),
            },
        }
        Task::none()
    }
}

fn tray_subscription(rx: CbReceiver<tray::TrayAction>) -> Subscription<Message> {
    use cosmic::iced_futures::Subscription;

    struct TraySub;

    Subscription::run_with_id(
        TypeId::of::<TraySub>(),
        cosmic::iced::stream::channel(10, move |mut output| async move {
            // Bridge the blocking crossbeam receiver into an async stream.
            let (mut tx, mut async_rx) =
                cosmic::iced_futures::futures::channel::mpsc::channel::<tray::TrayAction>(10);

            std::thread::spawn(move || {
                for action in rx.iter() {
                    // Best-effort: if the app is shutting down, ignore.
                    let _ = tx.try_send(action);
                }
            });

            while let Some(action) = async_rx.next().await {
                if output.send(Message::TrayAction(action)).await.is_err() {
                    break;
                }
            }
        }),
    )
}

impl KiwiApp {
    fn save_config(&self) {
        if let Some(ref handler) = self.config_handler {
            if let Err(e) = self.config.write_entry(handler) {
                log::error!("Failed to save config: {}", e);
            }
        }
    }

    fn update_tray_state(&self) {
        if let Some(ref handle) = self.tray_handle {
            // Make the tray immediately reflect the *requested* state.
            // (Daemon state can lag slightly behind systemctl start/stop.)
            let is_active = self.config.enabled;
            handle.update(move |tray| {
                tray.set_active(is_active);
            });
        }
    }

    fn settings_view(&self) -> Element<'_, Message> {
        // Find current selection index
        let current_index = PaletteType::ALL
            .iter()
            .position(|p| *p == self.config.palette);

        // Active = daemon running AND enabled
        let is_active = self.daemon_running && self.config.enabled;

        // Position selector widget (larger size)
        let position_selector =
            PositionSelector::new(160.0, self.config.position, Message::SetPosition);

        // Sample keystroke preview (scales with slider, cap at 250 for window)
        let preview_size = self.config.key_size.min(250.0);
        let sample_keystroke = Keystroke::single("Alt", false);
        let preview = keystroke_widget::<Message>(
            &sample_keystroke,
            preview_size,
            1.0, // fade_duration (unused when fade disabled)
            self.config.palette,
            false, // fade_enabled = false for static preview
        );

        // Checkerboard background for transparency preview
        let checkerboard = Svg::new(svg::Handle::from_memory(CHECKERBOARD_SVG))
            .width(Length::Fill)
            .height(Length::Fill);

        // Stack preview on top of checkerboard
        let preview_with_bg = widget::container(cosmic::iced::widget::stack![
            widget::container(checkerboard)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center),
            widget::container(preview)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center),
        ])
        .width(Length::Fill)
        .height(Length::Fill);

        // Fixed-size container for preview (tall enough for max preview size)
        let preview_container = widget::container(preview_with_bg)
            .width(Length::Fill)
            .height(Length::Fixed(260.0))
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center);

        let position_container = widget::container(position_selector)
            .width(Length::Fill)
            .padding([10, 0]) // vertical padding
            .align_x(cosmic::iced::alignment::Horizontal::Center);

        let content = widget::column()
            .padding(10)
            .spacing(8)
            // Active toggle at top
            .push(
                widget::row()
                    .spacing(10)
                    .align_y(cosmic::iced::Alignment::Center)
                    .push(widget::text::body("Active"))
                    .push(widget::Space::with_width(Length::Fill))
                    .push(widget::toggler(is_active).on_toggle(Message::ToggleActive)),
            )
            // Separator
            .push(widget::divider::horizontal::default())
            // Preview in fixed container (centered)
            .push(preview_container)
            // Size slider and Theme dropdown on same row
            .push(
                widget::row()
                    .spacing(10)
                    .align_y(cosmic::iced::Alignment::Center)
                    .push(widget::tooltip(
                        widget::slider(32.0..=256.0, self.config.key_size, Message::SetKeySize)
                            .width(Length::Fill),
                        "Size",
                        widget::tooltip::Position::Bottom,
                    ))
                    .push(widget::tooltip(
                        widget::dropdown(PALETTE_NAMES, current_index, Message::SetPaletteIndex),
                        "Theme",
                        widget::tooltip::Position::Bottom,
                    )),
            )
            // Separator
            .push(widget::divider::horizontal::default())
            // Fade slider
            .push(
                widget::row()
                    .spacing(10)
                    .align_y(cosmic::iced::Alignment::Center)
                    .push(widget::text::body(format!(
                        "Fade: {:.1}s",
                        self.config.fade_duration
                    )))
                    .push(
                        widget::slider(
                            1.0..=10.0,
                            self.config.fade_duration,
                            Message::SetFadeDuration,
                        )
                        .width(Length::Fill),
                    ),
            )
            .push(widget::divider::horizontal::default())
            // Position selector (centered, no label, with padding)
            .push(position_container);

        // Wrap content in scrollable so all widgets are accessible
        widget::container(
            scrollable(content)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
