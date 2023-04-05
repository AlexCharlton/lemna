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
}

impl lemna::App<Renderer> for HelloApp {
    fn new() -> Self {
        Self {}
    }
}

fn main() {
    println!("hello");
    Window::open_blocking::<Renderer, HelloApp>(
        "Hello".to_string(),
        400,
        300,
        baseview::WindowScalePolicy::SystemScaleFactor,
        vec![],
    );

    println!("bye");
}
