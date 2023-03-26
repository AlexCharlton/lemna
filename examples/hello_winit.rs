//  cargo run --example hello_winit --features backend_winit

#[allow(unused_imports)]
use lemna::{
    self,
    input::{Button, Input, Motion, MouseButton},
    layout::*,
    widgets::*,
    UI, *,
};

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
                            size: size!(100.0, 300.0),
                            padding: rect!(10.0),
                            margin: rect!(10.0),
                            direction: Direction::Column,
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
                            lay!(direction: Direction::Column, size: size!(100.0, Auto)),
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
                            lay!(direction: Direction::Column, size: size!(100.0, Auto)),
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

#[cfg(not(feature = "backend_winit"))]
fn main() {}

#[cfg(feature = "backend_winit")]
fn main() {
    use lemna::instrumenting::*;
    use simplelog::*;
    use winit::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };

    type HelloUI = UI<winit::window::Window, Renderer, HelloApp>;

    let _ = WriteLogger::init(
        LevelFilter::Info,
        ConfigBuilder::new().build(),
        std::fs::File::create("example.log").unwrap(),
    );
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Hello!")
        .build(&event_loop)
        .unwrap();
    let mut ui: HelloUI = UI::new(window);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        inst(&format!("event_handler <{:?}>", &event));

        match event {
            Event::MainEventsCleared => {
                ui.draw();
            }
            Event::RedrawRequested(_) => ui.render(),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::CursorMoved { position, .. } => {
                    // println!("{:?}", position);
                    ui.handle_input(&Input::Motion(Motion::Mouse {
                        x: position.x as f32,
                        y: position.y as f32,
                    }));
                }
                WindowEvent::MouseInput {
                    button,
                    state: winit::event::ElementState::Pressed,
                    ..
                } => {
                    ui.handle_input(&Input::Press(Button::Mouse(MouseButton::Left)));
                }
                WindowEvent::MouseInput {
                    button,
                    state: winit::event::ElementState::Released,
                    ..
                } => {
                    ui.handle_input(&Input::Press(Button::Mouse(MouseButton::Left)));
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    // println!("scroll delta{:?}", delta);
                    let scroll = match delta {
                        winit::event::MouseScrollDelta::LineDelta(x, y) => Motion::Scroll {
                            x: x * -10.0,
                            y: y * -10.0,
                        },
                        winit::event::MouseScrollDelta::PixelDelta(
                            winit::dpi::LogicalPosition { x, y },
                        ) => Motion::Scroll {
                            x: -x as f32,
                            y: -y as f32,
                        },
                    };
                    ui.handle_input(&Input::Motion(scroll));
                }
                _ => (),
            },
            _ => (),
        };

        inst_end();
    });
}
