//! Keystroke capture via libinput/evdev for Kiwi.
//!
//! This crate provides global keystroke capture functionality.
//! Note: Requires elevated privileges (input group membership or root).

use kiwi_common::KeyEvent;

/// Trait for keystroke capture backends
pub trait KeystrokeCapture {
    /// Start capturing keystrokes
    fn start(&mut self) -> Result<(), CaptureError>;
    
    /// Stop capturing
    fn stop(&mut self);
    
    /// Poll for the next key event (non-blocking)
    fn poll(&mut self) -> Option<KeyEvent>;
}

/// Errors that can occur during keystroke capture
#[derive(Debug)]
pub enum CaptureError {
    /// Permission denied (not in input group)
    PermissionDenied,
    /// No input devices found
    NoDevices,
    /// Device open failed
    DeviceError(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "Permission denied. Add user to 'input' group."),
            Self::NoDevices => write!(f, "No input devices found"),
            Self::DeviceError(e) => write!(f, "Device error: {}", e),
        }
    }
}

impl std::error::Error for CaptureError {}

/// Placeholder implementation - TODO: implement with evdev/libinput
pub struct EvdevCapture {
    // TODO: Add evdev state
}

impl EvdevCapture {
    pub fn new() -> Result<Self, CaptureError> {
        log::info!("EvdevCapture::new() - placeholder");
        Ok(Self {})
    }
}

impl Default for EvdevCapture {
    fn default() -> Self {
        Self {}
    }
}

impl KeystrokeCapture for EvdevCapture {
    fn start(&mut self) -> Result<(), CaptureError> {
        log::info!("EvdevCapture::start() - placeholder");
        Ok(())
    }
    
    fn stop(&mut self) {
        log::info!("EvdevCapture::stop() - placeholder");
    }
    
    fn poll(&mut self) -> Option<KeyEvent> {
        // TODO: Actually poll evdev devices
        None
    }
}
