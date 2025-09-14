use lemna::{style::HorizontalPosition, *};

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
        println!("Oops, you missed the target. Got {:?}", event.input.0);
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
            dirty: false,
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
        println!("Got {:?}", event.input.0);
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

fn main() {
    println!("hello");
    lemna_baseview::Window::open_blocking::<App>(
        lemna_baseview::WindowOptions::new("Hello DND", (400, 300))
            .resizable(false)
            .fonts(vec![(
                "noto sans regular".to_string(),
                ttf_noto_sans::REGULAR,
            )]),
    );
    println!("bye");
}
