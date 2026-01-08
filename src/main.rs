//! Kiwi keystroke visualizer - unified app with settings, tray icon, and overlay

mod capture;
mod config;
mod input;
mod keystroke;
mod overlay;
mod position_selector;
mod settings;
mod tray;

use std::any::TypeId;
use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver as CbReceiver, Sender as CbSender};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{window, Subscription};
use cosmic::iced::Size;
use cosmic::iced_core::event::wayland::OutputEvent;
use cosmic::iced_futures::event::listen_with;
use cosmic::iced_futures::futures::{SinkExt, StreamExt};
use cosmic::prelude::*;
use cosmic::widget;
use wayland_client::protocol::wl_output::WlOutput;

use config::{Config, OverlayPosition, PaletteType, APP_ID};
use overlay::{create_layer_surface_for_output, destroy_surface, view_overlay, OutputState, SharedState};

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (tray_tx, tray_rx) = crossbeam_channel::unbounded::<tray::TrayAction>();

    let settings = cosmic::app::Settings::default()
        .no_main_window(true) // Start with no window - tray only
        .size_limits(cosmic::iced::Limits::NONE.min_width(350.0).min_height(400.0))
        .size(Size::new(400.0, 550.0))
        .exit_on_close(false); // Don't exit when window closes - tray stays

    cosmic::app::run::<KiwiApp>(
        settings,
        Flags { tray_tx, tray_rx },
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
    pending_save: bool,
    /// Crossbeam receiver for tray actions
    tray_rx: CbReceiver<tray::TrayAction>,
    /// Handle to keep tray alive
    #[allow(dead_code)]
    tray_handle: Option<tray::TrayHandle>,
    /// Shared state for overlay (keystrokes, modifiers, etc.)
    shared_state: Arc<Mutex<SharedState>>,
    /// Wayland outputs with layer surfaces
    outputs: Vec<OutputState>,
}

