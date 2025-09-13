use lemna::renderables::Shape;
use lemna::*;
use lyon::path::Path;
use lyon::tessellation::math as lyon_math;

#[derive(Debug, Default)]
pub struct App {}

impl lemna::Component for App {
    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let mut path_builder = Path::builder();
        path_builder.begin(lyon_math::point(10.0, 10.0));
        path_builder.line_to(lyon_math::point(100.0, 10.0));
        path_builder.quadratic_bezier_to(
            lyon_math::point(200.0, 10.0),
            lyon_math::point(200.0, 100.0),
        );
        path_builder.close();
        let path1 = path_builder.build();

        let mut path_builder = Path::builder();
        path_builder.begin(lyon_math::point(200.0, 200.0));
        path_builder.line_to(lyon_math::point(100.0, 200.0));
        path_builder
            .quadratic_bezier_to(lyon_math::point(10.0, 200.0), lyon_math::point(10.0, 100.0));
        path_builder.close();
        let path2 = path_builder.build();

        let mut path_builder = Path::builder();
        path_builder.begin(lyon_math::point(230.0, 20.0));
        path_builder.quadratic_bezier_to(
            lyon_math::point(230.0, 100.0),
            lyon_math::point(330.0, 200.0),
        );
        path_builder.end(false); // Don't close the path
        let path3 = path_builder.build();

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

        Some(vec![shape1, shape2, shape3])
    }
}

fn main() {
    println!("hello");
    lemna_baseview::Window::open_blocking::<App>(
        lemna_baseview::WindowOptions::new("Hello Shapes", (400, 300)).resizable(false),
    );
    println!("bye");
}
