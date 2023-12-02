use lazy_static::lazy_static;
use lemna::*;
use png;

lazy_static! {
    static ref IMAGE: (Vec<u8>, usize, PixelSize) = {
        let decoder = png::Decoder::new(&include_bytes!("./icon_512x512@2x.png")[..]);
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();

        (
            buf, // This buffer is longer than the actual image data
            info.buffer_size(), // So we return the length of the data as well
            PixelSize {
                width: info.width,
                height: info.height,
            },
        )
    };
}

#[derive(Debug, Default)]
pub struct App {}

impl lemna::Component for App {
    fn view(&self) -> Option<Node> {
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
            .push(node!(widgets::Canvas::new()
                .init_with_color(
                    Color::WHITE,
                    PixelSize {
                        width: 500,
                        height: 500
                    }
                )
                .on_draw(Box::new(|p| vec![(p, Color::BLACK.into())])))),
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
