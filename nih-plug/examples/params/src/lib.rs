use lemna::{self, widgets, *};
use lemna_macros::{state_component, state_component_impl};
use lemna_nih_plug::nih_plug;
use nih_plug::prelude::*;
use std::sync::Arc;

type Renderer = lemna::render::wgpu::WGPURenderer;
type Node = lemna::Node<Renderer>;

#[derive(Debug)]
pub struct ParamsAppState {
    params: Arc<ParamsParams>,
}

#[state_component(ParamsAppState)]
#[derive(Debug)]
pub struct ParamsApp {}

#[state_component_impl(ParamsAppState)]
impl lemna::Component<Renderer> for ParamsApp {
    fn init(&mut self) {
        self.state = Some(ParamsAppState {
            params: Default::default(),
        })
    }

    fn view(&self) -> Option<Node> {
        Some(node!(
            widgets::Div::new().bg(Color::rgb(
                self.state_ref().params.red.value(),
                self.state_ref().params.green.value(),
                self.state_ref().params.blue.value()
            )),
            lay!(size: size_pct!(100.0))
        ))
    }
}

impl lemna::App<Renderer> for ParamsApp {
    fn new() -> Self {
        Self { state: None }
    }
}

#[derive(Default)]
pub struct ParamsPlugin {
    params: Arc<ParamsParams>,
}

#[derive(Params, Debug)]
struct ParamsParams {
    #[id = "red"]
    pub red: FloatParam,
    #[id = "green"]
    pub green: FloatParam,
    #[id = "blue"]
    pub blue: FloatParam,
}

impl Default for ParamsParams {
    fn default() -> Self {
        Self {
            red: FloatParam::new("Red", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            green: FloatParam::new("Green", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
            blue: FloatParam::new("Blue", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 }),
        }
    }
}

impl Plugin for ParamsPlugin {
    const NAME: &'static str = "Hello Lemna Params";
    const VENDOR: &'static str = "ANC";
    const URL: &'static str = "https://github.com/AlexCharlton/lemna";
    const EMAIL: &'static str = "alex.n.charlton@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // You need to have a audio or a midi output or else no processing will happen
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;

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
        let app_params = self.params.clone();
        lemna_nih_plug::create_lemna_editor::<Renderer, ParamsApp, _>(
            "Hello Lemna Params",
            400,
            300,
            vec![],
            move |_ctx, ui| {
                ui.with_app_state::<ParamsAppState, _>(|s| s.params = app_params.clone());
            },
        )
    }
}

impl ClapPlugin for ParamsPlugin {
    const CLAP_ID: &'static str = "anc.lemna.examples.hello";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Example plugin for Lemna");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Utility];
}

impl Vst3Plugin for ParamsPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"ANC-Params-Lemna";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Tools];
}

nih_export_clap!(ParamsPlugin);
nih_export_vst3!(ParamsPlugin);
