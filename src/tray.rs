//! System tray icon using StatusNotifierItem (ksni)

use crossbeam_channel::Sender;
use ksni::{menu::StandardItem, Handle, Icon, MenuItem, Tray};

/// Embedded PNG icons at 32x32 (good size for most tray implementations)
const ICON_ON_PNG: &[u8] = include_bytes!("../data/icons/kiwi-on-32.png");
const ICON_OFF_PNG: &[u8] = include_bytes!("../data/icons/kiwi-off-32.png");

/// Actions that can be triggered from the tray menu
#[derive(Debug, Clone)]
pub enum TrayAction {
    ShowSettings,
    ToggleActive,
    Quit,
}

/// The tray icon state
pub struct KiwiTray {
    active: bool,
    tx: Sender<TrayAction>,
    icon_on: Vec<Icon>,
    icon_off: Vec<Icon>,
}

impl KiwiTray {
    pub fn new(active: bool, tx: Sender<TrayAction>) -> Self {
        Self {
            active,
            tx,
            icon_on: load_icon_pixmap(ICON_ON_PNG),
            icon_off: load_icon_pixmap(ICON_OFF_PNG),
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

/// Load a PNG and convert to ksni::Icon (ARGB format)
fn load_icon_pixmap(png_data: &[u8]) -> Vec<Icon> {
    match image::load_from_memory_with_format(png_data, image::ImageFormat::Png) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let width = rgba.width() as i32;
            let height = rgba.height() as i32;

            // Convert RGBA to ARGB bytes (ksni expects ARGB in network byte order as bytes)
            let mut argb_data = Vec::with_capacity((width * height * 4) as usize);
            for pixel in rgba.pixels() {
                let [r, g, b, a] = pixel.0;
                // Pack as ARGB in big-endian (network byte order): A, R, G, B
                argb_data.push(a);
                argb_data.push(r);
                argb_data.push(g);
                argb_data.push(b);
            }

            vec![Icon {
                width,
                height,
                data: argb_data,
            }]
        }
        Err(e) => {
            log::error!("Failed to load tray icon: {}", e);
            Vec::new()
        }
    }
}

impl Tray for KiwiTray {
    fn id(&self) -> String {
        "dev.hojjat.kiwi".to_string()
    }

    fn title(&self) -> String {
        "Kiwi".to_string()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        if self.active {
            self.icon_on.clone()
        } else {
            self.icon_off.clone()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        // Left-click on tray icon opens settings
        log::info!("Tray icon clicked - sending ShowSettings");
        if let Err(e) = self.tx.send(TrayAction::ShowSettings) {
            log::error!("Failed to send ShowSettings: {}", e);
        }
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Kiwi Keystroke Visualizer".to_string(),
            description: if self.active {
                "Active - Keystrokes are being visualized".to_string()
            } else {
                "Inactive - Click to configure".to_string()
            },
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let tx_settings = self.tx.clone();
        let tx_toggle = self.tx.clone();
        let tx_quit = self.tx.clone();
        let is_active = self.active;

        vec![
            MenuItem::Standard(StandardItem {
                label: "Settings...".to_string(),
                activate: Box::new(move |_| {
                    log::info!("Menu: Settings clicked");
                    if let Err(e) = tx_settings.send(TrayAction::ShowSettings) {
                        log::error!("Failed to send ShowSettings: {}", e);
                    }
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: if is_active {
                    "Deactivate".to_string()
                } else {
                    "Activate".to_string()
                },
                activate: Box::new(move |_| {
                    log::info!("Menu: Toggle clicked");
                    if let Err(e) = tx_toggle.send(TrayAction::ToggleActive) {
                        log::error!("Failed to send ToggleActive: {}", e);
                    }
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Quit".to_string(),
                activate: Box::new(move |_| {
                    log::info!("Menu: Quit clicked");
                    if let Err(e) = tx_quit.send(TrayAction::Quit) {
                        log::error!("Failed to send Quit: {}", e);
                    }
                }),
                ..Default::default()
            }),
        ]
    }
}

/// Create the tray icon and return a handle and receiver for actions
pub fn create_tray(initial_active: bool, tx: Sender<TrayAction>) -> Handle<KiwiTray> {
    let tray = KiwiTray::new(initial_active, tx);
    let service = ksni::TrayService::new(tray);
    let handle = service.handle();
    service.spawn();
    handle
}
