extern crate alloc;

use core::marker::PhantomData;

use alloc::{string::String, vec};

use embedded_graphics::draw_target::DrawTarget;
use tiny_skia::{BlendMode, Color, Mask, Paint, Pixmap, Shader, Transform};

use super::{Renderer, RgbColor};
use crate::base_types::{AABB, PixelSize, Pos, Scale};
use crate::font_cache::FontCache;
use crate::node::Node;
use crate::window::Window;

mod rect;

pub mod renderables {
    pub use super::rect::Rect;
}

#[derive(Debug, PartialEq)]
pub enum Renderable {
    Rect(rect::Rect),
    Shape,  //(Shape),
    Text,   //(Text),
    Raster, //(Raster),
    // Renderable that just holds a counter, used for tests
    Inc { repr: String, i: usize },
}

#[derive(Default)]
pub struct Caches {
    pub font: FontCache,
}

#[derive(Debug)]
pub struct CPURenderer {
    size: PixelSize,
    pixmap: Pixmap,
}

impl Renderer for CPURenderer {
    fn new<W: Window>(window: &W) -> Self {
        let size = window.physical_size();
        let pixmap = Pixmap::new(size.width, size.height).unwrap();
        Self { size, pixmap }
    }

    fn render<D: DrawTarget<Color = C, Error = E>, C: RgbColor, E: core::fmt::Debug>(
        &mut self,
        draw_target: &mut D,
        node: &Node,
        _caches: &mut Caches,
        size: PixelSize,
    ) {
        if size != self.size {
            self.size = size;
            self.pixmap = Pixmap::new(size.width, size.height).unwrap();
        }
        self.pixmap.fill(Color::WHITE);

        let mut current_frame = vec![];
        let mut current_mask = None;
        for (renderable, aabb, frame) in node.iter_renderables() {
            if frame != current_frame {
                if frame.is_empty() {
                    current_mask = None;
                } else {
                    update_mask_from_frames(&size, &frame, &mut current_mask);
                }
                current_frame = frame;
            }
            match renderable {
                Renderable::Rect(rect::Rect { pos, scale, color }) => {
                    let paint = Paint {
                        shader: Shader::SolidColor(color.into()),
                        anti_alias: true,
                        blend_mode: BlendMode::SourceOver,
                        force_hq_pipeline: false,
                    };

                    self.pixmap.fill_rect(
                        rect_from_pos_scale(&(aabb.pos + *pos), scale),
                        &paint,
                        Transform::identity(),
                        current_mask.as_ref(),
                    );
                }
                // TODO
                _ => (),
            }
        }

        if let Err(e) = draw_target.fill_contiguous(
            &embedded_graphics::primitives::Rectangle::new(
                embedded_graphics::geometry::Point::new(0, 0),
                embedded_graphics::geometry::Size::new(size.width, size.height),
            ),
            PixMapIterator::new(&self.pixmap),
        ) {
            log::error!("Failed to fill draw target: {:?}", e);
        }
    }
}

fn rect_from_pos_scale(pos: &Pos, scale: &Scale) -> tiny_skia::Rect {
    tiny_skia::Rect::from_xywh(pos.x, pos.y, scale.width, scale.height).unwrap()
}

fn update_mask_from_frames(size: &PixelSize, frames: &[AABB], mask: &mut Option<Mask>) {
    if mask.is_none() {
        *mask = Some(Mask::new(size.width, size.height).unwrap());
    } else {
        mask.as_mut().unwrap().clear();
    }

    let mask = mask.as_mut().unwrap();
    let mut rect: tiny_skia::Rect = frames.first().expect("At least one frame").into();

    for frame in frames {
        if let Some(new_rect) = rect.intersect(&frame.into()) {
            rect = new_rect;
        } else {
            // No intersection, so nothing is visible
            mask.clear();
            return;
        }
    }

    let path = tiny_skia::PathBuilder::from_rect(rect);

    mask.fill_path(
        &path,
        tiny_skia::FillRule::EvenOdd,
        false,
        Transform::identity(),
    );
}

struct PixMapIterator<'a, C: RgbColor> {
    pixmap_data: &'a [u8], // RGBA
    index: usize,
    color: PhantomData<C>,
}

impl<'a, C: RgbColor> PixMapIterator<'a, C> {
    fn new(pixmap: &'a Pixmap) -> Self {
        Self {
            pixmap_data: pixmap.data(),
            index: 0,
            color: PhantomData,
        }
    }
}

impl<'a, C: RgbColor> Iterator for PixMapIterator<'a, C> {
    type Item = C;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.pixmap_data.len() {
            None
        } else {
            let color = C::new(
                self.pixmap_data[self.index],
                self.pixmap_data[self.index + 1],
                self.pixmap_data[self.index + 2],
            );
            self.index += 4; // RGBA
            Some(color)
        }
    }
}
