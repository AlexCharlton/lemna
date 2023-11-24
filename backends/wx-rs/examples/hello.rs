use lemna::{self, layout::*, widgets, *};

#[derive(Debug, Default)]
pub struct App {}

impl lemna::Component for App {
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
            ))
            .push(node!(
                widgets::Div::new().bg(Color::rgb(0.0, 1.0, 0.0)),
                lay!(size: size!(100.0), margin: rect!(5.0)),
            ))
            .push(node!(
                widgets::RoundedRect {
                    background_color: [0.0, 0.0, 1.0].into(),
                    border_width: 1.0,
                    ..Default::default()
                }
                .radius(5.0),
                lay!(size: size!(100.0), margin: rect!(5.0)),
            )),
        )
    }
}

fn main() {
    println!("hello");
    lemna_wx_rs::Window::<App>::open_blocking("Hello!", 400, 300, vec![]);
    println!("bye");
}
