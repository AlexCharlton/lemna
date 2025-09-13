extern crate alloc;

use tiny_skia::{BlendMode, Mask, Paint, Pixmap, Shader, Transform};

use crate::base_types::{AABB, Color, Pos, Scale};

pub use raster::Raster;
pub use rect::Rect;
pub use shape::Shape;
pub use text::Text;

#[derive(Debug, PartialEq)]
pub enum Renderable {
    Rect(Rect),
    Shape(Shape),
    Text(Text),
    Raster(Raster),
    // Renderable that just holds a counter, used for tests
    #[cfg(test)]
    Inc {
        repr: alloc::string::String,
        i: usize,
    },
}

mod rect {
    use super::*;

    #[derive(Debug, PartialEq)]
    pub struct Rect {
        pub pos: Pos,
        pub scale: Scale,
        pub color: Color,
    }

    impl Rect {
        pub fn new(pos: Pos, scale: Scale, color: Color) -> Self {
            Self { pos, scale, color }
        }

        pub(crate) fn render(&self, aabb: &AABB, mask: Option<&Mask>, pixmap: &mut Pixmap) {
            let paint = Paint {
                shader: Shader::SolidColor(self.color.into()),
                anti_alias: true,
                blend_mode: BlendMode::SourceOver,
                force_hq_pipeline: false,
            };

            pixmap.fill_rect(
                rect_from_pos_scale(&(aabb.pos + self.pos), &self.scale),
                &paint,
                Transform::identity(),
                mask,
            )
        }
    }

    fn rect_from_pos_scale(pos: &Pos, scale: &Scale) -> tiny_skia::Rect {
        tiny_skia::Rect::from_xywh(pos.x, pos.y, scale.width, scale.height).unwrap()
    }
}

mod shape {
    use super::*;

    use tiny_skia::{Path, Stroke};

    #[derive(Debug, PartialEq)]
    pub struct Shape {
        path: Path,
        fill_color: Color,
        stroke_color: Color,
        stroke_width: f32,
        z: f32,
    }

    impl Shape {
        pub fn new(
            path: Path,
            fill_color: Color,
            stroke_color: Color,
            stroke_width: f32,
            z: f32,
        ) -> Self {
            Self {
                path,
                fill_color,
                stroke_color,
                stroke_width,
                z,
            }
        }

        pub(crate) fn render(&self, aabb: &AABB, mask: Option<&Mask>, pixmap: &mut Pixmap) {
            let transform = Transform::from_translate(aabb.pos.x, aabb.pos.y);
            if self.fill_color.is_visible() {
                let paint = Paint {
                    shader: Shader::SolidColor(self.fill_color.into()),
                    anti_alias: true,
                    blend_mode: BlendMode::SourceOver,
                    force_hq_pipeline: false,
                };

                pixmap.fill_path(
                    &self.path,
                    &paint,
                    tiny_skia::FillRule::default(),
                    transform,
                    mask,
                );
            }
            if self.stroke_color.is_visible() {
                let paint = Paint {
                    shader: Shader::SolidColor(self.stroke_color.into()),
                    anti_alias: true,
                    blend_mode: BlendMode::SourceOver,
                    force_hq_pipeline: false,
                };
                let stroke = Stroke {
                    width: self.stroke_width,
                    ..Default::default()
                };
                pixmap.stroke_path(&self.path, &paint, &stroke, transform, mask);
            }
        }
    }
}

mod text {
    use super::*;

    #[derive(Debug, PartialEq)]
    pub struct Text {}

    impl Text {
        pub fn new() -> Self {
            Self {}
        }

        pub(crate) fn render(&self, aabb: &AABB, mask: Option<&Mask>, pixmap: &mut Pixmap) {
            // TODO
        }
    }
}

mod raster {
    use super::*;

    #[derive(Debug, PartialEq)]
    pub struct Raster {}

    impl Raster {
        pub fn new() -> Self {
            Self {}
        }

        pub(crate) fn render(&self, aabb: &AABB, mask: Option<&Mask>, pixmap: &mut Pixmap) {
            // TODO
        }
    }
}
