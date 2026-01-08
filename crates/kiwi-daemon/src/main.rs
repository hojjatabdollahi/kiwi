//! Kiwi daemon - layer-shell overlay for keystroke visualization

mod ui;

use std::sync::{Arc, Mutex};
use std::thread;

use cosmic::app::Core;
use cosmic::iced::{window, Limits};
use cosmic::iced_core::event::wayland::OutputEvent;
use cosmic::iced_futures::event::listen_with;
use cosmic::iced_futures::Subscription;
use cosmic::iced_runtime::platform_specific::wayland::layer_surface::{
    IcedOutput, SctkLayerSurfaceSettings,
};
use cosmic::iced_winit::commands::layer_surface::{destroy_layer_surface, get_layer_surface};
use cosmic_client_toolkit::sctk::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use wayland_client::protocol::wl_output::WlOutput;

use kiwi_common::{DBUS_NAME, DBUS_PATH};
use kiwi_input::{InputCapture, InputEvent, KeyState};
use ui::{KeyModifiers, Keystroke};

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let settings = cosmic::app::Settings::default()
        .no_main_window(true)
        .exit_on_close(false);
    cosmic::app::run::<Kiwi>(settings, ())
}

/// Shared state between D-Bus service and app
#[derive(Debug, Default)]
struct SharedState {
    enabled: bool,
    quit_requested: bool,
    /// Current modifier state
    modifiers: KeyModifiers,
    /// Currently pressed non-modifier key (if any)
    current_key: Option<String>,
    /// History of completed keystrokes (released) - shown as not pressed
    history: Vec<Keystroke>,
    /// Track if a non-modifier key was pressed while modifiers were held
    /// (to know if we should show modifier-only tap on release)
    key_pressed_with_modifiers: bool,
}

const MAX_HISTORY: usize = 10;

/// D-Bus service implementation
struct KiwiDbus {
    state: Arc<Mutex<SharedState>>,
}

#[zbus::interface(name = "dev.hojjat.Kiwi")]
impl KiwiDbus {
    fn set_enabled(&self, enabled: bool) {
        log::info!("D-Bus: set_enabled({})", enabled);
        if let Ok(mut state) = self.state.lock() {
            state.enabled = enabled;
        }
    }

    fn is_enabled(&self) -> bool {
        self.state.lock().map(|s| s.enabled).unwrap_or(false)
    }

    fn quit(&self) {
        log::info!("D-Bus: quit requested");
        if let Ok(mut state) = self.state.lock() {
            state.quit_requested = true;
        }
    }
}

/// Tracks an output and its associated layer surface
#[derive(Debug, Clone)]
struct OutputState {
    output: WlOutput,
    surface_id: window::Id,
    name: Option<String>,
}

struct Kiwi {
    core: Core,
    outputs: Vec<OutputState>,
    state: Arc<Mutex<SharedState>>,
}

#[derive(Debug, Clone)]
enum Message {
    OutputEvent(OutputEvent, WlOutput),
    Tick,
}

fn create_layer_surface_for_output(
    output: &WlOutput,
    id: window::Id,
) -> cosmic::iced::Task<cosmic::Action<Message>> {
    get_layer_surface(SctkLayerSurfaceSettings {
        id,
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::None,
        // Empty input zone = click-through (no input accepted)
        input_zone: Some(vec![]),
        anchor: Anchor::TOP | Anchor::RIGHT,
        output: IcedOutput::Output(output.clone()),
        namespace: "kiwi".to_string(),
        size: Some((Some(400), Some(80))),
        margin: cosmic::iced_runtime::platform_specific::wayland::layer_surface::IcedMargin {
            top: 20,
            right: 20,
            bottom: 0,
            left: 0,
        },
        exclusive_zone: -1,
        size_limits: Limits::NONE.min_width(1.0).min_height(1.0),
        ..Default::default()
    })
}

/// Push a keystroke to history, limiting size
fn push_history(history: &mut Vec<Keystroke>, keystroke: Keystroke) {
    if history.len() >= MAX_HISTORY {
        history.remove(0);
    }
    history.push(keystroke);
}

