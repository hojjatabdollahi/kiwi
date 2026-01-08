//! COSMIC panel applet for Kiwi keystroke visualizer.

use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{window::Id, Limits, Subscription};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use kiwi_common::{Config, APP_ID};

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
}

#[derive(Debug, Clone)]
enum Message {
    TogglePopup,
    PopupClosed(Id),
    ToggleEnabled(bool),
    ConfigChanged(Config),
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

        log::info!("Loaded config: enabled={}", config.enabled);

        let app = Self {
            core,
            popup: None,
            config,
            config_handler,
        };
        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // Show different icon based on enabled state
        let icon_name = if self.config.enabled {
            "input-keyboard-symbolic"  // TODO: Use custom icon with indicator
        } else {
            "input-keyboard-symbolic"
        };

        let button = self.core
            .applet
            .icon_button(icon_name)
            .on_press(Message::TogglePopup);

        // When enabled, add a colored indicator by wrapping in a container
        if self.config.enabled {
            widget::container(button)
                .class(cosmic::theme::Container::custom(|_theme| {
                    cosmic::iced_widget::container::Style {
                        text_color: Some(cosmic::iced::Color::from_rgb(0.9, 0.2, 0.2)),
                        icon_color: Some(cosmic::iced::Color::from_rgb(0.9, 0.2, 0.2)),
                        ..cosmic::iced_widget::container::Style::default()
                    }
                }))
                .into()
        } else {
            button.into()
        }
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let content_list = widget::list_column()
            .padding(5)
            .spacing(0)
            .add(widget::settings::item(
                "Enabled",
                widget::toggler(self.config.enabled).on_toggle(Message::ToggleEnabled),
            ));

        self.core.applet.popup_container(content_list).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        // Watch for config changes from other processes
        self.core()
            .watch_config::<Config>(APP_ID)
            .map(|update| Message::ConfigChanged(update.config))
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
            Message::ToggleEnabled(enabled) => {
                self.config.enabled = enabled;
                log::info!("Keystrokes enabled: {}", enabled);
                
                // Save config
                if let Some(ref handler) = self.config_handler {
                    if let Err(e) = self.config.write_entry(handler) {
                        log::error!("Failed to save config: {}", e);
                    } else {
                        log::info!("Config saved: enabled={}", enabled);
                    }
                }
            }
            Message::ConfigChanged(config) => {
                log::info!("Config changed externally: enabled={}", config.enabled);
                self.config = config;
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}
