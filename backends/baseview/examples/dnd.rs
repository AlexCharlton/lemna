use lemna::*;
use lemna_baseview::Window;

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
            .push(node!(DropTarget {}, lay!(size: size!(100.0)), 0))
            .push(node!(DragSource {}, lay!(size: size!(100.0)), 0)),
        )
    }
}

impl lemna::App<Renderer> for HelloApp {
    fn new() -> Self {
        Self {}
    }
}

#[derive(Debug)]
pub struct DropTarget {}

impl Component<Renderer> for DropTarget {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new()
                    .bg(Color::rgb(0.5, 1.0, 0.5))
                    .border(Color::BLACK, 2.0),
                lay!(
                    size: size_pct!(100.0),
                    margin: rect!(10.0),
                    padding: rect!(5.0),
                    cross_alignment: crate::layout::Alignment::Center,
                    axis_alignment: crate::layout::Alignment::Center
                ),
                0
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

    // fn on_text_entry(&mut self, event: &mut Event<event::TextEntry>) -> Vec<Message> {
    //     println!("{} got a some text: {:?})", &self.name, event.input.0);
    //     vec![]
    // }
}

#[derive(Debug)]
pub struct DragSource {}

impl Component<Renderer> for DragSource {
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
                0
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

    fn on_drag_start(&mut self, event: &mut Event<event::DragStart>) -> Vec<Message> {
        current_window()
            .unwrap()
            .start_drag(Data::Filepath("/foo".into()));
        event.stop_bubbling();
        vec![]
    }
}

fn main() {
    println!("hello");
    Window::open_blocking::<Renderer, HelloApp>(
        "Hello".to_string(),
        400,
        300,
        baseview::WindowScalePolicy::SystemScaleFactor,
        vec![("noto sans regular".to_string(), ttf_noto_sans::REGULAR)],
    );

    println!("bye");
}
