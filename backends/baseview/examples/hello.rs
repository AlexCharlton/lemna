use lemna::*;

#[derive(Debug, Default)]
pub struct App {}

impl Component for App {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                widgets::Div::new(),
                lay![size_pct: [100.0],
                     wrap: true,
                     padding: [10.0],
                     axis_alignment: Center,
                     cross_alignment: layout::Alignment::Center,
                ]
            )
            .push(node!(
                widgets::Div::new().bg(Color::rgb(1.0, 0.5, 0.5)),
                [size: [200.0, 100.0], margin: [5],],
            ))
            .push(node!(
                widgets::Div::new().bg(Color::rgb(0.5, 1.0, 0.5)),
                [size: size!(100.0), margin: [5.0]],
            ))
            .push(node!(
                widgets::RoundedRect {
                    background_color: [0.5, 0.5, 1.0].into(),
                    border_width: 1.0,
                    ..Default::default()
                }
                .radius(5.0),
                [size: [100], margin: bounds!(5)]
            )),
        )
    }
}

fn main() {
    println!("hello");
    lemna_baseview::Window::open_blocking::<App>(lemna_baseview::WindowOptions::new(
        "Hello",
        (400, 300),
    ));
    println!("bye");
}
