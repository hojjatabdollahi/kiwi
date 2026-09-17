//! Layer shell overlay for keystroke visualization

use std::sync::{Arc, Mutex};

use cosmic::iced::platform_specific::runtime::wayland::layer_surface::{
    IcedOutput, SctkLayerSurfaceSettings,
};
use cosmic::iced::platform_specific::shell::commands::layer_surface::destroy_layer_surface;
use cosmic::iced::{window, Limits};
use cosmic::surface::action::{app_layer_shell, LiveSettings};
use cosmic_client_toolkit::sctk::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use wayland_client::protocol::wl_output::WlOutput;

use crate::config::{IconStyle, OverlayPosition, Palette, PaletteType};
use crate::keystroke::{keystrokes_row, KeyModifiers, Keystroke};
use crate::{KiwiApp, Message};

/// Maximum number of keystrokes in history
pub const MAX_HISTORY: usize = 10;

/// How long a lifted touch point keeps fading out (seconds)
const TOUCH_FADE_DURATION: f32 = 0.25;

/// A single touchscreen contact, positioned in normalized 0.0..1.0 coordinates
/// so the overlay can scale it to the surface it is drawn on.
#[derive(Debug, Clone)]
pub struct TouchPoint {
    /// libinput seat slot - identifies the finger while it stays down
    pub slot: u32,
    pub x: f32,
    pub y: f32,
    /// Set when the finger was lifted; the marker fades out from then on
    pub released: Option<std::time::Instant>,
}

impl TouchPoint {
    /// 0.0 while the finger is down, growing to 1.0 over the fade-out
    fn fade_progress(&self) -> f32 {
        match self.released {
            None => 0.0,
            Some(at) => (at.elapsed().as_secs_f32() / TOUCH_FADE_DURATION).min(1.0),
        }
    }

    fn is_expired(&self) -> bool {
        self.released
            .is_some_and(|at| at.elapsed().as_secs_f32() >= TOUCH_FADE_DURATION)
    }
}

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
    /// Show keyboard input
    pub show_keyboard: bool,
    /// Show mouse input
    pub show_mouse: bool,
    /// Show touchpad gestures
    pub show_gestures: bool,
    /// Show touchscreen contacts
    pub show_touch: bool,
    /// Live touchscreen contacts (plus the ones currently fading out)
    pub touches: Vec<TouchPoint>,
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
        show_keyboard: bool,
        show_mouse: bool,
        show_gestures: bool,
        show_touch: bool,
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
            show_keyboard,
            show_mouse,
            show_gestures,
            show_touch,
            touches: Vec::new(),
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
        self.show_keyboard = config.show_keyboard;
        self.show_mouse = config.show_mouse;
        self.show_gestures = config.show_gestures;
        self.show_touch = config.show_touch;
    }

    /// Clean up expired keystrokes
    pub fn cleanup_expired(&mut self) {
        let fade_duration = self.fade_duration;
        self.history.retain(|k| !k.is_expired(fade_duration));
        self.touches.retain(|t| !t.is_expired());
    }

    /// True while something touch-related still needs to be animated
    pub fn has_touches(&self) -> bool {
        !self.touches.is_empty()
    }

    /// Record a new contact (or restart one that reuses a slot still fading out)
    pub fn touch_down(&mut self, slot: u32, x: f32, y: f32) {
        self.touches.retain(|t| t.slot != slot);
        self.touches.push(TouchPoint {
            slot,
            x,
            y,
            released: None,
        });
    }

    /// Move a live contact. Ignores slots that are already fading out.
    pub fn touch_motion(&mut self, slot: u32, x: f32, y: f32) {
        if let Some(touch) = self
            .touches
            .iter_mut()
            .find(|t| t.slot == slot && t.released.is_none())
        {
            touch.x = x;
            touch.y = y;
        }
    }

    /// Start the fade-out for a lifted contact
    pub fn touch_up(&mut self, slot: u32) {
        if let Some(touch) = self
            .touches
            .iter_mut()
            .find(|t| t.slot == slot && t.released.is_none())
        {
            touch.released = Some(std::time::Instant::now());
        }
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            enabled: true,
            key_size: 64.0,
            fade_duration: 5.0,
            palette: PaletteType::Frosted,
            position: OverlayPosition::TopRight,
            key_display_mode: crate::config::KeyDisplayMode::default(),
            icon_style: IconStyle::default(),
            history_count: 5,
            show_keyboard: true,
            show_mouse: true,
            show_gestures: true,
            show_touch: true,
            touches: Vec::new(),
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

/// Build the layer surface settings for a full-screen overlay on `output`.
fn layer_surface_settings(output: &WlOutput, id: window::Id) -> SctkLayerSurfaceSettings {
    // Anchor to all edges = full screen
    let anchor = Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT;

    SctkLayerSurfaceSettings {
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
            cosmic::iced::platform_specific::runtime::wayland::layer_surface::IcedMargin::default(),
        exclusive_zone: -1,
        size_limits: Limits::NONE,
    }
}

/// Create a full-screen layer surface for an output.
/// Positioning is handled via layout in view_overlay, not surface positioning.
///
/// The surface is created through libcosmic's surface API (rather than the raw
/// `get_layer_surface` command) so it is tracked by libcosmic with a per-surface
/// blur override. Without `blur: Some(false)`, libcosmic auto-enables background
/// blur on layer surfaces whenever the active COSMIC theme has the "frosted glass"
/// effect on, which would frost the entire transparent, full-screen overlay and blur
/// everything behind it. The override keeps the overlay transparent either way while
/// leaving the settings window's normal frosted behavior untouched.
pub fn create_layer_surface_for_output(
    output: &WlOutput,
    id: window::Id,
) -> cosmic::iced::Task<cosmic::Action<Message>> {
    let output = output.clone();
    let action = app_layer_shell::<KiwiApp>(
        |_app| LiveSettings {
            blur: Some(false),
            ..Default::default()
        },
        move |_app| layer_surface_settings(&output, id),
        Some(Box::new(move |app: &KiwiApp| {
            view_overlay(&app.shared_state, app.is_touch_surface(id)).map(cosmic::Action::App)
        })),
    );

    cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::Surface(action)))
}

