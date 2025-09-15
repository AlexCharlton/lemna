use lemna::*;

#[derive(Debug, Default)]
pub struct App {}

impl lemna::Component for App {
    fn view(&self) -> Option<Node> {
        let scale_factor = lemna::window::scale_factor().unwrap();
        Some(
            node!(
                widgets::Div::new().bg([0.5, 0.7, 0.7]),
                lay![size_pct: [100.0],
                     wrap: true,
                     padding: [10.0],
                     axis_alignment: Center,
                     cross_alignment: Center,
                ]
            )
            .push(node!(
                widgets::Canvas::new()
                    .init_with_color(
                        Color::WHITE,
                        PixelSize {
                            width: (500.0 * scale_factor) as u32,
                            height: (500.0 * scale_factor) as u32,
                        }
                    )
                    .on_draw(Box::new(|p| vec![(p, Color::BLACK.into())]))
            )),
        )
    }
}

fn main() {
    println!("hello");
    lemna_baseview::Window::open_blocking::<App>(lemna_baseview::WindowOptions::new(
        "A Canvas",
        (600, 600),
    ));
    println!("bye");
}
