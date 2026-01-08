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
pub use input::event::pointer::{Axis, ButtonState};

/// Errors that can occur during input capture
#[derive(Debug)]
#[allow(dead_code)]
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

/// Swipe direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Input event types we care about
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum InputEvent {
    /// Key press or release
    Key { key: u32, state: KeyState },
    /// Mouse button press or release
    MouseButton {
        button: u32,
        state: ButtonState,
        /// True if from a touchpad (tap), false if from a mouse
        is_touchpad: bool,
    },
    /// Mouse/pointer motion (used to detect drag)
    MouseMotion { dx: f64, dy: f64 },
    /// Mouse wheel scroll
    MouseScroll { axis: Axis, value: f64 },
    /// Touchpad two-finger scroll
    TouchpadScroll { axis: Axis, value: f64 },
    /// Multi-finger swipe gesture (3+ fingers)
    Swipe {
        finger_count: i32,
        direction: SwipeDirection,
    },
    /// Multi-finger hold/tap gesture (3+ fingers held briefly without movement)
    Hold { finger_count: i32 },
}

/// Tracks ongoing swipe gesture state
#[derive(Debug, Default)]
struct SwipeState {
    finger_count: i32,
    total_dx: f64,
    total_dy: f64,
    active: bool,
}

/// Tracks accumulated scroll for touchpad
#[derive(Debug, Default)]
struct ScrollAccumulator {
    vertical: f64,
    horizontal: f64,
}

/// Input capture context wrapping libinput
pub struct InputCapture {
    libinput: Libinput,
    swipe_state: SwipeState,
    touchpad_scroll: ScrollAccumulator,
}

impl InputCapture {
    /// Create a new input capture context
    pub fn new() -> Result<Self, CaptureError> {
        let mut libinput = Libinput::new_with_udev(DirectAccess);

        libinput
            .udev_assign_seat("seat0")
            .map_err(|_| CaptureError::SeatError("Failed to assign seat0".into()))?;

        // Process initial device events and enable tap-to-click on touchpads
        libinput.dispatch().ok();
        for event in libinput.by_ref() {
            use input::event::device::DeviceEvent;
            use input::event::Event;
            if let Event::Device(DeviceEvent::Added(dev_event)) = event {
                use input::event::EventTrait;
                let mut device = dev_event.device();
                Self::configure_device(&mut device);
            }
        }

        log::info!("Input capture initialized on seat0");
        Ok(Self {
            libinput,
            swipe_state: SwipeState::default(),
            touchpad_scroll: ScrollAccumulator::default(),
        })
    }

    /// Configure a device (enable tap-to-click for touchpads)
    fn configure_device(device: &mut input::Device) {
        use input::DeviceCapability;

        // Check if it's a touchpad (has pointer + gesture but not a mouse)
        if device.has_capability(DeviceCapability::Pointer)
            && device.has_capability(DeviceCapability::Gesture)
        {
            // Enable tap-to-click
            if device.config_tap_finger_count() > 0 {
                if let Err(e) = device.config_tap_set_enabled(true) {
                    log::warn!(
                        "Failed to enable tap-to-click on {}: {:?}",
                        device.name(),
                        e
                    );
                } else {
                    log::info!("Enabled tap-to-click on: {}", device.name());
                }

                // Also enable tap-and-drag if available
                let _ = device.config_tap_set_drag_enabled(true);
            }
        }
    }

