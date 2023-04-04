use std::cell::UnsafeCell;

use simplelog::*;

use lemna::{self, layout::*, widgets::*, UI, *};
use wx_rs;

type Renderer = lemna::render::wgpu::WGPURenderer;
type Node = lemna::Node<Renderer>;

#[derive(Debug)]
pub struct HelloApp {}

impl lemna::Component<Renderer> for HelloApp {
    fn view(&self) -> Option<Node> {
        Some(
            node!(Div::new(), lay!(wrap: true))
                .push(
                    node!(
                        Div::new()
                            .bg(Color::rgb(0.9, 0.9, 0.9))
                            .scroll(ScrollDescriptor {
                                scroll_y: true,
                                ..Default::default()
                            }),
                        lay!(
                            size: size!(100.0, 200.0),
                            padding: rect!(10.0),
                            margin: rect!(10.0),
                            direction: Direction::Column,
                            axis_alignment: Alignment::Stretch,
                            cross_alignment: Alignment::Stretch,
                        ),
                        0
                    )
                    .push(node!(
                        Div::new().bg([1.0, 0.0, 0.0].into()),
                        lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                        0
                    ))
                    .push(
                        node!(
                            Div::new()
                                .bg(Color::rgb(0.8, 0.8, 0.8))
                                .scroll(ScrollDescriptor {
                                    scroll_y: true,
                                    ..Default::default()
                                }),
                            lay!(
                                size: size!(70.0, 200.0),
                                margin: rect!(5.0),
                                direction: Direction::Column,
                                axis_alignment: Alignment::Stretch,
                                cross_alignment: Alignment::Stretch,
                            ),
                            1
                        )
                        .push(node!(
                            Div::new().bg([1.0, 0.0, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                            0
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 0.5, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                            1
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 1.0, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                            2
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 1.0, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                            3
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 0.0, 1.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                            4
                        )),
                    )
                    .push(node!(
                        Div::new().bg([1.0, 0.5, 0.0].into()),
                        lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                        2
                    ))
                    .push(node!(
                        Div::new().bg([1.0, 1.0, 0.0].into()),
                        lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                        3
                    ))
                    .push(node!(
                        Div::new().bg([0.0, 1.0, 0.0].into()),
                        lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                        4
                    ))
                    .push(node!(
                        Div::new().bg([0.0, 0.0, 1.0].into()),
                        lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                        5
                    )),
                )
                .push(
                    node!(
                        Div::new()
                            .bg(Color::rgb(0.9, 0.9, 0.9))
                            .scroll(ScrollDescriptor {
                                scroll_y: true,
                                scroll_x: true,
                                y_bar_position: HorizontalPosition::Left,
                                ..Default::default()
                            }),
                        lay!(
                            size: size!(160.0, 300.0),
                            padding: rect!(10.0),
                            margin: rect!(10.0),
                            direction: Direction::Row,
                        ),
                        1
                    )
                    .push(
                        node!(
                            Div::new(),
                            lay!(
                                direction: Direction::Column,
                                size: size!(100.0, Auto),
                                axis_alignment: Alignment::Stretch,
                                cross_alignment: Alignment::Stretch,
                            ),
                            0
                        )
                        .push(node!(
                            Div::new().bg([1.0, 0.0, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            0
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 0.5, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            1
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 1.0, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            2
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 1.0, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            3
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 0.0, 1.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            4
                        )),
                    )
                    .push(
                        node!(
                            Div::new(),
                            lay!(
                                direction: Direction::Column,
                                size: size!(100.0, Auto),
                                axis_alignment: Alignment::Stretch,
                                cross_alignment: Alignment::Stretch,
                            ),
                            1
                        )
                        .push(node!(
                            Div::new().bg([1.0, 0.0, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            0
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 0.5, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            1
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 1.0, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            2
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 1.0, 0.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            3
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 0.0, 1.0].into()),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            4
                        )),
                    ),
                ),
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
    let _ = WriteLogger::init(
        LevelFilter::Info,
        ConfigBuilder::new().build(),
        std::fs::File::create("example.log").unwrap(),
    );

    wx_rs::init_app("Hello!", 800, 600);
    ui().add_font("noto sans regular", ttf_noto_sans::REGULAR);
    wx_rs::set_render(render);
    wx_rs::bind_canvas_events(handle_event);
    wx_rs::run_app();
}
