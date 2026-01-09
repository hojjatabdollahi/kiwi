//! Input capture wrapper and event processing

use std::sync::{Arc, Mutex};
use std::thread;

use evdev::KeyCode;
use xkbcommon::xkb;

use crate::capture::{Axis, ButtonState, InputCapture, InputEvent, KeyState, SwipeDirection};
use crate::config::KeyDisplayMode;
use crate::keystroke::{KeyModifiers, Keystroke};
use crate::overlay::{push_history, SharedState};

/// XKB state wrapper for character lookup
struct XkbState {
    state: xkb::State,
}

impl XkbState {
    /// Create a new XKB state using the current keyboard layout
    fn new() -> Option<Self> {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let keymap = xkb::Keymap::new_from_names(
            &context,
            "",   // rules (empty = default)
            "",   // model (empty = default)
            "",   // layout (empty = current)
            "",   // variant (empty = default)
            None, // options
            xkb::COMPILE_NO_FLAGS,
        )?;
        let state = xkb::State::new(&keymap);
        Some(Self { state })
    }

    /// Get the typed character for a key press with current modifiers
    /// Returns None if the key doesn't produce a character
    fn get_utf8(&self, evdev_keycode: u32) -> Option<String> {
        // evdev keycodes are offset by 8 from XKB keycodes
        let xkb_keycode: xkb::Keycode = (evdev_keycode + 8).into();
        let utf8 = self.state.key_get_utf8(xkb_keycode);

        // Filter out control characters and empty strings
        if utf8.is_empty() || utf8.chars().all(|c| c.is_control()) {
            None
        } else {
            Some(utf8)
        }
    }

    /// Update modifier state based on key press/release
    fn update_key(&mut self, evdev_keycode: u32, is_pressed: bool) {
        let xkb_keycode: xkb::Keycode = (evdev_keycode + 8).into();
        let direction = if is_pressed {
            xkb::KeyDirection::Down
        } else {
            xkb::KeyDirection::Up
        };
        self.state.update_key(xkb_keycode, direction);
    }
}

/// Convert key code to display string using evdev
pub fn key_to_string(key: u32) -> Option<String> {
    let evdev_key = KeyCode::new(key as u16);

    // Special keys with custom symbols/names
    match evdev_key {
        // Modifiers
        KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL => return Some("Ctrl".to_string()),
        KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT => return Some("Alt".to_string()),
        KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => return Some("⇧".to_string()),
        KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA => return Some("Super".to_string()),

        // Special keys with symbols
        KeyCode::KEY_ENTER | KeyCode::KEY_KPENTER => return Some("↵".to_string()),
        KeyCode::KEY_BACKSPACE => return Some("⌫".to_string()),
        KeyCode::KEY_SPACE => return Some("␣".to_string()),
        KeyCode::KEY_TAB => return Some("Tab".to_string()),
        KeyCode::KEY_ESC => return Some("Esc".to_string()),
        KeyCode::KEY_CAPSLOCK => return Some("Caps".to_string()),

        // Arrow keys
        KeyCode::KEY_UP => return Some("↑".to_string()),
        KeyCode::KEY_DOWN => return Some("↓".to_string()),
        KeyCode::KEY_LEFT => return Some("←".to_string()),
        KeyCode::KEY_RIGHT => return Some("→".to_string()),

        // Navigation
        KeyCode::KEY_HOME => return Some("Home".to_string()),
        KeyCode::KEY_END => return Some("End".to_string()),
        KeyCode::KEY_PAGEUP => return Some("PgUp".to_string()),
        KeyCode::KEY_PAGEDOWN => return Some("PgDn".to_string()),
        KeyCode::KEY_INSERT => return Some("Ins".to_string()),
        KeyCode::KEY_DELETE => return Some("Del".to_string()),

        _ => {}
    }

    // For everything else, use the evdev name and prettify it
    let name = format!("{:?}", evdev_key);
    prettify_key_name(&name)
}

