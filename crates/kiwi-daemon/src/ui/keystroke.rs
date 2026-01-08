//! Keystroke visualization widget

use std::time::Instant;

use cosmic::iced::{self, Background, Border, Color, Length};
use cosmic::iced_widget::container;
use cosmic::widget::{self, text};
use cosmic::Element;

/// How long keystrokes stay visible before fully fading
pub const FADE_DURATION_SECS: f32 = 5.0;

/// Represents a keystroke to display
#[derive(Debug, Clone)]
pub struct Keystroke {
    /// All keys in this keystroke (modifiers + main key)
    pub keys: Vec<String>,
    /// Whether this keystroke is currently being held
    pub pressed: bool,
    /// When this keystroke was created
    pub timestamp: Instant,
}

/// Active modifier keys for a keystroke
#[derive(Debug, Clone, Default)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

impl KeyModifiers {
    /// Returns true if any modifier is active
    pub fn any(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.super_key
    }

    /// Returns the modifier keys as display strings
    pub fn to_parts(&self) -> Vec<String> {
        let mut parts = Vec::new();
        if self.super_key {
            parts.push("Super".to_string());
        }
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        if self.shift {
            parts.push("⇧".to_string());
        }
        parts
    }

    /// Create keystroke from just modifiers (for showing held modifiers)
    pub fn to_keystroke(&self, pressed: bool) -> Option<Keystroke> {
        let keys = self.to_parts();
        if keys.is_empty() {
            None
        } else {
            Some(Keystroke { keys, pressed, timestamp: Instant::now() })
        }
    }
}

impl Keystroke {
    /// Create a single key keystroke
    pub fn single(key: impl Into<String>, pressed: bool) -> Self {
        Self {
            keys: vec![key.into()],
            pressed,
            timestamp: Instant::now(),
        }
    }

    /// Create a combination keystroke from modifiers + key
    pub fn combination(modifiers: &KeyModifiers, key: impl Into<String>, pressed: bool) -> Self {
        let mut keys = modifiers.to_parts();
        keys.push(key.into());
        Self { keys, pressed, timestamp: Instant::now() }
    }

    /// Create from just modifiers
    pub fn from_modifiers(modifiers: &KeyModifiers, pressed: bool) -> Option<Self> {
        modifiers.to_keystroke(pressed)
    }

    /// Check if this is a combination (multiple keys)
    pub fn is_combination(&self) -> bool {
        self.keys.len() > 1
    }

    /// Get age in seconds
    pub fn age_secs(&self) -> f32 {
        self.timestamp.elapsed().as_secs_f32()
    }

    /// Check if this keystroke has expired (older than fade duration)
    pub fn is_expired(&self) -> bool {
        self.age_secs() >= FADE_DURATION_SECS
    }

    /// Get opacity based on age (1.0 = new, 0.0 = fully faded)
    /// Stays at 1.0 for the first 70% of duration, then fades in the last 30% with easing
    pub fn opacity(&self) -> f32 {
        if self.pressed {
            1.0  // Pressed keys are always fully visible
        } else {
            let age = self.age_secs();
            let fade_start = FADE_DURATION_SECS * 0.7;  // Start fading at 70%
            
            if age >= FADE_DURATION_SECS {
                0.0
            } else if age <= fade_start {
                1.0  // Full opacity for first 70%
            } else {
                // Fade from 1.0 to 0.0 in the last 30% with ease-out
                let fade_duration = FADE_DURATION_SECS - fade_start;
                let t = (age - fade_start) / fade_duration;  // 0.0 -> 1.0
                let eased = ease_in_cubic(t);
                1.0 - eased
            }
        }
    }
}

/// Ease-in cubic: slow start, fast end
/// t: 0.0 -> 1.0, returns 0.0 -> 1.0
fn ease_in_cubic(t: f32) -> f32 {
    t.powi(3)
}

// Style constants
const BORDER_WIDTH: f32 = 1.0;
const BORDER_RADIUS: f32 = 6.0;
const KEY_SIZE: f32 = 48.0;
const FONT_SIZE: f32 = 20.0;
const PLUS_FONT_SIZE: f32 = 14.0;
const INNER_PADDING: u16 = 10;
const KEY_GAP: f32 = 8.0;

