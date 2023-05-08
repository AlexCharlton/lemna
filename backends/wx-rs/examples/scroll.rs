use lemna::{self, *};
use simplelog::*;

#[derive(Debug, Default)]
pub struct HelloApp {}

impl lemna::Component for HelloApp {
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
                        Div::new().bg([1.0, 0.0, 0.0]),
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
                            Div::new().bg([1.0, 0.0, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                            0
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 0.5, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                            1
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 1.0, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                            2
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 1.0, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                            3
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 0.0, 1.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                            4
                        )),
                    )
                    .push(node!(
                        Div::new().bg([1.0, 0.5, 0.0]),
                        lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                        2
                    ))
                    .push(node!(
                        Div::new().bg([1.0, 1.0, 0.0]),
                        lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                        3
                    ))
                    .push(node!(
                        Div::new().bg([0.0, 1.0, 0.0]),
                        lay!(margin: rect!(5.0), size: size!(Auto, 50.0)),
                        4
                    ))
                    .push(node!(
                        Div::new().bg([0.0, 0.0, 1.0]),
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
                            Div::new().bg([1.0, 0.0, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            0
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 0.5, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            1
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 1.0, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            2
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 1.0, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            3
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 0.0, 1.0]),
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
                            Div::new().bg([1.0, 0.0, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            0
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 0.5, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            1
                        ))
                        .push(node!(
                            Div::new().bg([1.0, 1.0, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            2
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 1.0, 0.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            3
                        ))
                        .push(node!(
                            Div::new().bg([0.0, 0.0, 1.0]),
                            lay!(margin: rect!(5.0), size: size!(Auto, 80.0)),
                            4
                        )),
                    ),
                ),
        )
    }
}

fn main() {
    let _ = WriteLogger::init(
        LevelFilter::Info,
        ConfigBuilder::new().build(),
        std::fs::File::create("example.log").unwrap(),
    );

    lemna_wx_rs::Window::<lemna::render::wgpu::WGPURenderer, HelloApp>::open_blocking(
        "Hello scroll!",
        800,
        600,
        vec![("noto sans regular".to_string(), ttf_noto_sans::REGULAR)],
    );
}
