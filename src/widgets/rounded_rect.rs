extern crate alloc;

use alloc::{vec, vec::Vec};
use core::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::renderable::Renderable;

#[derive(Debug)]
pub struct RoundedRect {
    pub background_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radii: BorderRadii,
}

impl Default for RoundedRect {
    fn default() -> Self {
        Self {
            background_color: Color::WHITE,
            border_color: Color::BLACK,
            border_width: 0.0,
            radii: BorderRadii::all(3.0),
        }
    }
}

impl RoundedRect {
    pub fn new<C: Into<Color>>(bg: C, radius: f32) -> Self {
        Self {
            background_color: bg.into(),
            border_color: Color::BLACK,
            border_width: 0.0,
            radii: BorderRadii::all(radius),
        }
    }

    pub fn radius(mut self, r: f32) -> Self {
        self.radii = BorderRadii::all(r);
        self
    }

    pub fn border_width(mut self, w: f32) -> Self {
        self.border_width = w;
        self
    }

    pub fn border_color(mut self, c: Color) -> Self {
        self.border_color = c;
        self
    }
}

impl Component for RoundedRect {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.background_color.hash(hasher);
        self.border_color.hash(hasher);
        ((self.border_width * 100000.0) as u32).hash(hasher);
        self.radii.hash(hasher);
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        use crate::renderable::{Path, Shape};

        let rect = Rect {
            pos: Pos::ORIGIN,
            bottom_right: Point::new(context.aabb.width(), context.aabb.height()),
        };
        match Path::rounded_rectangle(&rect, &self.radii) {
            Ok(path) => Some(vec![Renderable::Shape(Shape::new(
                path,
                self.background_color,
                self.border_color,
                self.border_width * context.scale_factor,
                0.0,
                context.caches,
                context
                    .prev_state
                    .as_ref()
                    .and_then(|r| r.first())
                    .and_then(|r| r.as_shape()),
            ))]),
            Err(e) => {
                log::error!("Failed to build path: {:?}", e);
                None
            }
        }
    }
}
