use lemna::render::wgpu::Shape;
use lemna::{self, *};
use lyon::path::Path;
use lyon::tessellation::math as lyon_math;

type Renderer = lemna::render::wgpu::WGPURenderer;
type Renderable = lemna::render::wgpu::WGPURenderable;

#[derive(Debug)]
pub struct HelloApp {}

impl lemna::Component<Renderer> for HelloApp {
    fn render<'a>(&mut self, context: RenderContext<'a, Renderer>) -> Option<Vec<Renderable>> {
        let mut path_builder = Path::builder();
        path_builder.move_to(lyon_math::point(10.0, 10.0));
        path_builder.line_to(lyon_math::point(100.0, 10.0));
        path_builder.quadratic_bezier_to(
            lyon_math::point(200.0, 10.0),
            lyon_math::point(200.0, 100.0),
        );
        path_builder.close();
        let path1 = path_builder.build();
        let (geom1, index_count1) = Shape::path_to_shape_geometry(path1, true, true);

        let mut path_builder = Path::builder();
        path_builder.move_to(lyon_math::point(200.0, 200.0));
        path_builder.line_to(lyon_math::point(100.0, 200.0));
        path_builder
            .quadratic_bezier_to(lyon_math::point(10.0, 200.0), lyon_math::point(10.0, 100.0));
        path_builder.close();
        let path2 = path_builder.build();
        let (geom2, index_count2) = Shape::path_to_shape_geometry(path2, true, false);

        let mut path_builder = Path::builder();
        path_builder.move_to(lyon_math::point(230.0, 20.0));
        path_builder.quadratic_bezier_to(
            lyon_math::point(230.0, 100.0),
            lyon_math::point(330.0, 200.0),
        );
        let path3 = path_builder.build();
        let (geom3, _) = Shape::path_to_shape_geometry(path3, false, true);

        Some(vec![
            Renderable::Shape(Shape::new(
                geom1,
                index_count1,
                [1.0, 0.0, 0.0].into(),
                [0.0, 0.0, 0.0].into(),
                4.0,
                0.0,
                &mut context.renderer.shape_pipeline,
                context.prev_state.as_ref().and_then(|v| match v.get(0) {
                    Some(Renderable::Shape(r)) => Some(r.buffer_id),
                    _ => None,
                }),
            )),
            Renderable::Shape(Shape::new(
                geom2,
                index_count2,
                [0.0, 1.0, 0.3].into(),
                [1.0, 1.0, 1.0].into(),
                0.0,
                0.0,
                &mut context.renderer.shape_pipeline,
                context.prev_state.as_ref().and_then(|v| match v.get(1) {
                    Some(Renderable::Shape(r)) => Some(r.buffer_id),
                    _ => None,
                }),
            )),
            Renderable::Shape(Shape::stroke(
                geom3,
                [0.0, 1.0, 0.0].into(),
                6.0,
                0.0,
                &mut context.renderer.shape_pipeline,
                context.prev_state.as_ref().and_then(|v| match v.get(2) {
                    Some(Renderable::Shape(r)) => Some(r.buffer_id),
                    _ => None,
                }),
            )),
        ])
    }
}

impl lemna::App<Renderer> for HelloApp {
    fn new() -> Self {
        Self {}
    }
}

fn main() {
    println!("hello");
    lemna_wx_rs::Window::<Renderer, HelloApp>::open_blocking("Hello shapes!", 400, 300, vec![]);
    println!("bye");
}
