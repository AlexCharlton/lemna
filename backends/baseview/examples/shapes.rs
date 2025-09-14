use lemna::renderable::{Path, Renderable, Shape};
use lemna::*;

#[derive(Debug, Default)]
pub struct App {}

impl lemna::Component for App {
    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let mut path_builder = Path::builder();
        path_builder.begin(Point::new(10.0, 10.0));
        path_builder.line_to(Point::new(100.0, 10.0));
        path_builder.quad_to(Point::new(200.0, 10.0), Point::new(200.0, 100.0));
        path_builder.close();
        let path1 = path_builder.build().unwrap();

        let mut path_builder = Path::builder();
        path_builder.begin(Point::new(200.0, 200.0));
        path_builder.line_to(Point::new(100.0, 200.0));
        path_builder.cubic_to(
            Point::new(10.0, 200.0),
            Point::new(10.0, 100.0),
            Point::new(10.0, 100.0),
        );
        path_builder.close();
        let path2 = path_builder.build().unwrap();

        let mut path_builder = Path::builder();
        path_builder.begin(Point::new(230.0, 20.0));
        path_builder.cubic_to(
            Point::new(230.0, 100.0),
            Point::new(330.0, 200.0),
            Point::new(330.0, 200.0),
        );
        let path3 = path_builder.build().unwrap();

        let path4 = Path::ellipse(&Rect::new(
            Pos::new(350.0, 20.0, 0.0),
            Scale::new(130.0, 90.0),
        ))
        .unwrap();

        let shape1 = Renderable::Shape(Shape::new(
            path1,
            [1.0, 0.0, 0.0].into(),
            [0.0, 0.0, 0.0].into(),
            4.0,
            0.0,
            context.caches,
            context
                .prev_state
                .as_ref()
                .and_then(|r| r.first())
                .and_then(|r| r.as_shape()),
        ));
        let shape2 = Renderable::Shape(Shape::new(
            path2,
            [0.0, 1.0, 1.0].into(),
            Color::TRANSPARENT,
            4.0,
            0.0,
            context.caches,
            context
                .prev_state
                .as_ref()
                .and_then(|r| r.get(1))
                .and_then(|r| r.as_shape()),
        ));
        let shape3 = Renderable::Shape(Shape::new(
            path3,
            Color::TRANSPARENT,
            [0.0, 1.0, 0.0].into(),
            6.0,
            0.0,
            context.caches,
            context
                .prev_state
                .as_ref()
                .and_then(|r| r.get(2))
                .and_then(|r| r.as_shape()),
        ));
        let shape4 = Renderable::Shape(Shape::new(
            path4,
            Color::BLUE,
            Color::RED,
            4.0,
            0.0,
            context.caches,
            context
                .prev_state
                .as_ref()
                .and_then(|r| r.get(3))
                .and_then(|r| r.as_shape()),
        ));

        Some(vec![shape1, shape2, shape3, shape4])
    }
}

fn main() {
    println!("hello");
    lemna_baseview::Window::open_blocking::<App>(
        lemna_baseview::WindowOptions::new("Hello Shapes", (400, 300)).resizable(false),
    );
    println!("bye");
}
