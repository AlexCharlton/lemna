use lemna::{self, lay, node, rect, size_pct, txt, widgets};
use ttf_noto_sans;

type Renderer = lemna::render::wgpu::WGPURenderer;
type Node = lemna::Node<Renderer>;

#[derive(Debug)]
pub struct HelloApp {}

impl lemna::Component<Renderer> for HelloApp {
    fn view(&self) -> Option<Node> {
        Some(node!(widgets::Div::new().bg(0.5),
                   lay!(size: size_pct!(100.0, Auto)))
             .push(node!(widgets::Text::new(
                 txt!("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."),
                 widgets::TextStyle::default()),
                         lay!(margin: rect!(10.0)))))
    }
}

impl lemna::App<Renderer> for HelloApp {
    fn new() -> Self {
        Self {}
    }
}

fn main() {
    println!("hello");
    lemna_wx_rs::Window::<Renderer, HelloApp>::open_blocking(
        "Hello events!",
        400,
        300,
        vec![("noto sans regular".to_string(), ttf_noto_sans::REGULAR)],
    );
    println!("bye");
}
