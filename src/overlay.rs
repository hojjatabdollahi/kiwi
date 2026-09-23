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

use crate::config::{IconStyle, OverlayPosition};
use crate::keystroke::{keystrokes_row, KeyModifiers, Keystroke};
use crate::theme::{Layout, Theme, ThemeChoice};
use crate::{KiwiApp, Message};

/// Maximum number of keystrokes in history (the typewriter line needs more than a few)
pub const MAX_HISTORY: usize = 64;

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
    /// Which theme is in use, to notice when the config switches it
    pub theme_choice: ThemeChoice,
    /// The loaded theme (shared with every view that draws keys)
    pub theme: Arc<Theme>,
    /// Overlay position
    pub position: OverlayPosition,
    /// Distance between the keys and the screen edge
    pub margin: f32,
    /// Typewriter line width, overriding the theme's
    pub line_width: Option<f32>,
    /// Arrange mode: the overlay takes input and shows sample keys, snap spots and a toolbar
    pub arranging: bool,
    /// A theme preview playing on the overlay
    pub preview: Option<Preview>,
    /// How far the keys have been dragged from their spot, while arranging
    pub drag_offset: Option<cosmic::iced::Vector>,
    /// Last frame: how many finished keystrokes were showing, whether a held
    /// keystroke was showing after them, and how many keys that held one had
    row_len: usize,
    was_held: bool,
    held_parts: usize,
    /// When a new keystroke last appeared at the edge, which starts the slide
    shifted_at: Option<std::time::Instant>,
    /// When the held combination grew, and how many keys it had before
    slot_grew: Option<(std::time::Instant, usize)>,
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
    /// Show drawing tablet input
    pub show_tablet: bool,
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
    /// Currently pressed mouse/touchpad/pen button: (button_string, press_time, has_moved)
    pub current_mouse: Option<(String, std::time::Instant, bool)>,
}

impl SharedState {
    /// Update state from config
    pub fn update_from_config(&mut self, config: &crate::config::Config) {
        self.enabled = config.enabled;
        self.key_size = config.key_size;
        self.fade_duration = config.fade_duration;
        let theme_choice = ThemeChoice::from_config(config);
        if theme_choice != self.theme_choice {
            self.theme = Arc::new(theme_choice.load(&crate::theme::themes_dir()));
            self.theme_choice = theme_choice;
        }
        self.position = config.position;
        self.margin = config.margin;
        self.line_width = config.line_width;
        self.key_display_mode = config.key_display_mode;
        self.icon_style = config.icon_style;
        self.history_count = config.history_count;
        self.show_keyboard = config.show_keyboard;
        self.show_mouse = config.show_mouse;
        self.show_gestures = config.show_gestures;
        self.show_touch = config.show_touch;
        self.show_tablet = config.show_tablet;
    }

    /// Clean up expired keystrokes
    pub fn cleanup_expired(&mut self) {
        let fade_duration = self.fade_duration;
        if self
            .preview
            .as_ref()
            .is_some_and(|p| p.is_over(fade_duration))
        {
            self.preview = None;
        }
        self.history.retain(|k| !k.is_expired(fade_duration));
        self.touches.retain(|t| !t.is_expired());
    }

    /// True while something touch-related still needs to be animated
    pub fn has_touches(&self) -> bool {
        !self.touches.is_empty()
    }

