//! COSMIC panel applet for Kiwi keystroke visualizer.

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{window::Id, Limits, Subscription};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use kiwi_common::{Config, KiwiProxy, APP_ID};

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    cosmic::applet::run::<KiwiApplet>(())
}

/// Connection state to the daemon
#[derive(Debug, Clone, PartialEq, Eq)]
enum DaemonState {
    Unknown,
    Connected,
    Disconnected,
}

struct KiwiApplet {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,
    config_handler: Option<cosmic_config::Config>,
    daemon_state: Arc<Mutex<DaemonState>>,
}

#[derive(Debug, Clone)]
enum Message {
    TogglePopup,
    PopupClosed(Id),
    ToggleEnabled(bool),
    ConfigChanged(Config),
    Tick,
}

/// Spawn the daemon process
fn spawn_daemon() {
    log::info!("Spawning kiwi-daemon...");
    match Command::new("kiwi-daemon").spawn() {
        Ok(_) => log::info!("Daemon spawned"),
        Err(e) => log::error!("Failed to spawn daemon: {}", e),
    }
}

/// Try to connect to daemon and set enabled state
fn try_set_enabled(enabled: bool, daemon_state: Arc<Mutex<DaemonState>>) {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let connection = match zbus::Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to connect to D-Bus session: {}", e);
                    return;
                }
            };

            let proxy = match KiwiProxy::new(&connection).await {
                Ok(p) => p,
                Err(_) => {
                    if enabled {
                        spawn_daemon();
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        
                        match KiwiProxy::new(&connection).await {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("Failed to connect to daemon after spawn: {}", e);
                                if let Ok(mut state) = daemon_state.lock() {
                                    *state = DaemonState::Disconnected;
                                }
                                return;
                            }
                        }
                    } else {
                        if let Ok(mut state) = daemon_state.lock() {
                            *state = DaemonState::Disconnected;
                        }
                        return;
                    }
                }
            };

            if let Err(e) = proxy.set_enabled(enabled).await {
                log::error!("Failed to set enabled: {}", e);
            } else {
                log::info!("Daemon set_enabled({}) success", enabled);
            }

            if let Ok(mut state) = daemon_state.lock() {
                *state = DaemonState::Connected;
            }
        });
    });
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
        let config_handler = cosmic_config::Config::new(APP_ID, Config::VERSION).ok();
        let config = config_handler
            .as_ref()
            .and_then(|h| Config::get_entry(h).ok())
            .unwrap_or_default();

        log::info!("Loaded config: enabled={}", config.enabled);

        let daemon_state = Arc::new(Mutex::new(DaemonState::Unknown));

        if config.enabled {
            try_set_enabled(true, daemon_state.clone());
        }

        let app = Self {
            core,
            popup: None,
            config,
            config_handler,
            daemon_state,
        };
        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let icon_name = if self.config.enabled {
            "keyboard-brightness-symbolic"
        } else {
            "input-keyboard-symbolic"
        };

        self.core
            .applet
            .icon_button(icon_name)
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let daemon_status = match self.daemon_state.lock() {
            Ok(state) => match *state {
                DaemonState::Connected => "Connected",
                DaemonState::Disconnected => "Disconnected",
                DaemonState::Unknown => "...",
            },
            Err(_) => "Error",
        };

        let content_list = widget::list_column()
            .padding(5)
            .spacing(5)
            .add(widget::settings::item(
                "Enabled",
                widget::toggler(self.config.enabled).on_toggle(Message::ToggleEnabled),
            ))
            .add(widget::text::caption(format!("Daemon: {}", daemon_status)));

        self.core.applet.popup_container(content_list).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        use cosmic::iced::time;
        
        Subscription::batch([
            self.core()
                .watch_config::<Config>(APP_ID)
                .map(|update| Message::ConfigChanged(update.config)),
            time::every(Duration::from_secs(2)).map(|_| Message::Tick),
        ])
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
                        .min_height(100.0)
                        .max_height(400.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::ToggleEnabled(enabled) => {
                self.config.enabled = enabled;
                log::info!("Keystrokes enabled: {}", enabled);
                
                if let Some(ref handler) = self.config_handler {
                    if let Err(e) = self.config.write_entry(handler) {
                        log::error!("Failed to save config: {}", e);
                    }
                }

                try_set_enabled(enabled, self.daemon_state.clone());
            }
            Message::ConfigChanged(config) => {
                log::info!("Config changed externally: enabled={}", config.enabled);
                if self.config.enabled != config.enabled {
                    try_set_enabled(config.enabled, self.daemon_state.clone());
                }
                self.config = config;
            }
            Message::Tick => {
                // Periodic check - could verify daemon is still running
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}