/// Prettify an evdev key name like "KEY_A" to "A", "KEY_F1" to "F1", etc.
fn prettify_key_name(name: &str) -> Option<String> {
    // Unknown keys
    if name.starts_with("unknown") {
        return None;
    }

    // Strip "KEY_" prefix
    let stripped = if name.starts_with("KEY_") {
        &name[4..]
    } else {
        name
    };

    // Handle keypad keys
    if stripped.starts_with("KP") {
        let kp_key = &stripped[2..];
        return Some(format!("KP{}", kp_key));
    }

    // Single letter/number keys - return as-is
    if stripped.len() == 1 {
        return Some(stripped.to_string());
    }

    // Function keys (F1-F12)
    if stripped.starts_with('F') && stripped[1..].parse::<u32>().is_ok() {
        return Some(stripped.to_string());
    }

    // Number keys (0-9 are KEY_0 through KEY_9 but also KEY_1 = 2, etc.)
    if stripped.parse::<u32>().is_ok() {
        return Some(stripped.to_string());
    }

    // Common symbols - return the symbol character
    match stripped {
        "MINUS" => return Some("-".to_string()),
        "EQUAL" => return Some("=".to_string()),
        "LEFTBRACE" => return Some("[".to_string()),
        "RIGHTBRACE" => return Some("]".to_string()),
        "SEMICOLON" => return Some(";".to_string()),
        "APOSTROPHE" => return Some("'".to_string()),
        "GRAVE" => return Some("`".to_string()),
        "BACKSLASH" => return Some("\\".to_string()),
        "COMMA" => return Some(",".to_string()),
        "DOT" => return Some(".".to_string()),
        "SLASH" => return Some("/".to_string()),
        "KPASTERISK" => return Some("*".to_string()),
        "KPPLUS" => return Some("+".to_string()),
        "KPMINUS" => return Some("-".to_string()),
        "KPSLASH" => return Some("/".to_string()),
        "KPDOT" => return Some(".".to_string()),

        // These are handled above but just in case
        "SPACE" => return Some("␣".to_string()),
        "ENTER" => return Some("↵".to_string()),

        // For other named keys, title-case them
        _ => {
            // Convert SCROLLLOCK to ScrollLock, NUMLOCK to NumLock, etc.
            let result: String = stripped
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i == 0 {
                        c.to_ascii_uppercase()
                    } else {
                        c.to_ascii_lowercase()
                    }
                })
                .collect();
            Some(result)
        }
    }
}

/// Check if a key code is a modifier
pub fn is_modifier(key: u32) -> bool {
    let evdev_key = KeyCode::new(key as u16);
    matches!(
        evdev_key,
        KeyCode::KEY_LEFTCTRL
            | KeyCode::KEY_RIGHTCTRL
            | KeyCode::KEY_LEFTALT
            | KeyCode::KEY_RIGHTALT
            | KeyCode::KEY_LEFTSHIFT
            | KeyCode::KEY_RIGHTSHIFT
            | KeyCode::KEY_LEFTMETA
            | KeyCode::KEY_RIGHTMETA
    )
}