    /// Get the file descriptor for polling
    #[allow(dead_code)]
    pub fn as_fd(&self) -> impl AsFd + '_ {
        &self.libinput
    }

    /// Dispatch pending events (call after poll indicates ready)
    pub fn dispatch(&mut self) -> Result<(), CaptureError> {
        self.libinput
            .dispatch()
            .map_err(|e| CaptureError::DeviceError(e.to_string()))
    }

    /// Collect available events into a Vec (needed for stateful gesture tracking)
    pub fn events(&mut self) -> Vec<InputEvent> {
        use input::event::device::DeviceEvent;
        use input::event::gesture::{
            GestureEvent, GestureEventCoordinates, GestureEventTrait, GestureSwipeEvent,
        };
        use input::event::keyboard::KeyboardEventTrait;
        use input::event::pointer::PointerScrollEvent;
        use input::event::{Event, EventTrait};
        use input::DeviceCapability;

        let mut results = Vec::new();

        for event in self.libinput.by_ref() {
            match event {
                Event::Device(DeviceEvent::Added(dev_event)) => {
                    // Configure newly added devices (hot-plug support)
                    let mut device = dev_event.device();
                    Self::configure_device(&mut device);
                }
                Event::Keyboard(kb_event) => {
                    use input::event::keyboard::KeyboardEvent;
                    if let KeyboardEvent::Key(key) = kb_event {
                        results.push(InputEvent::Key {
                            key: key.key(),
                            state: key.key_state(),
                        });
                    }
                }
                Event::Pointer(ptr_event) => {
                    use input::event::pointer::PointerEvent;
                    match ptr_event {
                        PointerEvent::Button(ref btn) => {
                            let device = btn.device();
                            let is_touchpad = device.has_capability(DeviceCapability::Gesture);
                            results.push(InputEvent::MouseButton {
                                button: btn.button(),
                                state: btn.button_state(),
                                is_touchpad,
                            });
                        }
                        PointerEvent::Motion(motion) => {
                            results.push(InputEvent::MouseMotion {
                                dx: motion.dx(),
                                dy: motion.dy(),
                            });
                        }
                        PointerEvent::ScrollWheel(scroll) => {
                            if scroll.has_axis(Axis::Vertical) {
                                results.push(InputEvent::MouseScroll {
                                    axis: Axis::Vertical,
                                    value: scroll.scroll_value_v120(Axis::Vertical),
                                });
                            } else if scroll.has_axis(Axis::Horizontal) {
                                results.push(InputEvent::MouseScroll {
                                    axis: Axis::Horizontal,
                                    value: scroll.scroll_value_v120(Axis::Horizontal),
                                });
                            }
                        }
                        PointerEvent::ScrollFinger(scroll) => {
                            // Accumulate scroll values
                            if scroll.has_axis(Axis::Vertical) {
                                self.touchpad_scroll.vertical +=
                                    scroll.scroll_value(Axis::Vertical);
                            }
                            if scroll.has_axis(Axis::Horizontal) {
                                self.touchpad_scroll.horizontal +=
                                    scroll.scroll_value(Axis::Horizontal);
                            }

                            // Emit when accumulated scroll crosses threshold
                            const SCROLL_THRESHOLD: f64 = 15.0;

                            if self.touchpad_scroll.vertical.abs() >= SCROLL_THRESHOLD {
                                let value = self.touchpad_scroll.vertical;
                                self.touchpad_scroll.vertical = 0.0; // Reset after emitting
                                results.push(InputEvent::TouchpadScroll {
                                    axis: Axis::Vertical,
                                    value,
                                });
                            }
                            if self.touchpad_scroll.horizontal.abs() >= SCROLL_THRESHOLD {
                                let value = self.touchpad_scroll.horizontal;
                                self.touchpad_scroll.horizontal = 0.0; // Reset after emitting
                                results.push(InputEvent::TouchpadScroll {
                                    axis: Axis::Horizontal,
                                    value,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                Event::Gesture(gesture_event) => {
                    match gesture_event {
                        GestureEvent::Swipe(swipe) => {
                            match swipe {
                                GestureSwipeEvent::Begin(ref begin_event) => {
                                    // Start tracking swipe
                                    self.swipe_state.finger_count = begin_event.finger_count();
                                    self.swipe_state.total_dx = 0.0;
                                    self.swipe_state.total_dy = 0.0;
                                    self.swipe_state.active = true;
                                    log::debug!(
                                        "Swipe begin: {} fingers",
                                        self.swipe_state.finger_count
                                    );
                                }
                                GestureSwipeEvent::Update(ref update_event) => {
                                    // Accumulate delta
                                    if self.swipe_state.active {
                                        self.swipe_state.total_dx += update_event.dx();
                                        self.swipe_state.total_dy += update_event.dy();
                                    }
                                }
                                GestureSwipeEvent::End(_) => {
                                    // Emit swipe event if significant
                                    if self.swipe_state.active {
                                        const SWIPE_THRESHOLD: f64 = 50.0;
                                        let dx = self.swipe_state.total_dx;
                                        let dy = self.swipe_state.total_dy;

                                        log::debug!(
                                            "Swipe end: {} fingers, dx={:.1}, dy={:.1}",
                                            self.swipe_state.finger_count,
                                            dx,
                                            dy
                                        );

                                        if dy.abs() > SWIPE_THRESHOLD || dx.abs() > SWIPE_THRESHOLD
                                        {
                                            let direction = if dy.abs() > dx.abs() {
                                                if dy < 0.0 {
                                                    SwipeDirection::Up
                                                } else {
                                                    SwipeDirection::Down
                                                }
                                            } else {
                                                if dx < 0.0 {
                                                    SwipeDirection::Left
                                                } else {
                                                    SwipeDirection::Right
                                                }
                                            };
                                            results.push(InputEvent::Swipe {
                                                finger_count: self.swipe_state.finger_count,
                                                direction,
                                            });
                                        }

                                        self.swipe_state.active = false;
                                    }
                                }
                                _ => {} // Handle any future variants
                            }
                        }
                        GestureEvent::Hold(hold) => {
                            use input::event::gesture::{GestureEndEvent, GestureHoldEvent};
                            match hold {
                                GestureHoldEvent::End(ref end_event) => {
                                    let finger_count = end_event.finger_count();
                                    let cancelled = end_event.cancelled();
                                    log::debug!(
                                        "Hold end: {} fingers, cancelled={}",
                                        finger_count,
                                        cancelled
                                    );

                                    // If not cancelled, it was a clean tap-like gesture
                                    // (user lifted fingers without swiping)
                                    if !cancelled && finger_count >= 3 {
                                        results.push(InputEvent::Hold { finger_count });
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {} // Ignore pinch
                    }
                }
                _ => {}
            }
        }

        results
    }
}
