//! Keystroke and mouse capture via libinput for Kiwi.
//!
//! This crate provides global input capture functionality.
//! Note: Requires 'input' group membership.

use input::{Libinput, LibinputInterface};
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsFd, OwnedFd};
use std::path::Path;

pub use input::event::keyboard::KeyState;
pub use input::event::pointer::{ButtonState, Axis};

/// Errors that can occur during input capture
#[derive(Debug)]
pub enum CaptureError {
    /// Permission denied (not in input group)
    PermissionDenied,
    /// Failed to assign seat
    SeatError(String),
    /// Device open failed
    DeviceError(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "Permission denied. Add user to 'input' group."),
            Self::SeatError(e) => write!(f, "Failed to assign seat: {}", e),
            Self::DeviceError(e) => write!(f, "Device error: {}", e),
        }
    }
}

impl std::error::Error for CaptureError {}

/// Direct device access interface (requires input group)
struct DirectAccess;

impl LibinputInterface for DirectAccess {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        OpenOptions::new()
            .read(true)
            .write((flags & libc::O_WRONLY != 0) || (flags & libc::O_RDWR != 0))
            .custom_flags(flags & !libc::O_RDONLY & !libc::O_WRONLY & !libc::O_RDWR)
            .open(path)
            .map(|file| file.into())
            .map_err(|e| e.raw_os_error().unwrap_or(libc::EACCES))
    }

    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(fd);
    }
}

/// Input event types we care about
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Key press or release
    Key {
        key: u32,
        state: KeyState,
    },
    /// Mouse button press or release
    MouseButton {
        button: u32,
        state: ButtonState,
    },
    /// Mouse scroll
    MouseScroll {
        axis: Axis,
        value: f64,
    },
}

/// Input capture context wrapping libinput
pub struct InputCapture {
    libinput: Libinput,
}

impl InputCapture {
    /// Create a new input capture context
    pub fn new() -> Result<Self, CaptureError> {
        let mut libinput = Libinput::new_with_udev(DirectAccess);
        
        libinput
            .udev_assign_seat("seat0")
            .map_err(|_| CaptureError::SeatError("Failed to assign seat0".into()))?;
        
        log::info!("Input capture initialized on seat0");
        Ok(Self { libinput })
    }

    /// Get the file descriptor for polling
    pub fn as_fd(&self) -> impl AsFd + '_ {
        &self.libinput
    }

    /// Dispatch pending events (call after poll indicates ready)
    pub fn dispatch(&mut self) -> Result<(), CaptureError> {
        self.libinput
            .dispatch()
            .map_err(|e| CaptureError::DeviceError(e.to_string()))
    }

    /// Iterate over available events
    pub fn events(&mut self) -> impl Iterator<Item = InputEvent> + '_ {
        self.libinput.by_ref().filter_map(|event| {
            use input::event::Event;
            use input::event::keyboard::KeyboardEventTrait;
            use input::event::pointer::PointerScrollEvent;

            match event {
                Event::Keyboard(kb_event) => {
                    use input::event::keyboard::KeyboardEvent;
                    if let KeyboardEvent::Key(key) = kb_event {
                        Some(InputEvent::Key {
                            key: key.key(),
                            state: key.key_state(),
                        })
                    } else {
                        None
                    }
                }
                Event::Pointer(ptr_event) => {
                    use input::event::pointer::PointerEvent;
                    match ptr_event {
                        PointerEvent::Button(btn) => Some(InputEvent::MouseButton {
                            button: btn.button(),
                            state: btn.button_state(),
                        }),
                        PointerEvent::ScrollWheel(scroll) => {
                            // Report vertical scroll first, then horizontal if present
                            if scroll.has_axis(Axis::Vertical) {
                                Some(InputEvent::MouseScroll {
                                    axis: Axis::Vertical,
                                    value: scroll.scroll_value_v120(Axis::Vertical),
                                })
                            } else if scroll.has_axis(Axis::Horizontal) {
                                Some(InputEvent::MouseScroll {
                                    axis: Axis::Horizontal,
                                    value: scroll.scroll_value_v120(Axis::Horizontal),
                                })
                            } else {
                                None
                            }
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        })
    }
}