/// Start input capture in a background thread
pub fn spawn_input_capture(
    state: Arc<Mutex<SharedState>>,
    tray_tx: crossbeam_channel::Sender<crate::tray::TrayAction>,
) {
    thread::spawn(move || {
        match InputCapture::new() {
            Ok(mut capture) => {
                log::info!("Input capture started");

                // Initialize XKB state for character lookup
                let mut xkb_state = XkbState::new();
                if xkb_state.is_none() {
                    log::warn!(
                        "Failed to initialize XKB state, falling back to physical key names"
                    );
                }

                loop {
                    if let Err(e) = capture.dispatch() {
                        log::error!("Input dispatch error: {}", e);
                        break;
                    }

                    for event in capture.events() {
                        process_input_event(&state, event, &mut xkb_state, &tray_tx);
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
}

/// Process a single input event
fn process_input_event(
    state: &Arc<Mutex<SharedState>>,
    event: InputEvent,
    xkb_state: &mut Option<XkbState>,
    tray_tx: &crossbeam_channel::Sender<crate::tray::TrayAction>,
) {
    match event {
        InputEvent::Key {
            key,
            state: key_state,
        } => {
            // Get display mode and enabled state first
            let (enabled, display_mode) = {
                if let Ok(s) = state.lock() {
                    (s.enabled, s.key_display_mode)
                } else {
                    return;
                }
            };

            if !enabled {
                // Still update XKB state even when disabled to keep modifier state in sync
                if let Some(ref mut xkb) = xkb_state {
                    xkb.update_key(key, key_state == KeyState::Pressed);
                }
                return;
            }

            let is_mod = is_modifier(key);

            // Get the key string based on display mode
            let key_str = if is_mod {
                // Modifiers always use physical key names
                key_to_string(key)
            } else {
                match display_mode {
                    KeyDisplayMode::TypedCharacter => {
                        // Try to get the typed character from XKB
                        xkb_state
                            .as_ref()
                            .and_then(|xkb| xkb.get_utf8(key))
                            .or_else(|| key_to_string(key)) // Fallback to physical key
                    }
                    KeyDisplayMode::PhysicalKey => key_to_string(key),
                }
            };

            // Update XKB state for modifier tracking
            if let Some(ref mut xkb) = xkb_state {
                xkb.update_key(key, key_state == KeyState::Pressed);
            }

            if let Ok(mut s) = state.lock() {
                match key_state {
                    KeyState::Pressed => {
                        if is_mod {
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
                            s.peak_modifiers.super_key |= s.modifiers.super_key;
                        } else {
                            // Check for deactivation shortcut: Super+Shift+S
                            // KEY_S = 31 in evdev
                            if key == 31 && s.modifiers.super_key && s.modifiers.shift && !s.modifiers.ctrl && !s.modifiers.alt {
                                log::info!("Deactivation shortcut detected: Super+Shift+S");
                                // Clear current state to avoid showing the shortcut
                                s.current_key = None;
                                s.peak_modifiers = KeyModifiers::default();
                                s.key_pressed_with_modifiers = false;
                                // Drop the lock before sending to avoid potential deadlock
                                drop(s);
                                // Send toggle action to main app (will deactivate since we're currently active)
                                if let Err(e) = tray_tx.send(crate::tray::TrayAction::ToggleActive) {
                                    log::error!("Failed to send deactivation action: {}", e);
                                }
                                return;
                            }
                            
                            // Non-modifier key pressed
                            if let Some(key_str) = key_str {
                                // Mark that a key was pressed with modifiers
                                if s.modifiers.any() {
                                    s.key_pressed_with_modifiers = true;
                                }
                                // If there was a previous key being held, release it to history
                                if let Some((prev_key, prev_mods)) = s.current_key.take() {
                                    let completed = if prev_mods.any() {
                                        Keystroke::combination(&prev_mods, prev_key, false)
                                    } else {
                                        Keystroke::single(prev_key, false)
                                    };
                                    push_history(&mut s.history, completed);
                                }
                                // Set the new key as currently pressed, with current modifiers
                                s.current_key = Some((key_str, s.modifiers.clone()));
                            }
                        }
                    }
                    KeyState::Released => {
                        if is_mod {
                            // Update modifier state
                            match key {
                                29 | 97 => s.modifiers.ctrl = false,
                                56 | 100 => s.modifiers.alt = false,
                                42 | 54 => s.modifiers.shift = false,
                                125 | 126 => s.modifiers.super_key = false,
                                _ => {}
                            }

                            // Only add modifier tap to history when ALL modifiers are released
                            if !s.modifiers.any() {
                                // No modifiers left - check if this was a standalone modifier tap
                                if !s.key_pressed_with_modifiers && s.current_key.is_none() {
                                    if let Some(keystroke) =
                                        Keystroke::from_modifiers(&s.peak_modifiers, false)
                                    {
                                        push_history(&mut s.history, keystroke);
                                    }
                                }
                                // Reset tracking
                                s.key_pressed_with_modifiers = false;
                                s.peak_modifiers = KeyModifiers::default();
                            }
                        } else if key_str.is_some() {
                            // Non-modifier key released
                            if let Some((current, key_mods)) = s.current_key.take() {
                                // Add the completed keystroke to history
                                let completed = if key_mods.any() {
                                    Keystroke::combination(&key_mods, current, false)
                                } else {
                                    Keystroke::single(current, false)
                                };
                                push_history(&mut s.history, completed);
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
            if let Ok(mut s) = state.lock() {
                if !s.enabled {
                    return;
                }

                let btn_str = match (button, is_touchpad) {
                    (272, true) => "Tap",     // Touchpad 1-finger tap
                    (273, true) => "2Tap",    // Touchpad 2-finger tap
                    (274, true) => "3Tap",    // Touchpad 3-finger tap
                    (272, false) => "LClick", // Mouse left click
                    (273, false) => "RClick", // Mouse right click
                    (274, false) => "MClick", // Mouse middle click
                    _ => return,
                };

                match btn_state {
                    ButtonState::Pressed => {
                        // Track the pressed button with timestamp
                        s.current_mouse = Some((
                            btn_str.to_string(),
                            is_touchpad,
                            std::time::Instant::now(),
                            false,
                        ));
                        // Mark that an action occurred with modifiers
                        if s.modifiers.any() {
                            s.key_pressed_with_modifiers = true;
                        }
                    }
                    ButtonState::Released => {
                        // Check if this was a drag
                        let was_drag = s
                            .current_mouse
                            .as_ref()
                            .map(|(_, _, _, has_moved)| *has_moved)
                            .unwrap_or(false);

                        let final_str = if was_drag && btn_str == "LClick" {
                            "LDrag"
                        } else if was_drag && is_touchpad && btn_str == "Tap" {
                            "TapDrag"
                        } else {
                            btn_str
                        };

                        s.current_mouse = None;

                        let keystroke = if s.modifiers.any() {
                            Keystroke::combination(&s.modifiers, final_str.to_string(), false)
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
            if let Ok(mut s) = state.lock() {
                if let Some((_, _, _, ref mut has_moved)) = s.current_mouse {
                    *has_moved = true;
                }
            }
        }
        InputEvent::MouseScroll { axis, value } => {
            if let Ok(mut s) = state.lock() {
                if !s.enabled {
                    return;
                }

                // Only handle vertical scroll, ignore tiny movements
                if axis == Axis::Vertical && value.abs() > 10.0 {
                    let scroll_str = if value > 0.0 {
                        "ScrollDown"
                    } else {
                        "ScrollUp"
                    };

                    let keystroke = if s.modifiers.any() {
                        Keystroke::combination(&s.modifiers, scroll_str.to_string(), false)
                    } else {
                        Keystroke::single(scroll_str.to_string(), false)
                    };
                    push_history(&mut s.history, keystroke);
                }
            }
        }
        InputEvent::TouchpadScroll { axis, value } => {
            if let Ok(mut s) = state.lock() {
                if !s.enabled {
                    return;
                }

                // Input layer already accumulates and thresholds
                let scroll_str = match axis {
                    Axis::Vertical => {
                        if value > 0.0 {
                            "2Down"
                        } else {
                            "2Up"
                        }
                    }
                    Axis::Horizontal => {
                        if value > 0.0 {
                            "2Right"
                        } else {
                            "2Left"
                        }
                    }
                };

                let keystroke = if s.modifiers.any() {
                    Keystroke::combination(&s.modifiers, scroll_str.to_string(), false)
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
            if let Ok(mut s) = state.lock() {
                if !s.enabled {
                    return;
                }

                // Map finger count + direction to gesture name
                let gesture_str = match (finger_count, direction) {
                    (3, SwipeDirection::Up) => "3Up",
                    (3, SwipeDirection::Down) => "3Down",
                    (4, SwipeDirection::Up) => "4Up",
                    (4, SwipeDirection::Down) => "4Down",
                    _ => return,
                };

                let keystroke = if s.modifiers.any() {
                    Keystroke::combination(&s.modifiers, gesture_str.to_string(), false)
                } else {
                    Keystroke::single(gesture_str.to_string(), false)
                };
                push_history(&mut s.history, keystroke);
            }
        }
        InputEvent::Hold { finger_count } => {
            if let Ok(mut s) = state.lock() {
                if !s.enabled {
                    return;
                }

                // Map finger count to tap name
                let tap_str = match finger_count {
                    3 => "3Tap",
                    4 => "4Tap",
                    _ => return,
                };

                let keystroke = if s.modifiers.any() {
                    Keystroke::combination(&s.modifiers, tap_str.to_string(), false)
                } else {
                    Keystroke::single(tap_str.to_string(), false)
                };
                push_history(&mut s.history, keystroke);
            }
        }
    }
}
