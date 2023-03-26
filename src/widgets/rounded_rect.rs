use std::hash::Hash;

use lyon::tessellation;
use lyon::tessellation::basic_shapes;
use lyon::tessellation::math as lyon_math;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::render::wgpu::{self, pipelines::shape, WGPURenderable, WGPURenderer};

#[derive(Debug)]
pub struct RoundedRect {
    pub background_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radius: (f32, f32, f32, f32),
}

impl Default for RoundedRect {
    fn default() -> Self {
        Self {
            background_color: [1.0, 1.0, 1.0].into(),
            border_color: [0.0, 0.0, 0.0].into(),
            border_width: 0.0,
            radius: (3.0, 3.0, 3.0, 3.0),
        }
    }
}

impl RoundedRect {
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = (r, r, r, r);
        self
    }
}

impl Component<WGPURenderer> for RoundedRect {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.background_color.hash(hasher);
        self.border_color.hash(hasher);
        (self.border_width as u32).hash(hasher);
        (self.radius.0 as i32).hash(hasher);
        (self.radius.1 as i32).hash(hasher);
        (self.radius.2 as i32).hash(hasher);
        (self.radius.3 as i32).hash(hasher);
    }

    fn render<'a>(
        &mut self,
        context: RenderContext<'a, WGPURenderer>,
    ) -> Option<Vec<WGPURenderable>> {
        let mut geometry = shape::ShapeGeometry::new();
        let rect = lyon_math::rect(0.0, 0.0, context.aabb.width(), context.aabb.height());
        let radii = basic_shapes::BorderRadii {
            top_left: self.radius.0,
            top_right: self.radius.1,
            bottom_right: self.radius.2,
            bottom_left: self.radius.3,
        };

        let fill_count = basic_shapes::fill_rounded_rectangle(
            &rect,
            &radii,
            &tessellation::FillOptions::tolerance(shape::TOLERANCE),
            &mut tessellation::BuffersBuilder::new(
                &mut geometry,
                shape::Vertex::basic_vertex_constructor,
            ),
        )
        .unwrap();

        if self.border_width > 0.0 {
            basic_shapes::stroke_rounded_rectangle(
                &rect,
                &radii,
                &tessellation::StrokeOptions::tolerance(shape::TOLERANCE).dont_apply_line_width(),
                &mut tessellation::BuffersBuilder::new(
                    &mut geometry,
                    shape::Vertex::stroke_vertex_constructor,
                ),
            )
            .unwrap();
        }

        Some(vec![WGPURenderable::Shape(wgpu::Shape::new(
            geometry,
            fill_count.indices,
            self.background_color,
            self.border_color,
            self.border_width * 0.5,
            0.0,
            &mut context.renderer.shape_pipeline,
            context.prev_state.as_ref().and_then(|v| match v.get(0) {
                Some(WGPURenderable::Shape(r)) => Some(r.buffer_id),
                _ => None,
            }),
        ))])
    }
}
