//! Input capture wrapper and event processing

use std::sync::{Arc, Mutex};
use std::thread;

use crate::capture::{Axis, ButtonState, InputCapture, InputEvent, KeyState, SwipeDirection};
use crate::keystroke::{KeyModifiers, Keystroke};
use crate::overlay::{push_history, SharedState};

/// Convert key code to display string
pub fn key_to_string(key: u32) -> Option<String> {
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

/// Check if a key code is a modifier
pub fn is_modifier(key: u32) -> bool {
    matches!(key, 29 | 97 | 56 | 100 | 42 | 54 | 125 | 126)
}

/// Start input capture in a background thread
pub fn spawn_input_capture(state: Arc<Mutex<SharedState>>) {
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
                        process_input_event(&state, event);
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
fn process_input_event(state: &Arc<Mutex<SharedState>>, event: InputEvent) {
    match event {
        InputEvent::Key {
            key,
            state: key_state,
        } => {
            if let Ok(mut s) = state.lock() {
                if !s.enabled {
                    return;
                }

                let is_mod = is_modifier(key);
                let key_str = key_to_string(key);

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
                        } else if let Some(key_str) = key_str {
                            // Non-modifier key pressed
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
                    let scroll_str = if value > 0.0 { "ScrollDown" } else { "ScrollUp" };

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
