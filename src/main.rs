use cosmic::app::Core;
use cosmic::iced::{window, Limits};
use cosmic::iced_core::event::wayland::OutputEvent;
use cosmic::iced_futures::event::listen_with;
use cosmic::iced_futures::Subscription;
use cosmic::iced_runtime::platform_specific::wayland::layer_surface::{
    IcedOutput, SctkLayerSurfaceSettings,
};
use cosmic::iced_winit::commands::layer_surface::{destroy_layer_surface, get_layer_surface};
use cosmic::widget::text;
use cosmic_client_toolkit::sctk::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use wayland_client::protocol::wl_output::WlOutput;

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let settings = cosmic::app::Settings::default()
        .no_main_window(true)
        .exit_on_close(false);
    cosmic::app::run::<Kiwi>(settings, ())
}

/// Tracks an output and its associated layer surface
#[derive(Debug, Clone)]
struct OutputState {
    output: WlOutput,
    surface_id: window::Id,
    name: Option<String>,
}

struct Kiwi {
    core: Core,
    /// One layer surface per output
    outputs: Vec<OutputState>,
}

#[derive(Debug, Clone)]
enum Message {
    OutputEvent(OutputEvent, WlOutput),
}

fn create_layer_surface_for_output(
    output: &WlOutput,
    id: window::Id,
) -> cosmic::iced::Task<cosmic::Action<Message>> {
    get_layer_surface(SctkLayerSurfaceSettings {
        id,
        layer: Layer::Overlay,
        keyboard_interactivity: KeyboardInteractivity::None,
        anchor: Anchor::TOP | Anchor::RIGHT,
        output: IcedOutput::Output(output.clone()),
        namespace: "kiwi".to_string(),
        size: Some((Some(300), Some(60))),
        exclusive_zone: -1,
        size_limits: Limits::NONE.min_width(1.0).min_height(1.0),
        ..Default::default()
    })
}

impl cosmic::Application for Kiwi {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.hojjat.kiwi";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(
        core: Core,
        _flags: Self::Flags,
    ) -> (Self, cosmic::iced::Task<cosmic::Action<Self::Message>>) {
        let app = Self {
            core,
            outputs: Vec::new(),
        };
        // Wait for output events to create layer surfaces
        (app, cosmic::iced::Task::none())
    }

    fn view(&self) -> cosmic::Element<'_, Self::Message> {
        cosmic::widget::text("").into()
    }

    fn view_window(&self, id: window::Id) -> cosmic::Element<'_, Self::Message> {
        // Check if this window ID belongs to one of our output surfaces
        if self.outputs.iter().any(|o| o.surface_id == id) {
            cosmic::widget::container(
                text::Text::new("Test Overlay")
                    .size(24)
                    .class(cosmic::theme::Text::Color(cosmic::iced::Color::WHITE)),
            )
            .padding(10)
            .into()
        } else {
            cosmic::widget::text("").into()
        }
    }

    fn update(&mut self, message: Self::Message) -> cosmic::iced::Task<cosmic::Action<Self::Message>> {
        match message {
            Message::OutputEvent(event, wl_output) => match event {
                OutputEvent::Created(info_opt) => {
                    let name = info_opt.and_then(|i| i.name);
                    log::info!("Output created: {:?}", name);
                    
                    // Create a new layer surface for this output
                    let surface_id = window::Id::unique();
                    self.outputs.push(OutputState {
                        output: wl_output.clone(),
                        surface_id,
                        name,
                    });
                    
                    return create_layer_surface_for_output(&wl_output, surface_id);
                }
                OutputEvent::Removed => {
                    // Find and remove the output, destroy its layer surface
                    if let Some(idx) = self.outputs.iter().position(|o| o.output == wl_output) {
                        let removed = self.outputs.remove(idx);
                        log::info!("Output removed: {:?}", removed.name);
                        return destroy_layer_surface(removed.surface_id);
                    }
                }
                OutputEvent::InfoUpdate(info) => {
                    if let Some(output_state) = self.outputs.iter_mut().find(|o| o.output == wl_output) {
                        output_state.name = info.name;
                    }
                }
            },
        }
        cosmic::iced::Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        listen_with(|event, _, _| {
            if let cosmic::iced_core::Event::PlatformSpecific(
                cosmic::iced_core::event::PlatformSpecific::Wayland(
                    cosmic::iced_core::event::wayland::Event::Output(output_event, wl_output),
                ),
            ) = event
            {
                Some(Message::OutputEvent(output_event, wl_output))
            } else {
                None
            }
        })
    }
}
