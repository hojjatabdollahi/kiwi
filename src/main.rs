use cosmic::app::Core;
use cosmic::iced::{window, Limits};
use cosmic::iced_runtime::platform_specific::wayland::layer_surface::{
    IcedOutput, SctkLayerSurfaceSettings,
};
use cosmic::iced_winit::commands::layer_surface::get_layer_surface;
use cosmic::widget::text;
use cosmic_client_toolkit::sctk::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};

fn main() -> cosmic::iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let settings = cosmic::app::Settings::default()
        .no_main_window(true)
        .exit_on_close(false);
    cosmic::app::run::<Kiwi>(settings, ())
}

struct Kiwi {
    core: Core,
    overlay_id: window::Id,
}

#[derive(Debug, Clone)]
enum Message {}

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
        let overlay_id = window::Id::unique();

        let app = Self { core, overlay_id };

        // Create the layer surface overlay
        let layer_surface_cmd = get_layer_surface(SctkLayerSurfaceSettings {
            id: overlay_id,
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::None,
            anchor: Anchor::TOP | Anchor::RIGHT,
            output: IcedOutput::Active,
            namespace: "kiwi".to_string(),
            size: Some((Some(300), Some(60))),
            exclusive_zone: -1,
            size_limits: Limits::NONE.min_width(1.0).min_height(1.0),
            ..Default::default()
        });

        (app, layer_surface_cmd)
    }

    fn view(&self) -> cosmic::Element<'_, Self::Message> {
        // This won't be called since we use no_main_window(true)
        cosmic::widget::text("").into()
    }

    fn view_window(&self, id: window::Id) -> cosmic::Element<'_, Self::Message> {
        if id == self.overlay_id {
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

    fn update(
        &mut self,
        _message: Self::Message,
    ) -> cosmic::iced::Task<cosmic::Action<Self::Message>> {
        cosmic::iced::Task::none()
    }
}
