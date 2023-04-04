use std::cell::UnsafeCell;

use lemna::{self, widgets, UI, *};
use wx_rs;

type Renderer = lemna::render::wgpu::WGPURenderer;
type Node = lemna::Node<Renderer>;

#[derive(Debug)]
pub struct HelloApp {}

impl lemna::Component<Renderer> for HelloApp {
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
                0
            ))
            .push(node!(
                widgets::Div::new().bg(Color::rgb(0.0, 1.0, 0.0)),
                lay!(size: size!(100.0), margin: rect!(5.0)),
                1
            ))
            .push(node!(
                widgets::RoundedRect {
                    background_color: [0.0, 0.0, 1.0].into(),
                    border_width: 1.0,
                    ..Default::default()
                }
                .radius(5.0),
                lay!(size: size!(100.0), margin: rect!(5.0)),
                2
            )),
        )
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
    wx_rs::set_render(render);
    wx_rs::bind_canvas_events(handle_event);

    wx_rs::run_app();

    println!("bye");
}
