//! Kiwi daemon - layer-shell overlay for keystroke visualization

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

use cosmic::cosmic_config::{self, CosmicConfigEntry};
use kiwi_common::{
    keystrokes_row, Config, KeyModifiers, Keystroke, OverlayPosition, APP_ID, DBUS_NAME, DBUS_PATH,
};
use kiwi_input::{InputCapture, InputEvent, KeyState};

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let settings = cosmic::app::Settings::default()
        .no_main_window(true)
        .exit_on_close(false);
    cosmic::app::run::<Kiwi>(settings, ())
}

/// Shared state between D-Bus service and app
#[derive(Debug)]
struct SharedState {
    enabled: bool,
    quit_requested: bool,
    /// Size of keystroke widgets
    key_size: f32,
    /// How long keystrokes stay visible (seconds)
    fade_duration: f32,
    /// Color palette
    palette: kiwi_common::PaletteType,
    /// Overlay position
    position: OverlayPosition,
    /// Current modifier state (live)
    modifiers: KeyModifiers,
    /// Peak modifiers held during current modifier session (for showing full combo on release)
    peak_modifiers: KeyModifiers,
    /// Currently pressed non-modifier key and the modifiers that were active when it was pressed
    current_key: Option<(String, KeyModifiers)>,
    /// History of completed keystrokes (released) - shown as not pressed
    history: Vec<Keystroke>,
    /// Track if a non-modifier key was pressed while modifiers were held
    /// (to know if we should show modifier-only tap on release)
    key_pressed_with_modifiers: bool,
    /// Currently pressed mouse button: (button_string, is_touchpad, press_time, has_moved)
    current_mouse: Option<(String, bool, std::time::Instant, bool)>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            enabled: true,
            quit_requested: false,
            key_size: 36.0,
            fade_duration: 5.0,
            palette: kiwi_common::PaletteType::default(),
            position: OverlayPosition::default(),
            modifiers: KeyModifiers::default(),
            peak_modifiers: KeyModifiers::default(),
            current_key: None,
            history: Vec::new(),
            key_pressed_with_modifiers: false,
            current_mouse: None,
        }
    }
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
    width: u32, // Window width for this output
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
    ConfigChanged(Config),
}

/// Calculate window height based on key size (key + count badge + padding)
fn window_height_for_key_size(key_size: f32) -> u32 {
    // key_size + spacing (5%) + count text (30%) + some padding
    let count_height = key_size * 0.35; // font size + line height
    let spacing = key_size * 0.05;
    let padding = 10.0;
    (key_size + spacing + count_height + padding).max(60.0) as u32
}

/// Calculate window width based on key_size (fits ~12 keys + gaps)
fn window_width_for_key_size(key_size: f32) -> u32 {
    // Enough space for about 12 single keys with gaps, or ~6 combos
    let key_gap = 4.0;
    let margin = 40.0;
    let num_keys = 12.0;
    let width = (key_size * num_keys) + (key_gap * (num_keys - 1.0)) + margin;
    width.clamp(400.0, 1600.0) as u32 // Clamp between 400-1600
}

fn create_layer_surface_for_output(
    output: &WlOutput,
    id: window::Id,
    key_size: f32,
    position: OverlayPosition,
) -> (cosmic::iced::Task<cosmic::Action<Message>>, u32) {
    let width = window_width_for_key_size(key_size);
    let height = window_height_for_key_size(key_size);

    // Determine anchor and margin based on position
    let (anchor, margin) = match position {
        OverlayPosition::TopLeft => (
            Anchor::TOP | Anchor::LEFT,
            cosmic::iced_runtime::platform_specific::wayland::layer_surface::IcedMargin {
                top: 20,
                left: 20,
                bottom: 0,
                right: 0,
            },
        ),
        OverlayPosition::TopRight => (
            Anchor::TOP | Anchor::RIGHT,
            cosmic::iced_runtime::platform_specific::wayland::layer_surface::IcedMargin {
                top: 20,
                right: 20,
                bottom: 0,
                left: 0,
            },
        ),
        OverlayPosition::BottomLeft => (
            Anchor::BOTTOM | Anchor::LEFT,
            cosmic::iced_runtime::platform_specific::wayland::layer_surface::IcedMargin {
                bottom: 20,
                left: 20,
                top: 0,
                right: 0,
            },
        ),
        OverlayPosition::BottomRight => (
            Anchor::BOTTOM | Anchor::RIGHT,
            cosmic::iced_runtime::platform_specific::wayland::layer_surface::IcedMargin {
                bottom: 20,
                right: 20,
                top: 0,
                left: 0,
            },
        ),
        OverlayPosition::BottomCenter => (
            Anchor::BOTTOM,
            cosmic::iced_runtime::platform_specific::wayland::layer_surface::IcedMargin {
                bottom: 20,
                left: 0,
                top: 0,
                right: 0,
            },
        ),
    };

    let task = get_layer_surface(SctkLayerSurfaceSettings {
        id,
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::None,
        // Empty input zone = click-through (no input accepted)
        input_zone: Some(vec![]),
        anchor,
        output: IcedOutput::Output(output.clone()),
        namespace: "kiwi".to_string(),
        size: Some((Some(width), Some(height))),
        margin,
        exclusive_zone: -1,
        size_limits: Limits::NONE.min_width(300.0).min_height(1.0),
    });
    (task, width)
}

