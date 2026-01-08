//! System tray icon using StatusNotifierItem (ksni)

use ksni::{menu::StandardItem, Handle, MenuItem, Tray};
use std::sync::mpsc::{self, Receiver, Sender};

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
}

impl KiwiTray {
    pub fn new(active: bool, tx: Sender<TrayAction>) -> Self {
        Self { active, tx }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

impl Tray for KiwiTray {
    fn id(&self) -> String {
        "dev.hojjat.kiwi".to_string()
    }

    fn title(&self) -> String {
        "Kiwi".to_string()
    }

    fn icon_name(&self) -> String {
        // Use the installed icons (from hicolor theme)
        if self.active {
            "kiwi-on".to_string()
        } else {
            "kiwi-off".to_string()
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
pub fn create_tray(initial_active: bool) -> (Handle<KiwiTray>, Receiver<TrayAction>) {
    let (tx, rx) = mpsc::channel();
    let tray = KiwiTray::new(initial_active, tx);
    let service = ksni::TrayService::new(tray);
    let handle = service.handle();
    service.spawn();
    (handle, rx)
}
