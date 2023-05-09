use lemna::*;
use lemna_baseview::Window;

#[derive(Debug, Default)]
pub struct HelloApp {}

impl lemna::Component for HelloApp {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new(),
                lay!(size: size_pct!(100.0), wrap: true,
                     padding: rect!(10.0),
                     axis_alignment: Alignment::Center, cross_alignment: Alignment::Center)
            )
            .push(node!(DropTarget::new(), lay!(size: size!(100.0))))
            .push(node!(DragSource {}, lay!(size: size!(100.0)))),
        )
    }

    fn on_drag_drop(&mut self, event: &mut Event<event::DragDrop>) {
        // This will never print, because this is not a valid target per `on_drag_target`
        println!("Oops, you missed the target. Got {:?}", event.input.0);
    }

    fn on_drag_target(&mut self, _event: &mut Event<event::DragTarget>) {
        current_window().unwrap().set_drop_target_valid(false);
    }
}

#[derive(Debug, Default)]
pub struct DropTargetState {
    active: bool,
}

#[state_component(DropTargetState)]
#[derive(Debug)]
pub struct DropTarget {}

impl DropTarget {
    fn new() -> Self {
        Self {
            state: Some(DropTargetState::default()),
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
                lay!(
                    size: size_pct!(100.0),
                    margin: rect!(10.0),
                    padding: rect!(5.0),
                    cross_alignment: crate::layout::Alignment::Center,
                    axis_alignment: crate::layout::Alignment::Center
                ),
            )
            .push(node!(widgets::Text::new(
                txt!("Drag something onto me"),
                widgets::TextStyle {
                    h_alignment: HorizontalAlign::Center,
                    ..widgets::TextStyle::default()
                }
            ))),
        )
    }

    fn on_drag_drop(&mut self, event: &mut Event<event::DragDrop>) {
        println!("Got {:?}", event.input.0);
        self.state_mut().active = false;
        event.dirty();
    }

    fn on_drag_enter(&mut self, event: &mut Event<event::DragEnter>) {
        self.state_mut().active = true;
        current_window().unwrap().set_drop_target_valid(true);
        event.dirty();
    }

    fn on_drag_leave(&mut self, event: &mut Event<event::DragLeave>) {
        self.state_mut().active = false;
        current_window().unwrap().set_drop_target_valid(false);
        event.dirty();
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
                lay!(
                    size: size_pct!(100.0),
                    margin: rect!(10.0),
                    padding: rect!(5.0),
                    cross_alignment: crate::layout::Alignment::Center,
                    axis_alignment: crate::layout::Alignment::Center
                ),
            )
            .push(node!(widgets::Text::new(
                txt!("Drag from me"),
                widgets::TextStyle {
                    h_alignment: HorizontalAlign::Center,
                    ..widgets::TextStyle::default()
                }
            ))),
        )
    }

    fn on_drag_start(&mut self, event: &mut Event<event::DragStart>) {
        current_window()
            .unwrap()
            .start_drag(Data::Filepath("/test/file.txt".into()));
        event.stop_bubbling();
    }
}

fn main() {
    println!("hello");
    Window::open_blocking::<lemna::render::wgpu::WGPURenderer, HelloApp>(
        "Hello".to_string(),
        400,
        300,
        false,
        baseview::WindowScalePolicy::SystemScaleFactor,
        vec![("noto sans regular".to_string(), ttf_noto_sans::REGULAR)],
    );

    println!("bye");
}