/// Push a keystroke to history, limiting size
/// If the keystroke matches the last one within the threshold, increment its count instead
fn push_history(history: &mut Vec<Keystroke>, keystroke: Keystroke) {
    // Check if we can merge with the last keystroke
    if let Some(last) = history.last_mut() {
        if last.can_merge(&keystroke) {
            last.increment();
            return;
        }
    }

    // Otherwise, add as new
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
        16 => "Q",
        17 => "W",
        18 => "E",
        19 => "R",
        20 => "T",
        21 => "Y",
        22 => "U",
        23 => "I",
        24 => "O",
        25 => "P",
        26 => "[",
        27 => "]",
        28 => "↵",
        29 => "Ctrl",
        30 => "A",
        31 => "S",
        32 => "D",
        33 => "F",
        34 => "G",
        35 => "H",
        36 => "J",
        37 => "K",
        38 => "L",
        39 => ";",
        40 => "'",
        41 => "`",
        42 => "⇧",
        43 => "\\",
        44 => "Z",
        45 => "X",
        46 => "C",
        47 => "V",
        48 => "B",
        49 => "N",
        50 => "M",
        51 => ",",
        52 => ".",
        53 => "/",
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
        // Load config from cosmic-config
        let config = cosmic_config::Config::new(APP_ID, Config::VERSION)
            .ok()
            .and_then(|h| Config::get_entry(&h).ok())
            .unwrap_or_default();

        let state = Arc::new(Mutex::new(SharedState {
            enabled: config.enabled,
            quit_requested: false,
            key_size: config.key_size,
            fade_duration: config.fade_duration,
            palette: config.palette,
            position: config.position,
            modifiers: KeyModifiers::default(),
            peak_modifiers: KeyModifiers::default(),
            current_key: None,
            history: Vec::new(),
            key_pressed_with_modifiers: false,
            current_mouse: None,
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
                                InputEvent::Key {
                                    key,
                                    state: key_state,
                                } => {
                                    if let Ok(mut s) = input_state.lock() {
                                        if !s.enabled {
                                            continue;
                                        }

                                        let is_modifier =
                                            matches!(key, 29 | 97 | 56 | 100 | 42 | 54 | 125 | 126);
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
                                                    // Track peak modifiers (the full combo held together)
                                                    s.peak_modifiers.ctrl |= s.modifiers.ctrl;
                                                    s.peak_modifiers.alt |= s.modifiers.alt;
                                                    s.peak_modifiers.shift |= s.modifiers.shift;
                                                    s.peak_modifiers.super_key |=
                                                        s.modifiers.super_key;
                                                    // If no key is currently pressed, modifiers are shown as "pressed"
                                                    // (handled in view by building current keystroke from state)
                                                } else if let Some(key_str) = key_str {
                                                    // Non-modifier key pressed
                                                    // Mark that a key was pressed with modifiers
                                                    if s.modifiers.any() {
                                                        s.key_pressed_with_modifiers = true;
                                                    }
                                                    // If there was a previous key being held, release it to history
                                                    if let Some((prev_key, prev_mods)) =
                                                        s.current_key.take()
                                                    {
                                                        let completed = if prev_mods.any() {
                                                            Keystroke::combination(
                                                                &prev_mods, prev_key, false,
                                                            )
                                                        } else {
                                                            Keystroke::single(prev_key, false)
                                                        };
                                                        push_history(&mut s.history, completed);
                                                    }
                                                    // Set the new key as currently pressed, with current modifiers
                                                    s.current_key =
                                                        Some((key_str, s.modifiers.clone()));
                                                }
                                            }
                                            KeyState::Released => {
                                                if is_modifier {
                                                    // Update modifier state
                                                    match key {
                                                        29 | 97 => s.modifiers.ctrl = false,
                                                        56 | 100 => s.modifiers.alt = false,
                                                        42 | 54 => s.modifiers.shift = false,
                                                        125 | 126 => s.modifiers.super_key = false,
                                                        _ => {}
                                                    }

                                                    // Only add modifier tap to history when ALL modifiers are released
                                                    // (not when releasing one of multiple held modifiers)
                                                    if !s.modifiers.any() {
                                                        // No modifiers left - check if this was a standalone modifier tap
                                                        // Use peak_modifiers to get the full combo that was held
                                                        if !s.key_pressed_with_modifiers
                                                            && s.current_key.is_none()
                                                        {
                                                            if let Some(keystroke) =
                                                                Keystroke::from_modifiers(
                                                                    &s.peak_modifiers,
                                                                    false,
                                                                )
                                                            {
                                                                push_history(
                                                                    &mut s.history,
                                                                    keystroke,
                                                                );
                                                            }
                                                        }
                                                        // Reset tracking
                                                        s.key_pressed_with_modifiers = false;
                                                        s.peak_modifiers = KeyModifiers::default();
                                                    }
                                                } else if key_str.is_some() {
                                                    // Non-modifier key released
                                                    if let Some((current, key_mods)) =
                                                        s.current_key.take()
                                                    {
                                                        // Add the completed keystroke to history using the
                                                        // modifiers that were active when the key was PRESSED
                                                        let completed = if key_mods.any() {
                                                            Keystroke::combination(
                                                                &key_mods, current, false,
                                                            )
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
                                InputEvent::MouseButton {
                                    button,
                                    state: btn_state,
                                    is_touchpad,
                                } => {
                                    if let Ok(mut s) = input_state.lock() {
                                        if !s.enabled {
                                            continue;
                                        }

                                        let btn_str = match (button, is_touchpad) {
                                            (272, true) => "Tap",     // Touchpad 1-finger tap
                                            (273, true) => "2Tap",    // Touchpad 2-finger tap
                                            (274, true) => "3Tap",    // Touchpad 3-finger tap
                                            (272, false) => "LClick", // Mouse left click
                                            (273, false) => "RClick", // Mouse right click
                                            (274, false) => "MClick", // Mouse middle click
                                            _ => continue,
                                        };

                                        match btn_state {
                                            kiwi_input::ButtonState::Pressed => {
                                                // Track the pressed button with timestamp
                                                s.current_mouse = Some((
                                                    btn_str.to_string(),
                                                    is_touchpad,
                                                    std::time::Instant::now(),
                                                    false,
                                                ));
                                                // Mark that an action occurred with modifiers (prevents modifier-only tap on release)
                                                if s.modifiers.any() {
                                                    s.key_pressed_with_modifiers = true;
                                                }
                                            }
                                            kiwi_input::ButtonState::Released => {
                                                // Check if this was a drag (has_moved flag set by motion events)
                                                let was_drag = s
                                                    .current_mouse
                                                    .as_ref()
                                                    .map(|(_, _, _, has_moved)| *has_moved)
                                                    .unwrap_or(false);

                                                let final_str = if was_drag && btn_str == "LClick" {
                                                    "LDrag"
                                                } else if was_drag
                                                    && is_touchpad
                                                    && btn_str == "Tap"
                                                {
                                                    "TapDrag"
                                                } else {
                                                    btn_str
                                                };

                                                s.current_mouse = None;

                                                let keystroke = if s.modifiers.any() {
                                                    Keystroke::combination(
                                                        &s.modifiers,
                                                        final_str.to_string(),
                                                        false,
                                                    )
                                                } else {
                                                    Keystroke::single(final_str.to_string(), false)
                                                };
                                                push_history(&mut s.history, keystroke);
                                            }
                                        }
                                    }
                                }
                                InputEvent::MouseMotion { .. } => {
                                    // Mark current mouse button as dragging if we move while pressed
                                    if let Ok(mut s) = input_state.lock() {
                                        if let Some((_, _, _, ref mut has_moved)) = s.current_mouse
                                        {
                                            *has_moved = true;
                                        }
                                    }
                                }
                                InputEvent::MouseScroll { axis, value } => {
                                    if let Ok(mut s) = input_state.lock() {
                                        if !s.enabled {
                                            continue;
                                        }

                                        // Only handle vertical scroll, ignore tiny movements
                                        if axis == kiwi_input::Axis::Vertical && value.abs() > 10.0
                                        {
                                            let scroll_str = if value > 0.0 {
                                                "ScrollDown"
                                            } else {
                                                "ScrollUp"
                                            };

                                            let keystroke = if s.modifiers.any() {
                                                Keystroke::combination(
                                                    &s.modifiers,
                                                    scroll_str.to_string(),
                                                    false,
                                                )
                                            } else {
                                                Keystroke::single(scroll_str.to_string(), false)
                                            };
                                            push_history(&mut s.history, keystroke);
                                        }
                                    }
                                }
                                InputEvent::TouchpadScroll { axis, value } => {
                                    if let Ok(mut s) = input_state.lock() {
                                        if !s.enabled {
                                            continue;
                                        }

                                        // Input layer already accumulates and thresholds
                                        let scroll_str = match axis {
                                            kiwi_input::Axis::Vertical => {
                                                if value > 0.0 {
                                                    "2Down"
                                                } else {
                                                    "2Up"
                                                }
                                            }
                                            kiwi_input::Axis::Horizontal => {
                                                if value > 0.0 {
                                                    "2Right"
                                                } else {
                                                    "2Left"
                                                }
                                            }
                                        };

                                        let keystroke = if s.modifiers.any() {
                                            Keystroke::combination(
                                                &s.modifiers,
                                                scroll_str.to_string(),
                                                false,
                                            )
                                        } else {
                                            Keystroke::single(scroll_str.to_string(), false)
                                        };
                                        push_history(&mut s.history, keystroke);
                                    }
                                }
                                InputEvent::Swipe {
                                    finger_count,
                                    direction,
                                } => {
                                    if let Ok(mut s) = input_state.lock() {
                                        if !s.enabled {
                                            continue;
                                        }

                                        use kiwi_input::SwipeDirection;

                                        // Map finger count + direction to gesture name
                                        let gesture_str = match (finger_count, direction) {
                                            (3, SwipeDirection::Up) => "3Up",
                                            (3, SwipeDirection::Down) => "3Down",
                                            (4, SwipeDirection::Up) => "4Up",
                                            (4, SwipeDirection::Down) => "4Down",
                                            // Could add left/right if needed
                                            _ => continue,
                                        };

                                        let keystroke = if s.modifiers.any() {
                                            Keystroke::combination(
                                                &s.modifiers,
                                                gesture_str.to_string(),
                                                false,
                                            )
                                        } else {
                                            Keystroke::single(gesture_str.to_string(), false)
                                        };
                                        push_history(&mut s.history, keystroke);
                                    }
                                }
                                InputEvent::Hold { finger_count } => {
                                    if let Ok(mut s) = input_state.lock() {
                                        if !s.enabled {
                                            continue;
                                        }

                                        // Map finger count to tap name
                                        let tap_str = match finger_count {
                                            3 => "3Tap",
                                            4 => "4Tap",
                                            _ => continue,
                                        };

                                        let keystroke = if s.modifiers.any() {
                                            Keystroke::combination(
                                                &s.modifiers,
                                                tap_str.to_string(),
                                                false,
                                            )
                                        } else {
                                            Keystroke::single(tap_str.to_string(), false)
                                        };
                                        push_history(&mut s.history, keystroke);
                                    }
                                }
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
        if let Some(output) = self.outputs.iter().find(|o| o.surface_id == id) {
            let window_width = output.width as f32;

            let (keystrokes, key_size, fade_duration, palette, position) = self
                .state
                .lock()
                .map(|s| {
                    if !s.enabled {
                        return (
                            Vec::new(),
                            s.key_size,
                            s.fade_duration,
                            s.palette,
                            s.position,
                        );
                    }

                    let mut display: Vec<Keystroke> = s.history.clone();

                    // Build current "pressed" keystroke from state
                    // Priority: mouse action > key > modifiers-only
                    if let Some((ref btn_str, is_touchpad, _, has_moved)) = s.current_mouse {
                        // Mouse button is pressed - show it (with modifiers if any)
                        let display_str = if has_moved && btn_str == "LClick" {
                            "LDrag".to_string()
                        } else if has_moved && is_touchpad && btn_str == "Tap" {
                            "TapDrag".to_string()
                        } else {
                            btn_str.clone()
                        };

                        let mouse_keystroke = if s.modifiers.any() {
                            Keystroke::combination(&s.modifiers, display_str, true)
                        } else {
                            Keystroke::single(display_str, true)
                        };
                        display.push(mouse_keystroke);
                    } else if let Some((ref key, ref key_mods)) = s.current_key {
                        // Key + modifiers pressed (use modifiers from when key was pressed)
                        let current = if key_mods.any() {
                            Keystroke::combination(key_mods, key.clone(), true)
                        } else {
                            Keystroke::single(key.clone(), true)
                        };
                        display.push(current);
                    } else if s.modifiers.any() {
                        // Only modifiers pressed (no key, no mouse)
                        if let Some(mods_keystroke) = Keystroke::from_modifiers(&s.modifiers, true)
                        {
                            display.push(mods_keystroke);
                        }
                    }

                    (display, s.key_size, s.fade_duration, s.palette, s.position)
                })
                .unwrap_or((
                    Vec::new(),
                    36.0,
                    5.0,
                    kiwi_common::PaletteType::default(),
                    OverlayPosition::default(),
                ));

            if keystrokes.is_empty() {
                // Empty transparent container when no keystrokes
                cosmic::widget::container(cosmic::widget::text("")).into()
            } else {
                // Show keystrokes row with alignment based on position
                keystrokes_row(
                    &keystrokes,
                    key_size,
                    fade_duration,
                    palette,
                    window_width,
                    position,
                )
            }
        } else {
            cosmic::widget::text("").into()
        }
    }

    fn update(
        &mut self,
        message: Self::Message,
    ) -> cosmic::iced::Task<cosmic::Action<Self::Message>> {
        match message {
            Message::OutputEvent(event, wl_output) => match event {
                OutputEvent::Created(info_opt) => {
                    let name = info_opt.and_then(|i| i.name);
                    log::info!("Output created: {:?}", name);

                    let (key_size, position) = self
                        .state
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
                        return destroy_layer_surface(removed.surface_id);
                    }
                }
                OutputEvent::InfoUpdate(info) => {
                    if let Some(output_state) =
                        self.outputs.iter_mut().find(|o| o.output == wl_output)
                    {
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
                    let fade_duration = state.fade_duration;
                    state.history.retain(|k| !k.is_expired(fade_duration));
                }
            }
            Message::ConfigChanged(config) => {
                let (old_key_size, old_position) = self
                    .state
                    .lock()
                    .map(|s| (s.key_size, s.position))
                    .unwrap_or((36.0, OverlayPosition::default()));
                let size_changed = (old_key_size - config.key_size).abs() > 0.1;
                let position_changed = old_position != config.position;

                if let Ok(mut state) = self.state.lock() {
                    log::info!("Config changed: enabled={}, key_size={}, fade_duration={}, palette={:?}, position={:?}", 
                        config.enabled, config.key_size, config.fade_duration, config.palette, config.position);
                    state.enabled = config.enabled;
                    state.key_size = config.key_size;
                    state.fade_duration = config.fade_duration;
                    state.palette = config.palette;
                    state.position = config.position;
                }

                // Recreate layer surfaces if size or position changed
                if size_changed || position_changed {
                    let mut tasks = Vec::new();

                    // Destroy old surfaces and create new ones
                    for output_state in &mut self.outputs {
                        // Destroy old surface
                        tasks.push(destroy_layer_surface(output_state.surface_id));

                        // Create new surface with new ID
                        let new_id = window::Id::unique();
                        output_state.surface_id = new_id;
                        let (task, width) = create_layer_surface_for_output(
                            &output_state.output,
                            new_id,
                            config.key_size,
                            config.position,
                        );
                        output_state.width = width;
                        tasks.push(task);
                    }

                    return cosmic::iced::Task::batch(tasks);
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
            // Watch for config changes
            self.core()
                .watch_config::<Config>(APP_ID)
                .map(|update| Message::ConfigChanged(update.config)),
        ])
    }
}