/// Destroy a layer surface
pub fn destroy_surface(surface_id: window::Id) -> cosmic::iced::Task<cosmic::Action<Message>> {
    destroy_layer_surface(surface_id)
}

/// Paints a marker at every touchscreen contact.
///
/// Touch points are stored normalized, so they are scaled to the canvas bounds
/// here - the canvas fills the whole layer surface, which covers the output.
#[derive(Debug)]
struct TouchCanvas {
    touches: Vec<TouchPoint>,
    palette: PaletteType,
    /// Marker size follows the same size slider as the keystroke widgets
    key_size: f32,
}

impl cosmic::widget::canvas::Program<Message, cosmic::Theme> for TouchCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &cosmic::Renderer,
        _theme: &cosmic::Theme,
        bounds: cosmic::iced::Rectangle,
        _cursor: cosmic::iced::mouse::Cursor,
    ) -> Vec<cosmic::widget::canvas::Geometry> {
        use cosmic::widget::canvas::{Frame, Path, Stroke};

        let mut frame = Frame::new(renderer, bounds.size());
        let palette = Palette::from_type(self.palette);

        for touch in &self.touches {
            // Lifted fingers fade out while their ring expands slightly
            let progress = touch.fade_progress();
            let opacity = 1.0 - progress;
            let center = cosmic::iced::Point::new(touch.x * bounds.width, touch.y * bounds.height);
            let ring_radius = self.key_size * 0.5 * (1.0 + progress * 0.3);
            let dot_radius = self.key_size * 0.28;

            frame.fill(
                &Path::circle(center, dot_radius),
                with_opacity(palette.bg_pressed, opacity),
            );
            frame.stroke(
                &Path::circle(center, ring_radius),
                Stroke::default()
                    .with_color(with_opacity(palette.text, 0.85 * opacity))
                    .with_width(3.0),
            );
        }

        vec![frame.into_geometry()]
    }
}

/// Scale a color's alpha channel
fn with_opacity(color: cosmic::iced::Color, opacity: f32) -> cosmic::iced::Color {
    cosmic::iced::Color {
        a: color.a * opacity,
        ..color
    }
}

/// Render the overlay view (full-screen, with layout positioning).
///
/// `show_touches` is only set for the surface on the output touch input is
/// attributed to, so contacts are drawn once rather than on every display.
pub fn view_overlay(
    state: &Arc<Mutex<SharedState>>,
    show_touches: bool,
) -> cosmic::Element<'static, Message> {
    let (
        keystrokes,
        key_size,
        fade_duration,
        palette,
        position,
        history_count,
        icon_style,
        touches,
    ) = state
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
                    Vec::new(),
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

            let touches = if s.show_touch && show_touches {
                s.touches.clone()
            } else {
                Vec::new()
            };

            (
                display,
                s.key_size,
                s.fade_duration,
                s.palette,
                s.position,
                s.history_count,
                s.icon_style,
                touches,
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
            Vec::new(),
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
        cosmic::widget::Space::new().into()
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
            cosmic::widget::Row::new()
                // Left half: content aligned to the right edge (screen center)
                .push(
                    cosmic::widget::container(content)
                        .width(cosmic::iced::Length::FillPortion(1))
                        .align_x(cosmic::iced::alignment::Horizontal::Right),
                )
                // Right half: empty spacer (takes up right 50% of screen)
                .push(cosmic::widget::Space::new().width(cosmic::iced::Length::FillPortion(1)))
                .into()
        } else {
            content
        };

    // Full-screen container with proper alignment
    let keystroke_layer: cosmic::Element<'static, Message> =
        cosmic::widget::container(positioned_content)
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .align_x(h_align)
            .align_y(v_align)
            .padding(20) // Margin from edges
            .into();

    if touches.is_empty() {
        return keystroke_layer;
    }

    // Touch markers go behind the keystroke row, covering the whole surface
    let touch_layer = cosmic::widget::Canvas::new(TouchCanvas {
        touches,
        palette,
        key_size,
    })
    .width(cosmic::iced::Length::Fill)
    .height(cosmic::iced::Length::Fill);

    cosmic::iced::widget::stack![touch_layer, keystroke_layer].into()
}