#[derive(Debug, Clone)]
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
    SetKeySize(f32),
    SetFadeDuration(f32),
    SetPaletteIndex(usize),
    SetPosition(OverlayPosition),
    SaveConfig,
    ConfigChanged(Config),
    // Overlay
    OutputEvent(OutputEvent, WlOutput),
    Tick,
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

        // Create shared state for overlay
        let shared_state = Arc::new(Mutex::new(SharedState::new(
            config.enabled,
            config.key_size,
            config.fade_duration,
            config.palette,
            config.position,
        )));

        // Always start input capture (it checks enabled state internally)
        input::spawn_input_capture(shared_state.clone());

        // Create tray icon
        let tray_handle = tray::create_tray(config.enabled, flags.tray_tx);

        let app = Self {
            core,
            config,
            config_handler,
            pending_save: false,
            tray_rx: flags.tray_rx,
            tray_handle: Some(tray_handle),
            shared_state,
            outputs: Vec::new(),
        };

        (app, Task::none())
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![]
    }

    fn header_center(&self) -> Vec<Element<'_, Self::Message>> {
        vec![widget::text::title3("Kiwi Settings").into()]
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::WindowClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // This is for the settings window (main window when open)
        settings::settings_view(
            self.config.key_size,
            self.config.fade_duration,
            self.config.palette,
            self.config.position,
            self.config.enabled,
        )
    }

    fn view_window(&self, id: window::Id) -> Element<'_, Self::Message> {
        // Check if this is an overlay (layer surface)
        if let Some(output) = self.outputs.iter().find(|o| o.surface_id == id) {
            let window_width = output.width as f32;
            view_overlay(&self.shared_state, window_width)
        } else {
            // Settings window
            settings::settings_view(
                self.config.key_size,
                self.config.fade_duration,
                self.config.palette,
                self.config.position,
                self.config.enabled,
            )
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
            // Tray actions subscription
            tray_subscription(self.tray_rx.clone()),
            // Wayland output events
            listen_with(|event, _, _| {
                if let cosmic::iced_core::Event::PlatformSpecific(
                    cosmic::iced_core::event::PlatformSpecific::Wayland(
                        cosmic::iced_core::event::wayland::Event::Output(output_event, wl_output),
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
                log::info!("TrayShowSettings: Opening settings window");
                
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
                }
                
                // If enabling and input capture isn't running, start it
                if active {
                    // Input capture is already spawned at init - just enable in state
                    log::info!("Keystrokes enabled");
                } else {
                    log::info!("Keystrokes disabled");
                }
            }
            Message::SetKeySize(size) => {
                self.config.key_size = size;
                self.pending_save = true;
                
                // Update shared state
                if let Ok(mut state) = self.shared_state.lock() {
                    state.key_size = size;
                }
            }
            Message::SetFadeDuration(duration) => {
                self.config.fade_duration = duration;
                self.pending_save = true;
                
                // Update shared state
                if let Ok(mut state) = self.shared_state.lock() {
                    state.fade_duration = duration;
                }
            }
            Message::SetPaletteIndex(index) => {
                if let Some(palette) = PaletteType::ALL.get(index) {
                    self.config.palette = *palette;
                    self.save_config();
                    
                    // Update shared state
                    if let Ok(mut state) = self.shared_state.lock() {
                        state.palette = *palette;
                    }
                }
            }
            Message::SetPosition(position) => {
                let old_position = self.config.position;
                self.config.position = position;
                self.save_config();
                
                // Update shared state
                if let Ok(mut state) = self.shared_state.lock() {
                    state.position = position;
                }
                
                // Recreate layer surfaces if position changed
                if old_position != position {
                    return self.recreate_layer_surfaces();
                }
            }
            Message::SaveConfig => {
                if self.pending_save {
                    self.pending_save = false;
                    self.save_config();
                    
                    // Check if key_size changed significantly - recreate surfaces
                    let old_key_size = self.shared_state
                        .lock()
                        .map(|s| s.key_size)
                        .unwrap_or(36.0);
                    if (old_key_size - self.config.key_size).abs() > 0.1 {
                        return self.recreate_layer_surfaces();
                    }
                }
            }
            Message::ConfigChanged(config) => {
                log::info!("Config changed externally: enabled={}", config.enabled);
                let size_changed = (self.config.key_size - config.key_size).abs() > 0.1;
                let position_changed = self.config.position != config.position;
                
                self.config = config.clone();
                self.update_tray_state();
                
                // Update shared state
                if let Ok(mut state) = self.shared_state.lock() {
                    state.update_from_config(&config);
                }
                
                // Recreate surfaces if needed
                if size_changed || position_changed {
                    return self.recreate_layer_surfaces();
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
            let is_active = self.config.enabled;
            handle.update(move |tray| {
                tray.set_active(is_active);
            });
        }
    }

    fn handle_output_event(
        &mut self,
        event: OutputEvent,
        wl_output: WlOutput,
    ) -> Task<cosmic::Action<Message>> {
        match event {
            OutputEvent::Created(info_opt) => {
                let name = info_opt.and_then(|i| i.name);
                log::info!("Output created: {:?}", name);

                let (key_size, position) = self
                    .shared_state
                    .lock()
                    .map(|s| (s.key_size, s.position))
                    .unwrap_or((36.0, OverlayPosition::default()));

                let surface_id = window::Id::unique();
                let (task, width) =
                    create_layer_surface_for_output(&wl_output, surface_id, key_size, position);
                self.outputs.push(OutputState {
                    output: wl_output.clone(),
                    surface_id,
                    name,
                    width,
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

    fn recreate_layer_surfaces(&mut self) -> Task<cosmic::Action<Message>> {
        let mut tasks = Vec::new();

        let (key_size, position) = self
            .shared_state
            .lock()
            .map(|s| (s.key_size, s.position))
            .unwrap_or((self.config.key_size, self.config.position));

        // Destroy old surfaces and create new ones
        for output_state in &mut self.outputs {
            // Destroy old surface
            tasks.push(destroy_surface(output_state.surface_id));

            // Create new surface with new ID
            let new_id = window::Id::unique();
            output_state.surface_id = new_id;
            let (task, width) =
                create_layer_surface_for_output(&output_state.output, new_id, key_size, position);
            output_state.width = width;
            tasks.push(task);
        }

        cosmic::iced::Task::batch(tasks)
    }
}
