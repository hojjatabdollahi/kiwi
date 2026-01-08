//! Layer shell overlay for keystroke visualization

use std::sync::{Arc, Mutex};

use cosmic::iced::{window, Limits};
use cosmic::iced_runtime::platform_specific::wayland::layer_surface::{
    IcedOutput, SctkLayerSurfaceSettings,
};
use cosmic::iced_winit::commands::layer_surface::{destroy_layer_surface, get_layer_surface};
use cosmic_client_toolkit::sctk::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use wayland_client::protocol::wl_output::WlOutput;

use crate::config::{OverlayPosition, PaletteType};
use crate::keystroke::{keystrokes_row, KeyModifiers, Keystroke};
use crate::Message;

/// Maximum number of keystrokes in history
pub const MAX_HISTORY: usize = 10;

/// Shared state for keystroke visualization
#[derive(Debug)]
pub struct SharedState {
    pub enabled: bool,
    /// Size of keystroke widgets
    pub key_size: f32,
    /// How long keystrokes stay visible (seconds)
    pub fade_duration: f32,
    /// Color palette
    pub palette: PaletteType,
    /// Overlay position
    pub position: OverlayPosition,
    /// Current modifier state (live)
    pub modifiers: KeyModifiers,
    /// Peak modifiers held during current modifier session (for showing full combo on release)
    pub peak_modifiers: KeyModifiers,
    /// Currently pressed non-modifier key and the modifiers that were active when it was pressed
    pub current_key: Option<(String, KeyModifiers)>,
    /// History of completed keystrokes (released) - shown as not pressed
    pub history: Vec<Keystroke>,
    /// Track if a non-modifier key was pressed while modifiers were held
    /// (to know if we should show modifier-only tap on release)
    pub key_pressed_with_modifiers: bool,
    /// Currently pressed mouse button: (button_string, is_touchpad, press_time, has_moved)
    pub current_mouse: Option<(String, bool, std::time::Instant, bool)>,
}

impl SharedState {
    pub fn new(
        enabled: bool,
        key_size: f32,
        fade_duration: f32,
        palette: PaletteType,
        position: OverlayPosition,
    ) -> Self {
        Self {
            enabled,
            key_size,
            fade_duration,
            palette,
            position,
            modifiers: KeyModifiers::default(),
            peak_modifiers: KeyModifiers::default(),
            current_key: None,
            history: Vec::new(),
            key_pressed_with_modifiers: false,
            current_mouse: None,
        }
    }

    /// Update state from config
    pub fn update_from_config(&mut self, config: &crate::config::Config) {
        self.enabled = config.enabled;
        self.key_size = config.key_size;
        self.fade_duration = config.fade_duration;
        self.palette = config.palette;
        self.position = config.position;
    }

    /// Clean up expired keystrokes
    pub fn cleanup_expired(&mut self) {
        let fade_duration = self.fade_duration;
        self.history.retain(|k| !k.is_expired(fade_duration));
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            enabled: true,
            key_size: 36.0,
            fade_duration: 5.0,
            palette: PaletteType::default(),
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

/// Push a keystroke to history, limiting size
/// If the keystroke matches the last one within the threshold, increment its count instead
pub fn push_history(history: &mut Vec<Keystroke>, keystroke: Keystroke) {
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

/// Tracks an output and its associated layer surface
#[derive(Debug, Clone)]
pub struct OutputState {
    pub output: WlOutput,
    pub surface_id: window::Id,
    pub name: Option<String>,
    pub width: u32, // Window width for this output
}

/// Calculate window height based on key size (key + count badge + padding)
pub fn window_height_for_key_size(key_size: f32) -> u32 {
    // key_size + spacing (5%) + count text (30%) + some padding
    let count_height = key_size * 0.35; // font size + line height
    let spacing = key_size * 0.05;
    let padding = 10.0;
    (key_size + spacing + count_height + padding).max(60.0) as u32
}

/// Calculate window width based on key_size (fits ~12 keys + gaps)
pub fn window_width_for_key_size(key_size: f32) -> u32 {
    // Enough space for about 12 single keys with gaps, or ~6 combos
    let key_gap = 4.0;
    let margin = 40.0;
    let num_keys = 12.0;
    let width = (key_size * num_keys) + (key_gap * (num_keys - 1.0)) + margin;
    width.clamp(400.0, 1600.0) as u32 // Clamp between 400-1600
}

/// Create a layer surface for an output
pub fn create_layer_surface_for_output(
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

/// Destroy a layer surface
pub fn destroy_surface(surface_id: window::Id) -> cosmic::iced::Task<cosmic::Action<Message>> {
    destroy_layer_surface(surface_id)
}

/// Render the overlay view for a specific window
pub fn view_overlay(
    state: &Arc<Mutex<SharedState>>,
    window_width: f32,
) -> cosmic::Element<'static, Message> {
    let (keystrokes, key_size, fade_duration, palette, position) = state
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
                if let Some(mods_keystroke) = Keystroke::from_modifiers(&s.modifiers, true) {
                    display.push(mods_keystroke);
                }
            }

            (display, s.key_size, s.fade_duration, s.palette, s.position)
        })
        .unwrap_or((
            Vec::new(),
            36.0,
            5.0,
            PaletteType::default(),
            OverlayPosition::default(),
        ));

    if keystrokes.is_empty() {
        // Empty transparent container when no keystrokes
        cosmic::widget::container(cosmic::widget::text("")).into()
    } else {
        // Show keystrokes row with alignment based on position
        keystrokes_row(&keystrokes, key_size, fade_duration, palette, window_width, position)
    }
}