    /// True while keys are sliding: one just left the slot, or one is shrinking
    /// away as it expires. The overlay then needs frequent redraws.
    pub fn is_sliding(&self) -> bool {
        use crate::keystroke::{closing_secs, SLIDE_SECS, SLOT_GROW_SECS};
        let expiring = |k: &Keystroke| {
            let left = self.fade_duration - k.age_secs();
            !k.pressed && (0.0..closing_secs(k)).contains(&left)
        };
        self.shifted_at
            .is_some_and(|at| at.elapsed().as_secs_f32() < SLIDE_SECS)
            || self
                .slot_grew
                .is_some_and(|(at, _)| at.elapsed().as_secs_f32() < SLOT_GROW_SECS)
            || self.history.iter().any(expiring)
            || self.preview.is_some()
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
            theme_choice: ThemeChoice::Builtin(crate::config::BuiltinTheme::Frosted),
            theme: Arc::new(Theme::default()),
            position: OverlayPosition::TopRight,
            margin: 20.0,
            line_width: None,
            arranging: false,
            preview: None,
            drag_offset: None,
            row_len: 0,
            was_held: false,
            held_parts: 0,
            shifted_at: None,
            slot_grew: None,
            key_display_mode: crate::config::KeyDisplayMode::default(),
            icon_style: IconStyle::default(),
            history_count: 5,
            show_keyboard: true,
            show_mouse: true,
            show_gestures: true,
            show_touch: true,
            show_tablet: true,
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

    cosmic::surface::surface_task(action)
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
    style: crate::theme::KeyStyle,
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
        let style = self.style;

        for touch in &self.touches {
            // Lifted fingers fade out while their ring expands slightly
            let progress = touch.fade_progress();
            let opacity = 1.0 - progress;
            let center = cosmic::iced::Point::new(touch.x * bounds.width, touch.y * bounds.height);
            let ring_radius = self.key_size * 0.5 * (1.0 + progress * 0.3);
            let dot_radius = self.key_size * 0.28;

            frame.fill(
                &Path::circle(center, dot_radius),
                with_opacity(style.pressed.0, opacity),
            );
            frame.stroke(
                &Path::circle(center, ring_radius),
                Stroke::default()
                    .with_color(with_opacity(style.text.0, 0.85 * opacity))
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

/// What one frame of the overlay needs, copied out of the shared state
struct Snapshot {
    keystrokes: Vec<Keystroke>,
    key_size: f32,
    fade_duration: f32,
    theme: Arc<Theme>,
    position: OverlayPosition,
    margin: f32,
    line_width: Option<f32>,
    history_count: u8,
    icon_style: IconStyle,
    touches: Vec<TouchPoint>,
    arranging: bool,
    drag_offset: cosmic::iced::Vector,
    motion: crate::keystroke::Motion,
}

impl Snapshot {
    fn take(s: &mut SharedState, show_touches: bool) -> Self {
        let theme = match &s.preview {
            Some(preview) if !s.arranging => preview.theme.clone(),
            _ => s.theme.clone(),
        };
        let (keystrokes, held) = if s.arranging {
            (sample_keystrokes(s.theme.layout), false)
        } else if let Some(preview) = &s.preview {
            (preview.keystrokes(), false)
        } else if s.enabled {
            live_keystrokes(s)
        } else {
            (Vec::new(), false)
        };

        // A new keystroke at the edge starts the slide: a new press (or a new key
        // pressed while the last is still held), or a finished keystroke that
        // arrived without being held first (a scroll, a gesture). Releasing a held
        // key doesn't: it stays right where it was, now finished.
        let fade_duration = s.fade_duration;
        let visible = keystrokes
            .iter()
            .filter(|k| !k.is_expired(fade_duration))
            .count();
        let row_len = visible - usize::from(held && visible > 0);
        let held_parts = if held {
            keystrokes.last().map_or(0, |k| k.keys.len())
        } else {
            0
        };
        let now = std::time::Instant::now();
        let new_press = held && (!s.was_held || row_len > s.row_len);
        let arrived = !held && !s.was_held && row_len > s.row_len;
        if new_press || arrived {
            s.shifted_at = Some(now);
            // A fresh keystroke appears as it is, without growing
            s.slot_grew = None;
        } else if held && s.was_held && held_parts > s.held_parts {
            // The held combination gained a key (Ctrl, then Ctrl + Shift…)
            s.slot_grew = Some((now, s.held_parts));
        }
        s.row_len = row_len;
        s.was_held = held;
        s.held_parts = held_parts;

        let touches = if s.enabled && s.show_touch && show_touches {
            s.touches.clone()
        } else {
            Vec::new()
        };
        Self {
            keystrokes,
            key_size: s.key_size,
            fade_duration: s.fade_duration,
            theme,
            position: s.position,
            margin: s.margin,
            line_width: s.line_width,
            history_count: s.history_count,
            icon_style: s.icon_style,
            touches,
            arranging: s.arranging,
            drag_offset: s.drag_offset.unwrap_or_default(),
            motion: crate::keystroke::Motion {
                shifted_at: s.shifted_at,
                slot_grew: s.slot_grew,
            },
        }
    }
}

/// The history plus whatever is held down right now, and whether the last
/// keystroke is a held one (rather than a finished one from the history)
fn live_keystrokes(s: &SharedState) -> (Vec<Keystroke>, bool) {
    let mut display: Vec<Keystroke> = s.history.clone();
    let history_len = display.len();

    // Build current "pressed" keystroke from state
    // Priority: mouse action > key > modifiers-only
    if let Some((ref btn_str, _, has_moved)) = s.current_mouse {
        // Mouse button is pressed - show it (with modifiers if any)
        let display_str = if has_moved {
            crate::keystroke::drag_variant(btn_str).unwrap_or(btn_str)
        } else {
            btn_str.as_str()
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
        // Pressing the same key again quickly counts up on the last keystroke (it
        // merges on release) instead of adding another
        match display.last_mut() {
            Some(last) if last.can_merge(&current) => {
                last.pressed = true;
                last.count += 1;
            }
            _ => display.push(current),
        }
    } else if s.modifiers.any() && !s.key_pressed_with_modifiers {
        // Only modifiers pressed (no key, no mouse). Once a key was pressed with
        // them, the finished combination stays in the slot instead.
        //
        // Show every modifier held so far, so letting go of one (Ctrl, while Shift
        // is still down) leaves its place empty instead of shifting the rest
        if let Some(mut held) = Keystroke::from_modifiers(&s.peak_modifiers, true) {
            held.released_parts = released_modifiers(&s.peak_modifiers, &s.modifiers);
            display.push(held);
        }
    }
    let held = display.len() > history_len;
    (display, held)
}

/// A theme preview: a short scripted bit of typing played on the overlay
#[derive(Debug)]
pub struct Preview {
    pub theme: Arc<Theme>,
    pub started: std::time::Instant,
}

/// The preview script: when each keystroke happens (ms from the start) and its keys
const PREVIEW_SCRIPT: &[(u64, &[&str], u32)] = &[
    (0, &["⇧", "H"], 1),
    (160, &["i"], 1),
    (320, &["␣"], 1),
    (480, &["⇧", "K"], 1),
    (640, &["i"], 1),
    (800, &["w"], 1),
    (960, &["i"], 1),
    (1500, &["Ctrl", "C"], 1),
    (2100, &["LClick"], 1),
    (2700, &["⌫"], 3),
    (3300, &["↵"], 1),
];

/// How long a key looks pressed in the preview
const PREVIEW_PRESS_MS: u64 = 120;

impl Preview {
    pub fn new(theme: Arc<Theme>) -> Self {
        Self {
            theme,
            started: std::time::Instant::now(),
        }
    }

    /// The keystrokes played so far, timestamped so they fade like real ones
    fn keystrokes(&self) -> Vec<Keystroke> {
        let elapsed = self.started.elapsed().as_millis() as u64;
        PREVIEW_SCRIPT
            .iter()
            .filter(|(at, ..)| *at <= elapsed)
            .map(|(at, keys, count)| Keystroke {
                keys: keys.iter().map(|k| k.to_string()).collect(),
                pressed: elapsed - at < PREVIEW_PRESS_MS,
                timestamp: self.started + std::time::Duration::from_millis(*at),
                count: *count,
                released_parts: 0,
            })
            .collect()
    }

    /// Done once the last keystroke has faded out
    fn is_over(&self, fade_duration: f32) -> bool {
        let last = PREVIEW_SCRIPT.last().map_or(0, |(at, ..)| *at);
        self.started.elapsed().as_secs_f32() > last as f32 / 1000.0 + fade_duration
    }
}

/// Which of the `peak` modifiers are no longer held, one bit per key in the order
/// `KeyModifiers::to_parts` lists them
fn released_modifiers(peak: &KeyModifiers, held: &KeyModifiers) -> u8 {
    let pairs = [
        (peak.super_key, held.super_key),
        (peak.ctrl, held.ctrl),
        (peak.alt, held.alt),
        (peak.shift, held.shift),
    ];
    let mut mask = 0;
    let mut part = 0;
    for (was_held, still_held) in pairs {
        if was_held {
            if !still_held {
                mask |= 1 << part;
            }
            part += 1;
        }
    }
    mask
}

/// What arrange mode shows in place of real input
fn sample_keystrokes(layout: Layout) -> Vec<Keystroke> {
    let ctrl = KeyModifiers {
        ctrl: true,
        ..Default::default()
    };
    let key = |k: &str| Keystroke::single(k, false);
    match layout {
        Layout::Keys => vec![
            key("V"),
            Keystroke::combination(&ctrl, "C", false),
            key("A"),
        ],
        Layout::Text => "git push"
            .chars()
            .map(|c| key(&c.to_string()))
            .chain([Keystroke::combination(&ctrl, "S", false)])
            .collect(),
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
    use cosmic::iced::alignment::{Horizontal, Vertical};

    let Ok(frame) = state
        .lock()
        .map(|mut s| Snapshot::take(&mut s, show_touches))
    else {
        return cosmic::widget::Space::new().into();
    };

    let (v_align, h_align) = match frame.position {
        OverlayPosition::TopLeft => (Vertical::Top, Horizontal::Left),
        OverlayPosition::TopCenter => (Vertical::Top, Horizontal::Center),
        OverlayPosition::TopRight => (Vertical::Top, Horizontal::Right),
        OverlayPosition::BottomLeft => (Vertical::Bottom, Horizontal::Left),
        OverlayPosition::BottomCenter => (Vertical::Bottom, Horizontal::Center),
        OverlayPosition::BottomRight => (Vertical::Bottom, Horizontal::Right),
    };

    let content: cosmic::Element<'static, Message> = if frame.keystrokes.is_empty() {
        cosmic::widget::Space::new().into()
    } else {
        keystrokes_row(
            &frame.keystrokes,
            frame.key_size,
            frame.fade_duration,
            &frame.theme,
            frame.line_width.unwrap_or(frame.theme.line_width),
            frame.position,
            frame.history_count as usize,
            frame.icon_style,
            frame.motion,
        )
    };

    let content = if frame.arranging {
        arranging_keys(content, v_align, h_align)
    } else {
        content
    };

    // While dragging, the padding on the aligned side follows the pointer so
    // the keys move with it
    let margin = frame.margin;
    let offset = frame.drag_offset;
    let (top, bottom) = match v_align {
        Vertical::Top => (margin + offset.y, margin),
        _ => (margin, margin - offset.y),
    };
    let (left, right) = match h_align {
        Horizontal::Left => (margin + offset.x, margin),
        Horizontal::Right => (margin, margin - offset.x),
        // Centered: the horizontal offset is handled by `centered` below
        _ => (0.0, 0.0),
    };
    let content = if h_align == Horizontal::Center {
        centered(content, offset.x)
    } else {
        content
    };
    let keystroke_layer: cosmic::Element<'static, Message> = cosmic::widget::container(content)
        .width(cosmic::iced::Length::Fill)
        .height(cosmic::iced::Length::Fill)
        .align_x(h_align)
        .align_y(v_align)
        .padding(cosmic::iced::Padding {
            top: top.max(0.0),
            right: right.max(0.0),
            bottom: bottom.max(0.0),
            left: left.max(0.0),
        })
        .into();

    if frame.arranging {
        let spots = cosmic::widget::Canvas::new(ArrangeCanvas {
            position: frame.position,
            margin: frame.margin,
            key_size: frame.key_size,
        })
        .width(cosmic::iced::Length::Fill)
        .height(cosmic::iced::Length::Fill);
        let toolbar = cosmic::widget::container(arrange_toolbar(&frame))
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);
        return cosmic::iced::widget::stack![spots, keystroke_layer, toolbar].into();
    }

    if frame.touches.is_empty() {
        return keystroke_layer;
    }

    // Touch markers go behind the keystroke row, covering the whole surface
    let touch_layer = cosmic::widget::Canvas::new(TouchCanvas {
        touches: frame.touches,
        style: frame.theme.key,
        key_size: frame.key_size,
    })
    .width(cosmic::iced::Length::Fill)
    .height(cosmic::iced::Length::Fill);

    cosmic::iced::widget::stack![touch_layer, keystroke_layer].into()
}

/// Center positions: the keys end at the middle of the screen and grow leftward
/// only, so nothing shifts in both directions as keys come and go. `dx` moves
/// them sideways while they're dragged.
fn centered(
    content: cosmic::Element<'static, Message>,
    dx: f32,
) -> cosmic::Element<'static, Message> {
    use cosmic::iced::Length;
    use cosmic::widget;

    // Two halves split the space that's left; fixed space before them (or
    // between them) moves the middle right (or left) by half its width
    let shift = widget::Space::new().width(Length::Fixed(2.0 * dx.abs()));
    let left_half = widget::container(content)
        .width(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Right);
    let right_half = widget::Space::new().width(Length::Fill);
    let row = widget::Row::new();
    let row = if dx >= 0.0 {
        row.push(shift).push(left_half).push(right_half)
    } else {
        row.push(left_half).push(shift).push(right_half)
    };
    row.width(Length::Fill).into()
}

/// Let a surface take pointer and keyboard input (for arrange mode), or make it click-through again
pub fn set_interactive(
    surface_id: window::Id,
    interactive: bool,
) -> cosmic::iced::Task<cosmic::Action<Message>> {
    use cosmic::iced::platform_specific::runtime::{self as platform, wayland};
    use cosmic::iced::platform_specific::shell::commands::layer_surface::set_keyboard_interactivity;
    use cosmic::iced::runtime::{task, Action};

    let (zone, keyboard) = if interactive {
        // No input zone = the whole surface takes input; keyboard focus lets Esc work
        (None, KeyboardInteractivity::Exclusive)
    } else {
        (Some(vec![]), KeyboardInteractivity::None)
    };
    // iced has the input zone action but no helper function for it
    let set_input_zone = task::effect(Action::PlatformSpecific(platform::Action::Wayland(
        wayland::Action::LayerSurface(wayland::layer_surface::Action::InputZone {
            id: surface_id,
            zone,
        }),
    )));
    cosmic::iced::Task::batch([
        set_input_zone,
        set_keyboard_interactivity(surface_id, keyboard),
    ])
}

/// Where the keys would sit for each position, as a drop target in arrange mode
fn spot_bounds(
    position: OverlayPosition,
    bounds: cosmic::iced::Rectangle,
    margin: f32,
    key_size: f32,
) -> cosmic::iced::Rectangle {
    let side_width = (bounds.width * 0.28).min(360.0);
    // The keys end at the middle of the screen, so the center spot runs leftward
    // from there, stopping short of the left spot
    let center_width = side_width.min(bounds.width / 2.0 - margin - side_width - 12.0);
    let height = key_size * 1.6;
    let (x, width) = match position {
        OverlayPosition::TopLeft | OverlayPosition::BottomLeft => (margin, side_width),
        OverlayPosition::TopCenter | OverlayPosition::BottomCenter => {
            (bounds.width / 2.0 - center_width, center_width)
        }
        OverlayPosition::TopRight | OverlayPosition::BottomRight => {
            (bounds.width - side_width - margin, side_width)
        }
    };
    let y = match position {
        OverlayPosition::TopLeft | OverlayPosition::TopCenter | OverlayPosition::TopRight => margin,
        _ => bounds.height - height - margin,
    };
    cosmic::iced::Rectangle {
        x,
        y,
        width,
        height,
    }
}

/// Arrange mode: dims the screen and draws the six spots the keys can snap to.
/// Pressing on a spot and releasing on another (or clicking one) moves the keys there.
#[derive(Debug)]
struct ArrangeCanvas {
    position: OverlayPosition,
    margin: f32,
    key_size: f32,
}

impl ArrangeCanvas {
    fn spot_at(
        &self,
        bounds: cosmic::iced::Rectangle,
        cursor: cosmic::iced::mouse::Cursor,
    ) -> Option<OverlayPosition> {
        let point = cursor.position_in(bounds)?;
        let local = cosmic::iced::Rectangle {
            x: 0.0,
            y: 0.0,
            ..bounds
        };
        crate::config::OverlayPosition::ALL
            .iter()
            .copied()
            .find(|p| spot_bounds(*p, local, self.margin, self.key_size).contains(point))
    }
}

/// A press on a spot, which a release completes as a click or a drag
#[derive(Debug, Default)]
struct Press {
    spot: Option<OverlayPosition>,
    origin: cosmic::iced::Point,
}

impl cosmic::widget::canvas::Program<Message, cosmic::Theme> for ArrangeCanvas {
    type State = Press;

    fn update(
        &self,
        press: &mut Press,
        event: &cosmic::iced::Event,
        bounds: cosmic::iced::Rectangle,
        cursor: cosmic::iced::mouse::Cursor,
    ) -> Option<cosmic::widget::canvas::Action<Message>> {
        use cosmic::iced::mouse::{Button, Event as Mouse};
        use cosmic::widget::canvas::Action;
        match event {
            cosmic::iced::Event::Mouse(Mouse::ButtonPressed(Button::Left)) => {
                press.spot = Some(self.spot_at(bounds, cursor)?);
                press.origin = cursor.position()?;
                Some(Action::capture())
            }
            cosmic::iced::Event::Mouse(Mouse::ButtonReleased(Button::Left)) => {
                press.spot.take()?;
                Some(
                    Action::publish(match self.spot_at(bounds, cursor) {
                        Some(spot) => Message::SetPosition(spot),
                        // Dropped between spots: the keys go back where they were
                        None => Message::ArrangeDrag(None),
                    })
                    .and_capture(),
                )
            }
            // Dragging the keys from their own spot moves them with the pointer
            cosmic::iced::Event::Mouse(Mouse::CursorMoved { position })
                if press.spot == Some(self.position) =>
            {
                let offset = *position - press.origin;
                Some(Action::publish(Message::ArrangeDrag(Some(offset))).and_capture())
            }
            // Otherwise just redraw so the spot under the pointer lights up
            cosmic::iced::Event::Mouse(Mouse::CursorMoved { .. }) => Some(Action::request_redraw()),
            _ => None,
        }
    }

    fn draw(
        &self,
        press: &Press,
        renderer: &cosmic::Renderer,
        theme: &cosmic::Theme,
        bounds: cosmic::iced::Rectangle,
        cursor: cosmic::iced::mouse::Cursor,
    ) -> Vec<cosmic::widget::canvas::Geometry> {
        use cosmic::iced::Color;
        use cosmic::widget::canvas::{Frame, LineDash, Path, Stroke};

        let mut frame = Frame::new(renderer, bounds.size());
        let local = cosmic::iced::Rectangle {
            x: 0.0,
            y: 0.0,
            ..bounds
        };
        frame.fill_rectangle(
            local.position(),
            local.size(),
            Color::from_rgba(0.0, 0.0, 0.0, 0.35),
        );

        let accent = Color::from(theme.cosmic().accent_color());
        let hovered = self.spot_at(bounds, cursor);
        for position in crate::config::OverlayPosition::ALL.iter().copied() {
            let spot = spot_bounds(position, local, self.margin, self.key_size);
            let path = Path::rounded_rectangle(spot.position(), spot.size(), 10.0.into());
            if position == self.position {
                frame.fill(&path, with_opacity(accent, 0.15));
                frame.stroke(&path, Stroke::default().with_color(accent).with_width(2.0));
            } else if hovered == Some(position) {
                let highlight = if press.spot.is_some() { 0.2 } else { 0.08 };
                frame.fill(&path, with_opacity(Color::WHITE, highlight));
                frame.stroke(
                    &path,
                    Stroke::default().with_color(Color::WHITE).with_width(2.0),
                );
            } else {
                frame.stroke(
                    &path,
                    Stroke {
                        line_dash: LineDash {
                            segments: &[6.0, 5.0],
                            offset: 0,
                        },
                        ..Stroke::default()
                            .with_color(with_opacity(Color::WHITE, 0.5))
                            .with_width(1.5)
                    },
                );
            }
        }

        // A dashed guide from the keys' spot to the nearest screen edge, with the distance
        let spot = spot_bounds(self.position, local, self.margin, self.key_size);
        let (from, to, label_at) = match self.position {
            OverlayPosition::TopLeft | OverlayPosition::BottomLeft => {
                let y = spot.center_y();
                ((0.0, y), (spot.x, y), (4.0, y + 6.0))
            }
            OverlayPosition::TopRight | OverlayPosition::BottomRight => {
                let y = spot.center_y();
                let right = spot.x + spot.width;
                ((right, y), (local.width, y), (right + 4.0, y + 6.0))
            }
            OverlayPosition::TopCenter => {
                let x = spot.center_x();
                ((x, 0.0), (x, spot.y), (x + 6.0, 2.0))
            }
            OverlayPosition::BottomCenter => {
                let x = spot.center_x();
                let bottom = spot.y + spot.height;
                ((x, bottom), (x, local.height), (x + 6.0, bottom + 2.0))
            }
        };
        let kiwi = Color::from_rgb8(0xd9, 0xfb, 0x69);
        if self.margin > 0.0 {
            frame.stroke(
                &Path::line(from.into(), to.into()),
                Stroke {
                    line_dash: LineDash {
                        segments: &[4.0, 3.0],
                        offset: 0,
                    },
                    ..Stroke::default().with_color(kiwi).with_width(1.5)
                },
            );
        }
        frame.fill_text(cosmic::widget::canvas::Text {
            content: format!("{:.0} px from edge", self.margin),
            position: label_at.into(),
            color: kiwi,
            size: 13.0.into(),
            ..Default::default()
        });
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        press: &Press,
        bounds: cosmic::iced::Rectangle,
        cursor: cosmic::iced::mouse::Cursor,
    ) -> cosmic::iced::mouse::Interaction {
        use cosmic::iced::mouse::Interaction;
        match self.spot_at(bounds, cursor) {
            Some(_) if press.spot.is_some() => Interaction::Grabbing,
            Some(spot) if spot == self.position => Interaction::Grab,
            Some(_) => Interaction::Pointer,
            None => Interaction::default(),
        }
    }
}

/// The keys while arranging: outlined in the accent color, with a hint to drag them
fn arranging_keys(
    content: cosmic::Element<'static, Message>,
    v_align: cosmic::iced::alignment::Vertical,
    h_align: cosmic::iced::alignment::Horizontal,
) -> cosmic::Element<'static, Message> {
    use cosmic::widget;

    let outlined = widget::container(content)
        .padding(6)
        .class(cosmic::theme::Container::custom(|theme| {
            widget::container::Style {
                border: cosmic::iced::Border {
                    color: cosmic::iced::Color::from(theme.cosmic().accent_color()),
                    width: 2.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        }));
    let hint = widget::container(widget::text::caption("Drag to move").class(
        cosmic::theme::Text::Color(cosmic::iced::Color::from_rgb8(0x1b, 0x24, 0x10)),
    ))
    .padding([3, 10])
    .class(cosmic::theme::Container::custom(|_| {
        widget::container::Style {
            background: Some(cosmic::iced::Color::from_rgb8(0xd9, 0xfb, 0x69).into()),
            border: cosmic::iced::Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }));

    // The hint goes on the side facing the middle of the screen
    let column = widget::Column::new().spacing(8).align_x(match h_align {
        cosmic::iced::alignment::Horizontal::Left => cosmic::iced::Alignment::Start,
        cosmic::iced::alignment::Horizontal::Right => cosmic::iced::Alignment::End,
        // Center positions end at the middle of the screen, like the right side
        _ => cosmic::iced::Alignment::End,
    });
    match v_align {
        cosmic::iced::alignment::Vertical::Top => column.push(outlined).push(hint),
        _ => column.push(hint).push(outlined),
    }
    .into()
}

/// The arrange toolbar: a compact pill with size, length and edge distance,
/// Reset and Done, with the ways out written underneath
fn arrange_toolbar(frame: &Snapshot) -> cosmic::Element<'static, Message> {
    use crate::widgets::stepper;
    use cosmic::widget;

    let length = match frame.theme.layout {
        Layout::Keys => format!("{} keys", frame.history_count),
        Layout::Text => format!(
            "{:.0} px long",
            frame.line_width.unwrap_or(frame.theme.line_width)
        ),
    };
    let divider = || {
        widget::container(widget::Space::new())
            .width(cosmic::iced::Length::Fixed(1.0))
            .height(cosmic::iced::Length::Fixed(20.0))
            .class(cosmic::theme::Container::custom(|theme| {
                widget::container::Style {
                    background: Some(cosmic::iced::Color::from(theme.cosmic().bg_divider()).into()),
                    ..Default::default()
                }
            }))
    };

    let pill = widget::container(
        widget::Row::new()
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .push(stepper(
                format!("{:.0} px", frame.key_size),
                64.0,
                (frame.key_size > 32.0).then_some(Message::NudgeSize(-4.0)),
                (frame.key_size < 160.0).then_some(Message::NudgeSize(4.0)),
            ))
            .push(stepper(
                length,
                96.0,
                Some(Message::NudgeLength(-1)),
                Some(Message::NudgeLength(1)),
            ))
            .push(stepper(
                format!("{:.0} px from edge", frame.margin),
                120.0,
                (frame.margin > 0.0).then_some(Message::NudgeMargin(-4.0)),
                Some(Message::NudgeMargin(4.0)),
            ))
            .push(divider())
            .push(widget::button::standard("Reset").on_press(Message::ResetArrangement))
            .push(widget::button::suggested("Done").on_press(Message::FinishArranging)),
    )
    .padding(8)
    .class(cosmic::theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        widget::container::Style {
            background: Some(cosmic::iced::Color::from(cosmic.bg_color()).into()),
            text_color: Some(cosmic::iced::Color::from(cosmic.on_bg_color())),
            border: cosmic::iced::Border {
                radius: cosmic.radius_xl().into(),
                ..Default::default()
            },
            shadow: cosmic::iced::Shadow {
                color: cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                offset: cosmic::iced::Vector::new(0.0, 8.0),
                blur_radius: 24.0,
            },
            ..Default::default()
        }
    }));

    widget::Column::new()
        .spacing(10)
        .align_x(cosmic::iced::Alignment::Center)
        .push(pill)
        .push(
            widget::text::caption("Esc cancels. Leaves on its own after a minute without input.")
                .class(cosmic::theme::Text::Color(cosmic::iced::Color::from_rgba(
                    1.0, 1.0, 1.0, 0.85,
                ))),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_press_slides_in_but_letting_go_does_not() {
        let mut state = SharedState::default();
        state.history.push(Keystroke::single("a", false));
        Snapshot::take(&mut state, false);
        state.shifted_at = None;

        // Pressing "b" opens room at the edge for it
        state.current_key = Some(("b".into(), KeyModifiers::default()));
        let frame = Snapshot::take(&mut state, false);
        assert_eq!(frame.keystrokes.len(), 2);
        assert!(state.shifted_at.is_some());

        // Letting go of "b" finishes it where it is: nothing moves
        state.shifted_at = None;
        state.current_key = None;
        state.history.push(Keystroke::single("b", false));
        Snapshot::take(&mut state, false);
        assert!(state.shifted_at.is_none());

        // Pressing "c" while "b" is still held (rollover) also slides in
        state.current_key = Some(("c".into(), KeyModifiers::default()));
        Snapshot::take(&mut state, false);
        state.shifted_at = None;
        state.history.push(Keystroke::single("c", false));
        state.current_key = Some(("d".into(), KeyModifiers::default()));
        Snapshot::take(&mut state, false);
        assert!(state.shifted_at.is_some());
    }

    #[test]
    fn a_key_arriving_without_being_held_slides_in() {
        let mut state = SharedState::default();
        state.history.push(Keystroke::single("a", false));
        Snapshot::take(&mut state, false);
        // A scroll goes straight into the history
        state.history.push(Keystroke::single("ScrollUp", false));
        Snapshot::take(&mut state, false);
        assert!(state.shifted_at.is_some());
    }

    #[test]
    fn a_growing_combination_widens_the_slot() {
        let mut state = SharedState::default();
        state.history.push(Keystroke::single("a", false));
        Snapshot::take(&mut state, false);

        // Holding Ctrl: a fresh keystroke in the slot, which doesn't grow
        state.modifiers.ctrl = true;
        state.peak_modifiers.ctrl = true;
        Snapshot::take(&mut state, false);
        assert!(state.slot_grew.is_none());

        // Adding Shift grows the held combination from one key to two
        state.modifiers.shift = true;
        state.peak_modifiers.shift = true;
        let frame = Snapshot::take(&mut state, false);
        assert_eq!(state.slot_grew.map(|(_, from)| from), Some(1));
        assert_eq!(frame.motion.slot_grew.map(|(_, from)| from), Some(1));
    }

    #[test]
    fn letting_go_of_one_modifier_keeps_the_others_in_place() {
        let mut state = SharedState::default();
        // Ctrl and Shift held, then Ctrl let go
        state.peak_modifiers.ctrl = true;
        state.peak_modifiers.shift = true;
        state.modifiers.shift = true;
        let (keys, _) = live_keystrokes(&state);
        let held = keys.last().unwrap();
        assert_eq!(held.keys, ["Ctrl", "⇧"]);
        // Ctrl (the first key) is drawn as empty space; Shift stays where it was
        assert_eq!(held.released_parts, 0b01);

        let peak = KeyModifiers {
            super_key: true,
            ctrl: true,
            alt: true,
            shift: true,
        };
        let only_alt = KeyModifiers {
            alt: true,
            ..Default::default()
        };
        assert_eq!(released_modifiers(&peak, &only_alt), 0b1011);
    }

    #[test]
    fn a_quick_repeat_counts_up_in_the_slot() {
        let mut state = SharedState::default();
        state.history.push(Keystroke::single("a", false));
        state.current_key = Some(("a".into(), KeyModifiers::default()));
        let (keys, held) = live_keystrokes(&state);
        assert_eq!(keys.len(), 1);
        // The repeat counts up on the finished keystroke; nothing new is held
        assert!(!held);
        assert!(keys[0].pressed);
        assert_eq!(keys[0].count, 2);
    }

    #[test]
    fn held_modifiers_after_a_combination_keep_it_in_the_slot() {
        let mut state = SharedState::default();
        state.modifiers.ctrl = true;
        state.peak_modifiers.ctrl = true;
        state.history.push(Keystroke::single("C", false));
        // Ctrl is still held, but a key was already pressed with it
        state.key_pressed_with_modifiers = true;
        assert_eq!(live_keystrokes(&state).0.len(), 1);
        state.key_pressed_with_modifiers = false;
        assert_eq!(live_keystrokes(&state).0.len(), 2);
    }

    #[test]
    fn snap_spots_fit_and_dont_overlap() {
        for (width, height) in [(1920.0, 1080.0), (1280.0, 800.0)] {
            let screen = cosmic::iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width,
                height,
            };
            let spots: Vec<_> = OverlayPosition::ALL
                .iter()
                .map(|p| spot_bounds(*p, screen, 20.0, 160.0))
                .collect();
            for (i, a) in spots.iter().enumerate() {
                assert!(a.x >= 0.0 && a.y >= 0.0);
                assert!(a.x + a.width <= width && a.y + a.height <= height);
                for b in &spots[i + 1..] {
                    assert!(a.intersection(b).is_none(), "{a:?} overlaps {b:?}");
                }
            }
        }
    }
}
