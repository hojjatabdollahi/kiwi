//! Shared types, configuration, and IPC protocol for Kiwi keystroke visualizer.

use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic_config::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

/// The APP_ID used for cosmic-config
pub const APP_ID: &str = "dev.hojjat.kiwi";

/// D-Bus service name
pub const DBUS_NAME: &str = "dev.hojjat.Kiwi";
/// D-Bus object path
pub const DBUS_PATH: &str = "/dev/hojjat/Kiwi";

/// User configuration - persisted via cosmic-config
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, CosmicConfigEntry)]
#[version = 1]
pub struct Config {
    /// Whether keystroke visualization is enabled
    pub enabled: bool,
}

/// Input event for display (serializable for IPC if needed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    /// Key press or release
    Key {
        key: u32,
        pressed: bool,
        modifiers: Modifiers,
    },
    /// Mouse button press or release
    MouseButton {
        button: u32,
        pressed: bool,
    },
    /// Mouse scroll
    MouseScroll {
        dx: f64,
        dy: f64,
    },
}

/// Active modifier keys
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

/// Status response from daemon
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonStatus {
    pub running: bool,
    pub enabled: bool,
}

/// D-Bus interface name
pub const DBUS_INTERFACE: &str = "dev.hojjat.Kiwi";

/// D-Bus proxy for the daemon - used by the applet to control the daemon
#[zbus::proxy(
    interface = "dev.hojjat.Kiwi",
    default_service = "dev.hojjat.Kiwi",
    default_path = "/dev/hojjat/Kiwi"
)]
pub trait Kiwi {
    /// Enable or disable keystroke visualization
    fn set_enabled(&self, enabled: bool) -> zbus::Result<()>;

    /// Check if keystroke visualization is enabled
    fn is_enabled(&self) -> zbus::Result<bool>;

    /// Quit the daemon
    fn quit(&self) -> zbus::Result<()>;
}
