//! Layer shell overlay for keystroke visualization

use std::sync::{Arc, Mutex};

use cosmic::iced::{window, Limits};
use cosmic::iced_runtime::platform_specific::wayland::layer_surface::{
    IcedOutput, SctkLayerSurfaceSettings,
};
use cosmic::iced_winit::commands::layer_surface::{destroy_layer_surface, get_layer_surface};
use cosmic_client_toolkit::sctk::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use wayland_client::protocol::wl_output::WlOutput;

use crate::config::{IconStyle, OverlayPosition, PaletteType};
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
    /// Key display mode (typed character vs physical key)
    pub key_display_mode: crate::config::KeyDisplayMode,
    /// Icon style (symbols vs text)
    pub icon_style: IconStyle,
    /// Maximum number of keystroke widgets to show
    pub history_count: u8,
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
        key_display_mode: crate::config::KeyDisplayMode,
        icon_style: IconStyle,
        history_count: u8,
    ) -> Self {
        Self {
            enabled,
            key_size,
            fade_duration,
            palette,
            position,
            key_display_mode,
            icon_style,
            history_count,
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
        self.key_display_mode = config.key_display_mode;
        self.icon_style = config.icon_style;
        self.history_count = config.history_count;
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
            key_display_mode: crate::config::KeyDisplayMode::default(),
            icon_style: IconStyle::default(),
            history_count: 5,
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
}

/// Create a full-screen layer surface for an output
/// Positioning is handled via layout in view_overlay, not surface positioning
pub fn create_layer_surface_for_output(
    output: &WlOutput,
    id: window::Id,
) -> cosmic::iced::Task<cosmic::Action<Message>> {
    // Anchor to all edges = full screen
    let anchor = Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT;

    get_layer_surface(SctkLayerSurfaceSettings {
        id,
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::None,
        // Empty input zone = click-through (no input accepted)
        input_zone: Some(vec![]),
        anchor,
        output: IcedOutput::Output(output.clone()),
        namespace: "kiwi".to_string(),
        // None = compositor decides size (full screen when anchored to all edges)
        size: None,
        margin:
            cosmic::iced_runtime::platform_specific::wayland::layer_surface::IcedMargin::default(),
        exclusive_zone: -1,
        size_limits: Limits::NONE,
    })
}

/// Destroy a layer surface
pub fn destroy_surface(surface_id: window::Id) -> cosmic::iced::Task<cosmic::Action<Message>> {
    destroy_layer_surface(surface_id)
}

/// Render the overlay view (full-screen, with layout positioning)
pub fn view_overlay(state: &Arc<Mutex<SharedState>>) -> cosmic::Element<'static, Message> {
    let (keystrokes, key_size, fade_duration, palette, position, history_count, icon_style) = state
        .lock()
        .map(|s| {
            if !s.enabled {
                return (
                    Vec::new(),
                    s.key_size,
                    s.fade_duration,
                    s.palette,
                    s.position,
                    s.history_count,
                    s.icon_style,
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

            (
                display,
                s.key_size,
                s.fade_duration,
                s.palette,
                s.position,
                s.history_count,
                s.icon_style,
            )
        })
        .unwrap_or((
            Vec::new(),
            36.0,
            5.0,
            PaletteType::default(),
            OverlayPosition::default(),
            5,
            IconStyle::default(),
        ));

    // Determine vertical and horizontal alignment based on position
    let (v_align, h_align) = match position {
        OverlayPosition::TopLeft => (
            cosmic::iced::alignment::Vertical::Top,
            cosmic::iced::alignment::Horizontal::Left,
        ),
        OverlayPosition::TopRight => (
            cosmic::iced::alignment::Vertical::Top,
            cosmic::iced::alignment::Horizontal::Right,
        ),
        OverlayPosition::BottomLeft => (
            cosmic::iced::alignment::Vertical::Bottom,
            cosmic::iced::alignment::Horizontal::Left,
        ),
        OverlayPosition::BottomRight => (
            cosmic::iced::alignment::Vertical::Bottom,
            cosmic::iced::alignment::Horizontal::Right,
        ),
        OverlayPosition::BottomCenter => (
            cosmic::iced::alignment::Vertical::Bottom,
            // Right edge at center - we'll handle this specially
            cosmic::iced::alignment::Horizontal::Center,
        ),
    };

    let content: cosmic::Element<'static, Message> = if keystrokes.is_empty() {
        // Empty widget when no keystrokes
        cosmic::widget::Space::new(0, 0).into()
    } else {
        // Show keystrokes row
        keystrokes_row(
            &keystrokes,
            key_size,
            fade_duration,
            palette,
            position,
            history_count as usize,
            icon_style,
        )
    };

    // For BottomCenter: newest key at center, older keys grow to the left
    // Use a row with two halves: [left half with content aligned right] [right half empty spacer]
    // This puts the rightmost key at screen center
    let positioned_content: cosmic::Element<'static, Message> =
        if position == OverlayPosition::BottomCenter {
            cosmic::widget::row()
                // Left half: content aligned to the right edge (screen center)
                .push(
                    cosmic::widget::container(content)
                        .width(cosmic::iced::Length::FillPortion(1))
                        .align_x(cosmic::iced::alignment::Horizontal::Right),
                )
                // Right half: empty spacer (takes up right 50% of screen)
                .push(cosmic::widget::Space::with_width(
                    cosmic::iced::Length::FillPortion(1),
                ))
                .into()
        } else {
            content
        };

    // Full-screen container with proper alignment
    cosmic::widget::container(positioned_content)
        .width(cosmic::iced::Length::Fill)
        .height(cosmic::iced::Length::Fill)
        .align_x(h_align)
        .align_y(v_align)
        .padding(20) // Margin from edges
        .into()
}