/// Convert key code to display string
fn key_to_string(key: u32) -> Option<String> {
    // Common key codes (Linux input event codes)
    let s = match key {
        1 => "Esc",
        2..=10 => return Some(format!("{}", key - 1)),
        11 => "0",
        12 => "-",
        13 => "=",
        14 => "⌫",
        15 => "Tab",
        16 => "Q", 17 => "W", 18 => "E", 19 => "R", 20 => "T",
        21 => "Y", 22 => "U", 23 => "I", 24 => "O", 25 => "P",
        26 => "[", 27 => "]",
        28 => "↵",
        29 => "Ctrl",
        30 => "A", 31 => "S", 32 => "D", 33 => "F", 34 => "G",
        35 => "H", 36 => "J", 37 => "K", 38 => "L",
        39 => ";", 40 => "'", 41 => "`",
        42 => "⇧",
        43 => "\\",
        44 => "Z", 45 => "X", 46 => "C", 47 => "V", 48 => "B",
        49 => "N", 50 => "M",
        51 => ",", 52 => ".", 53 => "/",
        54 => "⇧",
        55 => "*",
        56 => "Alt",
        57 => "␣",
        58 => "Caps",
        59..=68 => return Some(format!("F{}", key - 58)),
        87 => "F11",
        88 => "F12",
        96 => "↵",
        97 => "Ctrl",
        100 => "Alt",
        102 => "Home",
        103 => "↑",
        104 => "PgUp",
        105 => "←",
        106 => "→",
        107 => "End",
        108 => "↓",
        109 => "PgDn",
        110 => "Ins",
        111 => "Del",
        125 | 126 => "Super",
        _ => return None,
    };
    Some(s.to_string())
}

