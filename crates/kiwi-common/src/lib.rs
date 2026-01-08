//! Shared types, configuration, and IPC protocol for Kiwi keystroke visualizer.

use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic_config::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

/// The APP_ID used for cosmic-config
pub const APP_ID: &str = "dev.hojjat.kiwi";

/// User configuration - persisted via cosmic-config
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, CosmicConfigEntry)]
#[version = 1]
pub struct Config {
    /// Whether keystroke visualization is enabled
    pub enabled: bool,
}

/// A captured keystroke event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    /// The key that was pressed (e.g., "A", "Ctrl", "Space")
    pub key: String,
    /// Modifier keys held during the press
    pub modifiers: Modifiers,
    /// Timestamp in milliseconds
    pub timestamp: u64,
}

/// Active modifier keys
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

/// Commands from applet to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonCommand {
    /// Show/hide the overlay
    SetVisible(bool),
    /// Request current status
    GetStatus,
}

/// Status response from daemon to applet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub visible: bool,
    pub capturing: bool,
}
