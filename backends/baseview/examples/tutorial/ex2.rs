use lemna::{widgets::*, *};

#[derive(Debug)]
pub struct BlueBorder {}
impl Component for BlueBorder {
    fn view(&self) -> Option<Node> {
        Some(node!(
            Div::new().bg(Color::BLUE),
            [padding: [10]]
        ))
    }

    fn container(&self) -> Option<Vec<usize>> {
        Some(vec![0])
    }
}

#[derive(Debug, Default)]
pub struct App {}
impl Component for App {
    fn view(&self) -> Option<Node> {
        Some(
            node!(Div::new())
                .push(node!(BlueBorder {}).push(node!(Div::new().bg(Color::RED), [size: [100]])))
                .push(node!(BlueBorder {}).push(node!(Div::new().bg(Color::GREEN), [size: [100]]))),
        )
    }
}

fn main() {
    lemna_baseview::Window::open_blocking::<App>(lemna_baseview::WindowOptions::new(
        "Hello",
        (400, 300),
    ));
}
