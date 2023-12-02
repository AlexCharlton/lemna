use lemna::{widgets::*, *};

#[derive(Debug, Default)]
pub struct App {}

impl Component for App {
    fn view(&self) -> Option<Node> {
        Some(node!(
            Div::new().bg(0xFF00FFFF),
            [size: [100, 100]]
        ))
    }
}

fn main() {
    lemna_baseview::Window::open_blocking::<App>(lemna_baseview::WindowOptions::new(
        "Hello",
        (400, 300),
    ));
}
