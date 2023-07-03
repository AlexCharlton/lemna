use lemna::*;
use lemna_baseview::Window;
use png;

#[derive(Debug, Default)]
pub struct App {}

impl lemna::Component for App {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new(),
                lay![size_pct: [100.0],
                     wrap: true,
                     padding: [10.0],
                     axis_alignment: Center,
                     cross_alignment: Alignment::Center,
                ]
            )
            .push(node!(
                widgets::Canvas::new()//.set(//TODO)
            )),
        )
    }
}

fn main() {
    println!("hello");
    Window::open_blocking::<lemna::render::wgpu::WGPURenderer, App>(
        "An Image".to_string(),
        600,
        600,
        true,
        baseview::WindowScalePolicy::SystemScaleFactor,
        vec![],
    );
    println!("bye");
}