/// Renders a keystroke widget with opacity based on age
/// 
/// - Single key: square with border
/// - Combination: single rectangle with keys + "+" as text (no inner borders)
pub fn keystroke_widget<'a, M: 'a>(keystroke: &Keystroke) -> Element<'a, M> {
    let opacity = keystroke.opacity();
    
    let bg = if keystroke.pressed {
        Color::from_rgba(0.3, 0.3, 0.5, 0.85 * opacity)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.7 * opacity)
    };
    
    let border_color = Color::from_rgba(1.0, 1.0, 1.0, 0.3 * opacity);
    let text_color = Color::from_rgba(1.0, 1.0, 1.0, opacity);
    let plus_color = Color::from_rgba(1.0, 1.0, 1.0, 0.5 * opacity);

    if keystroke.is_combination() {
        // Combination: single rectangle with "Key + Key + Key" text
        let mut row_children: Vec<Element<'a, M>> = Vec::new();

        for (i, key) in keystroke.keys.iter().enumerate() {
            if i > 0 {
                // Add "+" separator
                row_children.push(
                    text::Text::new("+")
                        .size(PLUS_FONT_SIZE)
                        .class(cosmic::theme::Text::Color(plus_color))
                        .into(),
                );
            }
            row_children.push(
                text::Text::new(key.clone())
                    .size(FONT_SIZE)
                    .class(cosmic::theme::Text::Color(text_color))
                    .into(),
            );
        }

        widget::container(
            widget::row::with_children(row_children)
                .spacing(6)
                .align_y(iced::Alignment::Center),
        )
        .height(Length::Fixed(KEY_SIZE))
        .padding([0, INNER_PADDING])
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .class(cosmic::theme::Container::custom(move |_| container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: border_color,
                width: BORDER_WIDTH,
                radius: BORDER_RADIUS.into(),
            },
            ..Default::default()
        }))
        .into()
    } else {
        // Single key: square
        widget::container(
            text::Text::new(keystroke.keys[0].clone())
                .size(FONT_SIZE)
                .class(cosmic::theme::Text::Color(text_color))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .width(Length::Fixed(KEY_SIZE))
        .height(Length::Fixed(KEY_SIZE))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .class(cosmic::theme::Container::custom(move |_| container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: border_color,
                width: BORDER_WIDTH,
                radius: BORDER_RADIUS.into(),
            },
            ..Default::default()
        }))
        .into()
    }
}

// Available width for keystrokes (window width minus padding)
const AVAILABLE_WIDTH: f32 = 400.0 - 20.0; // 400px window - 2*10px padding

impl Keystroke {
    /// Estimate the width of this keystroke widget
    fn estimated_width(&self) -> f32 {
        if self.is_combination() {
            // Combination: text for each key + "+" separators + padding
            // Rough estimate: ~20px per character, 6px spacing between parts
            let text_width: f32 = self.keys.iter()
                .map(|k| k.len() as f32 * 12.0)  // ~12px per char
                .sum();
            let plus_width = (self.keys.len() - 1) as f32 * (PLUS_FONT_SIZE + 6.0);  // "+" with spacing
            text_width + plus_width + (INNER_PADDING as f32 * 2.0)
        } else {
            // Single key: fixed square size
            KEY_SIZE
        }
    }
}

/// Renders a row of keystrokes, right-aligned with newest on the right.
/// Filters out expired keystrokes and only includes ones that fit in available width.
pub fn keystrokes_row<'a, M: 'a + Clone>(keystrokes: &[Keystroke]) -> Element<'a, M> {
    let mut total_width: f32 = 0.0;
    let mut fitting_keystrokes: Vec<&Keystroke> = Vec::new();

    // Process from newest to oldest, skip expired, accumulate width
    for keystroke in keystrokes.iter().rev() {
        // Skip expired keystrokes
        if keystroke.is_expired() {
            continue;
        }

        let width = keystroke.estimated_width();
        let gap = if fitting_keystrokes.is_empty() { 0.0 } else { KEY_GAP };
        
        if total_width + gap + width <= AVAILABLE_WIDTH {
            total_width += gap + width;
            fitting_keystrokes.push(keystroke);
        } else {
            break;  // No more space
        }
    }

    // Reverse so newest is on the right
    fitting_keystrokes.reverse();

    let children: Vec<Element<'a, M>> = fitting_keystrokes
        .into_iter()
        .map(|k| keystroke_widget(k))
        .collect();

    widget::container(
        widget::row::with_children(children)
            .spacing(KEY_GAP)
            .align_y(iced::Alignment::Center)
    )
    .width(Length::Fill)
    .align_x(iced::alignment::Horizontal::Right)
    .into()
}
