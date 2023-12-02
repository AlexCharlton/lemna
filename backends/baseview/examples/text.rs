use lemna::*;
use lemna_baseview::Window;
use ttf_noto_sans;

#[derive(Debug, Default)]
pub struct App {}

impl lemna::Component for App {
    fn init(&mut self) {
        let dark_blue: Color = [0.0, 0.0, 0.3].into();
        style::set_current_style(style!(
            Text.color = dark_blue;
        ));
    }

    fn view(&self) -> Option<Node> {
        Some(node!(widgets::Div::new().bg(0.7),
                   [size_pct: [100, Auto]])
             .push(node!(widgets::Text::new(
                 txt!("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.")),
                         [size_pct: [100.0, Auto], margin: [10]])))
    }
}

fn main() {
    println!("hello");
    Window::open_blocking::<App>(
        "Hello text".to_string(),
        400,
        300,
        true,
        baseview::WindowScalePolicy::SystemScaleFactor,
        vec![("noto sans regular".to_string(), ttf_noto_sans::REGULAR)],
    );
    println!("bye");
}
