use lemna::{self, style::HorizontalPosition, widgets, *};
use nice_plug::prelude::*;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct App {}

impl Component for App {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new(),
                [size_pct: [100.0], wrap: true,
                 padding: [10.0],
                 axis_alignment: Center, cross_alignment: Center]
            )
            .push(node!(DropTarget::new(), lay!(size: size!(100.0))))
            .push(node!(DragSource {}, lay!(size: size!(100.0)))),
        )
    }

    fn on_drag_drop(&mut self, event: &mut Event<event::DragDrop>) {
        // This will never print, because this is not a valid target per `on_drag_target`
        println!("Oops, you missed the target. Got {:?}", event.input.data);
    }

    fn on_drag_target(&mut self, _event: &mut Event<event::DragTarget>) {
        window::set_drop_target_valid(false);
    }
}

#[derive(Debug, Default)]
pub struct DropTargetState {
    active: bool,
}

#[component(State = "DropTargetState")]
#[derive(Debug)]
pub struct DropTarget {}

impl DropTarget {
    fn new() -> Self {
        Self {
            state: Some(DropTargetState::default()),
            dirty: lemna::Dirty::No,
        }
    }
}

#[state_component_impl(DropTargetState)]
impl Component for DropTarget {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new()
                    .bg(if self.state_ref().active {
                        Color::rgb(1.0, 0.5, 0.5)
                    } else {
                        Color::rgb(0.5, 1.0, 0.5)
                    })
                    .border(Color::BLACK, 2.0),
                [
                    size_pct: [100],
                    margin: [10],
                    padding: [5],
                    cross_alignment: Center,
                    axis_alignment: Center,
                ],
            )
            .push(node!(
                widgets::Text::new(txt!("Drag something onto me"))
                    .style("h_alignment", HorizontalPosition::Center)
            )),
        )
    }

    fn on_drag_drop(&mut self, event: &mut Event<event::DragDrop>) {
        println!("Got {:?}", event.input.data);
        self.state_mut().active = false;
    }

    fn on_drag_enter(&mut self, _event: &mut Event<event::DragEnter>) {
        self.state_mut().active = true;
        window::set_drop_target_valid(true);
    }

    fn on_drag_leave(&mut self, _event: &mut Event<event::DragLeave>) {
        self.state_mut().active = false;
        window::set_drop_target_valid(false);
    }

    fn on_drag_target(&mut self, event: &mut Event<event::DragTarget>) {
        event.stop_bubbling();
    }
}

#[derive(Debug)]
pub struct DragSource {}

impl Component for DragSource {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new()
                    .bg(Color::rgb(0.5, 0.5, 1.0))
                    .border(Color::BLACK, 2.0),
                [
                    size_pct: [100],
                    margin: [10],
                    padding: [5],
                    cross_alignment: Center,
                    axis_alignment: Center,
                ],
            )
            .push(node!(
                widgets::Text::new(txt!("Drag from me"))
                    .style("h_alignment", HorizontalPosition::Center)
            )),
        )
    }

    fn on_drag_start(&mut self, event: &mut Event<event::DragStart>) {
        window::start_drag(Data::Filepath("/test/file.txt".into()));
        event.stop_bubbling();
    }
}

#[derive(Default)]
pub struct DndPlugin {
    params: Arc<DndParams>,
}

#[derive(Params, Default)]
struct DndParams {}

impl Plugin for DndPlugin {
    const NAME: &'static str = "DND Example";
    const VENDOR: &'static str = "ANC";
    const URL: &'static str = "https://github.com/AlexCharlton/lemna";
    const EMAIL: &'static str = "alex.n.charlton@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

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

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        lemna_nice_plug::create_lemna_editor::<App, _, _>(
            lemna_nice_plug::WindowOptions::new("DND Example", (400, 300)).fonts(vec![(
                "noto sans regular".to_string(),
                ttf_noto_sans::REGULAR,
            )]),
            |_ctx, _ui| {},
            Vec::new,
        )
    }
}

impl ClapPlugin for DndPlugin {
    const CLAP_ID: &'static str = "anc.lemna.examples.dnd";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("DND Example for Lemna");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Utility];
}

impl Vst3Plugin for DndPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"ANC-DND-Example-";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Tools];
}

nice_export_clap!(DndPlugin);
nice_export_vst3!(DndPlugin);