impl cosmic::Application for Kiwi {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.hojjat.kiwi.daemon";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(
        core: Core,
        _flags: Self::Flags,
    ) -> (Self, cosmic::iced::Task<cosmic::Action<Self::Message>>) {
        let state = Arc::new(Mutex::new(SharedState {
            enabled: true,
            quit_requested: false,
            modifiers: KeyModifiers::default(),
            current_key: None,
            history: Vec::new(),
            key_pressed_with_modifiers: false,
        }));

        // Start D-Bus service in background
        let dbus_state = state.clone();
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let connection = zbus::Connection::session().await.unwrap();
                let service = KiwiDbus { state: dbus_state };
                
                connection
                    .object_server()
                    .at(DBUS_PATH, service)
                    .await
                    .unwrap();
                
                connection.request_name(DBUS_NAME).await.unwrap();
                log::info!("D-Bus service registered: {}", DBUS_NAME);
                
                // Keep the connection alive
                std::future::pending::<()>().await;
            });
        });

        // Start input capture in background
        let input_state = state.clone();
        thread::spawn(move || {
            match InputCapture::new() {
                Ok(mut capture) => {
                    log::info!("Input capture started");
                    loop {
                        if let Err(e) = capture.dispatch() {
                            log::error!("Input dispatch error: {}", e);
                            break;
                        }
                        
                        for event in capture.events() {
                            match event {
                                InputEvent::Key { key, state: key_state } => {
                                    if let Ok(mut s) = input_state.lock() {
                                        if !s.enabled {
                                            continue;
                                        }

                                        let is_modifier = matches!(key, 29 | 97 | 56 | 100 | 42 | 54 | 125 | 126);
                                        let key_str = key_to_string(key);

                                        match key_state {
                                            KeyState::Pressed => {
                                                if is_modifier {
                                                    // Update modifier state
                                                    match key {
                                                        29 | 97 => s.modifiers.ctrl = true,
                                                        56 | 100 => s.modifiers.alt = true,
                                                        42 | 54 => s.modifiers.shift = true,
                                                        125 | 126 => s.modifiers.super_key = true,
                                                        _ => {}
                                                    }
                                                    // If no key is currently pressed, modifiers are shown as "pressed"
                                                    // (handled in view by building current keystroke from state)
                                                } else if let Some(key_str) = key_str {
                                                    // Non-modifier key pressed
                                                    // Mark that a key was pressed with modifiers
                                                    if s.modifiers.any() {
                                                        s.key_pressed_with_modifiers = true;
                                                    }
                                                    // If there was a previous key being held, release it to history
                                                    if let Some(prev_key) = s.current_key.take() {
                                                        let completed = if s.modifiers.any() {
                                                            Keystroke::combination(&s.modifiers, prev_key, false)
                                                        } else {
                                                            Keystroke::single(prev_key, false)
                                                        };
                                                        push_history(&mut s.history, completed);
                                                    }
                                                    // Set the new key as currently pressed
                                                    s.current_key = Some(key_str);
                                                }
                                            }
                                            KeyState::Released => {
                                                if is_modifier {
                                                    // Capture modifiers before this one is released
                                                    let mods_before = s.modifiers.clone();
                                                    
                                                    // Update modifier state
                                                    match key {
                                                        29 | 97 => s.modifiers.ctrl = false,
                                                        56 | 100 => s.modifiers.alt = false,
                                                        42 | 54 => s.modifiers.shift = false,
                                                        125 | 126 => s.modifiers.super_key = false,
                                                        _ => {}
                                                    }
                                                    
                                                    // If no key was pressed while modifier was held,
                                                    // and no other key is currently pressed,
                                                    // add the modifier tap to history
                                                    if !s.key_pressed_with_modifiers && s.current_key.is_none() {
                                                        if let Some(keystroke) = Keystroke::from_modifiers(&mods_before, false) {
                                                            push_history(&mut s.history, keystroke);
                                                        }
                                                    }
                                                    
                                                    // Reset tracking when all modifiers are released
                                                    if !s.modifiers.any() {
                                                        s.key_pressed_with_modifiers = false;
                                                    }
                                                } else if key_str.is_some() {
                                                    // Non-modifier key released
                                                    if let Some(current) = s.current_key.take() {
                                                        // Add the completed keystroke to history
                                                        let completed = if s.modifiers.any() {
                                                            Keystroke::combination(&s.modifiers, current, false)
                                                        } else {
                                                            Keystroke::single(current, false)
                                                        };
                                                        push_history(&mut s.history, completed);
                                                        
                                                        // If modifiers are still held, they become the new "pressed" state
                                                        // (no current_key, but modifiers shown as pressed in view)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        
                        // Small sleep to prevent busy-waiting
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
                Err(e) => {
                    log::error!("Failed to start input capture: {}", e);
                }
            }
        });

        let app = Kiwi {
            core,
            outputs: Vec::new(),
            state,
        };
        
        (app, cosmic::iced::Task::none())
    }

    fn view(&self) -> cosmic::Element<'_, Self::Message> {
        cosmic::widget::text("").into()
    }

    fn view_window(&self, id: window::Id) -> cosmic::Element<'_, Self::Message> {
        if self.outputs.iter().any(|o| o.surface_id == id) {
            let keystrokes = self.state
                .lock()
                .map(|s| {
                    if !s.enabled {
                        return Vec::new();
                    }

                    let mut display: Vec<Keystroke> = s.history.clone();

                    // Build current "pressed" keystroke from state
                    if let Some(ref key) = s.current_key {
                        // Key + modifiers pressed
                        let current = if s.modifiers.any() {
                            Keystroke::combination(&s.modifiers, key.clone(), true)
                        } else {
                            Keystroke::single(key.clone(), true)
                        };
                        display.push(current);
                    } else if s.modifiers.any() {
                        // Only modifiers pressed (no key)
                        if let Some(mods_keystroke) = Keystroke::from_modifiers(&s.modifiers, true) {
                            display.push(mods_keystroke);
                        }
                    }

                    display
                })
                .unwrap_or_default();
            
            if keystrokes.is_empty() {
                // Empty transparent container when no keystrokes
                cosmic::widget::container(cosmic::widget::text(""))
                    .into()
            } else {
                // Show keystrokes row with clipping (spacer inside pushes to right)
                // Width::Fill needed for spacer to expand, but row items keep natural size
                ui::keystrokes_row(&keystrokes)
            }
        } else {
            cosmic::widget::text("").into()
        }
    }

    fn update(&mut self, message: Self::Message) -> cosmic::iced::Task<cosmic::Action<Self::Message>> {
        match message {
            Message::OutputEvent(event, wl_output) => match event {
                OutputEvent::Created(info_opt) => {
                    let name = info_opt.and_then(|i| i.name);
                    log::info!("Output created: {:?}", name);
                    
                    let surface_id = window::Id::unique();
                    self.outputs.push(OutputState {
                        output: wl_output.clone(),
                        surface_id,
                        name,
                    });
                    
                    return create_layer_surface_for_output(&wl_output, surface_id);
                }
                OutputEvent::Removed => {
                    if let Some(idx) = self.outputs.iter().position(|o| o.output == wl_output) {
                        let removed = self.outputs.remove(idx);
                        log::info!("Output removed: {:?}", removed.name);
                        return destroy_layer_surface(removed.surface_id);
                    }
                }
                OutputEvent::InfoUpdate(info) => {
                    if let Some(output_state) = self.outputs.iter_mut().find(|o| o.output == wl_output) {
                        output_state.name = info.name;
                    }
                }
            },
            Message::Tick => {
                if let Ok(mut state) = self.state.lock() {
                    // Check if quit was requested
                    if state.quit_requested {
                        std::process::exit(0);
                    }
                    // Clean up expired keystrokes from history
                    state.history.retain(|k| !k.is_expired());
                }
            }
        }
        cosmic::iced::Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        use cosmic::iced::time;
        
        Subscription::batch([
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
            // Periodic tick to update display and check quit
            time::every(std::time::Duration::from_millis(50)).map(|_| Message::Tick),
        ])
    }
}
