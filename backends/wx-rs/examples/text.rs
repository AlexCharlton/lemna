use std::cell::UnsafeCell;

use lemna::{self, lay, node, rect, size_pct, txt, widgets, UI};
use ttf_noto_sans;
use wx_rs;

type Renderer = lemna::render::wgpu::WGPURenderer;
type Node = lemna::Node<Renderer>;

#[derive(Debug)]
pub struct HelloApp {}

impl lemna::Component<Renderer> for HelloApp {
    fn view(&self) -> Option<Node> {
        Some(node!(widgets::Div::new().bg(0.5.into()),
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

type HelloUI = UI<wx_rs::Window, Renderer, HelloApp>;

thread_local!(
    pub static UI: UnsafeCell<HelloUI> = {
        UnsafeCell::new(UI::new(wx_rs::Window::new()))
    }
);

pub fn ui() -> &'static mut HelloUI {
    UI.with(|r| unsafe { r.get().as_mut().unwrap() })
}

extern "C" fn render() {
    if ui().draw() {
        ui().render();
    }
}

use std::os::raw::c_void;
extern "C" fn handle_event(event: *const c_void) {
    for input in lemna_wx_rs::event_to_input(event).iter() {
        ui().handle_input(input);
        if input != &lemna::input::Input::Timer {
            wx_rs::set_status_text(&format!("Got input: {:?}", input));
        }
    }
}

fn main() {
    println!("hello");
    wx_rs::init_app("Hello!", 400, 300);
    ui().add_font("noto sans regular", ttf_noto_sans::REGULAR);
    wx_rs::set_render(render);
    wx_rs::bind_canvas_events(handle_event);

    wx_rs::run_app();

    println!("bye");
}
