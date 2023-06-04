use lemna::*;

#[derive(Debug, Default)]
pub struct App {}

impl lemna::Component for App {
    fn view(&self) -> Option<Node> {
        Some(
            node!(Div::new(), [wrap: true])
                .push(
                    node!(
                        Div::new().bg(Color::rgb(0.9, 0.9, 0.9)).scroll_y(),
                        [
                            size: [100, 200],
                            padding: [10],
                            margin: [10],
                            direction: Column,
                            axis_alignment: Stretch,
                            cross_alignment: Stretch,
                        ],
                    )
                    .push(node!(
                        Div::new().bg([1.0, 0.0, 0.0]),
                        [margin: [5], size: [Auto, 50]],
                    ))
                    .push(
                        node!(
                            Div::new().bg(Color::rgb(0.8, 0.8, 0.8)).scroll_y(),
                            [
                                size: [70, 200],
                                margin: [5],
                                direction: Column,
                                axis_alignment: Stretch,
                                cross_alignment: Stretch,
                            ],
                        )
                        .push(node!(
                            Div::new().bg([1.0, 0.0, 0.0]),
                            [margin: [5], size: [Auto, 50]],
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 0.5, 0.0]),
                            [margin: [5], size: [Auto, 50]],
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 1.0, 0.0]),
                            [margin: [5], size: [Auto, 50]],
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 1.0, 0.0]),
                            [margin: [5], size: [Auto, 50]],
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 0.0, 1.0]),
                            [margin: [5], size: [Auto, 50]],
                        )),
                    )
                    .push(node!(
                        Div::new().bg([1.0, 0.5, 0.0]),
                        [margin: [5], size: [Auto, 50]],
                    ))
                    .push(node!(
                        Div::new().bg([1.0, 1.0, 0.0]),
                        [margin: [5], size: [Auto, 50]],
                    ))
                    .push(node!(
                        Div::new().bg([0.0, 1.0, 0.0]),
                        [margin: [5], size: [Auto, 50]],
                    ))
                    .push(node!(
                        Div::new().bg([0.0, 0.0, 1.0]),
                        [margin: [5], size: [Auto, 50]],
                    )),
                )
                .push(
                    node!(
                        Div::new()
                            .bg(Color::rgb(0.9, 0.9, 0.9))
                            .scroll_x()
                            .scroll_y()
                            .style("y_bar_position", HorizontalPosition::Left.into()),
                        [
                            size: [160, 300],
                            padding: [10],
                            margin: [10],
                            direction: Row,
                        ],
                    )
                    .push(
                        node!(
                            Div::new(),
                            [
                                direction: Column,
                                size: [100, Auto],
                                axis_alignment: Stretch,
                                cross_alignment: Stretch,
                            ],
                        )
                        .push(node!(
                            Div::new().bg([1.0, 0.0, 0.0]),
                            [margin: [5], size: [Auto, 80]],
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 0.5, 0.0]),
                            [margin: [5], size: [Auto, 80]],
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 1.0, 0.0]),
                            [margin: [5], size: [Auto, 80]],
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 1.0, 0.0]),
                            [margin: [5], size: [Auto, 80]],
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 0.0, 1.0]),
                            [margin: [5], size: [Auto, 80]],
                        )),
                    )
                    .push(
                        node!(
                            Div::new(),
                            [
                                direction: Column,
                                size: [100, Auto],
                                axis_alignment: Stretch,
                                cross_alignment: Stretch,
                            ],
                        )
                        .push(node!(
                            Div::new().bg([1.0, 0.0, 0.0]),
                            [margin: [5], size: [Auto, 80]],
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 0.5, 0.0]),
                            [margin: [5], size: [Auto, 80]],
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 1.0, 0.0]),
                            [margin: [5], size: [Auto, 80]],
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 1.0, 0.0]),
                            [margin: [5], size: [Auto, 80]],
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 0.0, 1.0]),
                            [margin: [5], size: [Auto, 80]],
                        )),
                    ),
                ),
        )
    }
}

fn main() {
    use simplelog::*;
    println!("hello");

    let _ = WriteLogger::init(
        LevelFilter::Info,
        ConfigBuilder::new().build(),
        std::fs::File::create("example.log").unwrap(),
    );
    lemna_winit::Window::open_blocking::<lemna::render::wgpu::WGPURenderer, App>(
        "Hello scroll!",
        800,
        600,
        vec![],
    );

    println!("bye");
}
