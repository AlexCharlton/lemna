use lemna::{self, widgets, *};
use lemna_nih_plug::nih_plug;
use nih_plug::prelude::*;
use std::sync::Arc;

type Renderer = lemna::render::wgpu::WGPURenderer;
type Node = lemna::Node<Renderer>;

#[derive(Debug)]
pub struct HelloApp {}

impl lemna::Component<Renderer> for HelloApp {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new(),
                lay!(size: size_pct!(100.0), wrap: true,
                     padding: rect!(10.0),
                     axis_alignment: Alignment::Center, cross_alignment: Alignment::Center)
            )
            .push(node!(
                widgets::Div::new().bg(Color::rgb(1.0, 0.0, 0.0)),
                lay!(size: size!(200.0, 100.0), margin: rect!(5.0)),
                0
            ))
            .push(node!(
                widgets::Div::new().bg(Color::rgb(0.0, 1.0, 0.0)),
                lay!(size: size!(100.0), margin: rect!(5.0)),
                1
            ))
            .push(node!(
                widgets::RoundedRect {
                    background_color: [0.0, 0.0, 1.0].into(),
                    border_width: 1.0,
                    ..Default::default()
                }
                .radius(5.0),
                lay!(size: size!(100.0), margin: rect!(5.0)),
                2
            )),
        )
    }

    fn on_mouse_enter(&mut self, _event: &mut event::Event<event::MouseEnter>) -> Vec<Message> {
        if let Some(w) = crate::current_window() {
            w.set_cursor("Cross");
        }
        vec![]
    }
}

impl lemna::App<Renderer> for HelloApp {
    fn new() -> Self {
        Self {}
    }
}

#[derive(Default)]
pub struct HelloPlugin {
    params: Arc<HelloParams>,
}

#[derive(Params, Default)]
struct HelloParams {}

impl Plugin for HelloPlugin {
    const NAME: &'static str = "Hello Lemna";
    const VENDOR: &'static str = "ANC";
    const URL: &'static str = "https://github.com/AlexCharlton/lemna";
    const EMAIL: &'static str = "alex.n.charlton@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[];
    const MIDI_INPUT: MidiConfig = MidiConfig::None;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        ProcessStatus::Normal
    }

    fn editor(&self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        lemna_nih_plug::create_lemna_editor::<Renderer, HelloApp, _, _>(
            "Hello Lemna",
            400,
            300,
            vec![],
            |_ctx, _ui| {},
            || vec![],
        )
    }
}

impl ClapPlugin for HelloPlugin {
    const CLAP_ID: &'static str = "anc.lemna.examples.hello";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Example plugin for Lemna");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Utility];
}

impl Vst3Plugin for HelloPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"ANC-Hello-Lemna-";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Tools];
}

nih_export_clap!(HelloPlugin);
nih_export_vst3!(HelloPlugin);
