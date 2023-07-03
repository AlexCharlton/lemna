use lazy_static::lazy_static;
use lemna::*;
use lemna_baseview::Window;
use png;

lazy_static! {
    static ref IMAGE: (Vec<u8>, usize, PixelSize) = {
        let decoder = png::Decoder::new(&include_bytes!("./icon_512x512@2x.png")[..]);
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let slice = &buf[..];

        (
            buf,
            info.buffer_size(),
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
                widgets::Div::new(),
                lay![size_pct: [100.0],
                     wrap: true,
                     padding: [10.0],
                     axis_alignment: Center,
                     cross_alignment: Alignment::Center,
                ]
            )
            .push(node!(widgets::Canvas::new()
                .set(&IMAGE.0[..IMAGE.1], IMAGE.2)
                .scale(0.5))),
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
