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
                     axis_alignment: Alignment::Center,
                     cross_alignment: Alignment::Center,
                )
            )
            .push(node!(
                widgets::Div::new().bg(Color::rgb(1.0, 0.5, 0.5)),
                [size: size!(200.0, 100.0), margin: rect!(5.0)],
            ))
            .push(node!(
                widgets::Div::new().bg(Color::rgb(0.5, 1.0, 0.5)),
                [size: size!(100.0), margin: rect!(5.0)],
            ))
            .push(node!(
                widgets::RoundedRect {
                    background_color: [0.5, 0.5, 1.0].into(),
                    border_width: 1.0,
                    ..Default::default()
                }
                .radius(5.0),
                [size: size!(100.0), margin: rect!(5.0)]
            )),
        )
    }
}

fn main() {
    println!("hello");
    Window::open_blocking::<lemna::render::wgpu::WGPURenderer, HelloApp>(
        "Hello".to_string(),
        400,
        300,
        true,
        baseview::WindowScalePolicy::SystemScaleFactor,
        vec![],
    );

    println!("bye");
}
