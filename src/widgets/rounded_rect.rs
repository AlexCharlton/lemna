extern crate alloc;

use alloc::{vec, vec::Vec};
use core::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::render::Renderable;

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
            background_color: Color::WHITE,
            border_color: Color::BLACK,
            border_width: 0.0,
            radius: (3.0, 3.0, 3.0, 3.0),
        }
    }
}

impl RoundedRect {
    pub fn new<C: Into<Color>>(bg: C, radius: f32) -> Self {
        Self {
            background_color: bg.into(),
            border_color: Color::BLACK,
            border_width: 0.0,
            radius: (radius, radius, radius, radius),
        }
    }

    pub fn radius(mut self, r: f32) -> Self {
        self.radius = (r, r, r, r);
        self
    }
}

impl Component for RoundedRect {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.background_color.hash(hasher);
        self.border_color.hash(hasher);
        (self.border_width as u32).hash(hasher);
        (self.radius.0 as i32).hash(hasher);
        (self.radius.1 as i32).hash(hasher);
        (self.radius.2 as i32).hash(hasher);
        (self.radius.3 as i32).hash(hasher);
    }

    #[cfg(feature = "wgpu_renderer")]
    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        use crate::render::renderables::shape::Shape;
        use lyon::tessellation::math as lyon_math;
        use lyon_math::{Box2D, Point};

        let rect = Box2D {
            min: Point::new(0.0, 0.0),
            max: Point::new(context.aabb.width(), context.aabb.height()),
        };
        let radii = lyon::path::builder::BorderRadii {
            top_left: self.radius.0,
            top_right: self.radius.1,
            bottom_right: self.radius.2,
            bottom_left: self.radius.3,
        };
        let mut builder = lyon::path::Path::builder();
        builder.add_rounded_rectangle(&rect, &radii, lyon::path::Winding::Positive);
        let path = builder.build();

        let (geometry, fill_count) =
            Shape::path_to_shape_geometry(path, true, self.border_width > 0.0);

        Some(vec![Renderable::Shape(Shape::new(
            geometry,
            fill_count,
            self.background_color,
            self.border_color,
            self.border_width * 0.5,
            0.0,
            &mut context.caches.shape_buffer,
            context.prev_state.as_ref().and_then(|v| match v.first() {
                Some(Renderable::Shape(r)) => Some(r.buffer_id),
                _ => None,
            }),
        ))])
    }

    #[cfg(feature = "cpu_renderer")]
    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        todo!()
    }
}
