extern crate alloc;

use core::marker::PhantomData;

use alloc::{vec, vec::Vec};

use embedded_graphics::draw_target::DrawTarget;
use tiny_skia::{Color, Mask, Pixmap, Transform};

use super::{Renderer, RgbColor};
use crate::base_types::{PixelSize, Rect};
use crate::font_cache::FontCache;
use crate::node::Node;
use crate::render::raster_cache::RasterCache;
use crate::renderable::Renderable;
use crate::window::Window;

mod renderable;
pub use renderable::*;

mod glyph_cache;
use glyph_cache::GlyphCache;

#[derive(Default)]
pub struct Caches {
    pub(crate) raster: RasterCache,
    pub(crate) font: FontCache,
    pub(crate) glyph: GlyphCache,
}

#[derive(Debug)]
pub(crate) struct CPURenderer {
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
        caches: &mut Caches,
        size: PixelSize,
    ) {
        if size != self.size {
            self.size = size;
            self.pixmap = Pixmap::new(size.width, size.height).unwrap();
        }
        self.pixmap.fill(Color::WHITE);

        // Structure to hold a renderable with its frame index
        struct RenderableWithFrame<'a> {
            renderable: &'a Renderable,
            aabb: &'a Rect,
            frame_index: usize,
        }

        // First pass: collect all renderables and unique frames
        let mut renderables: Vec<RenderableWithFrame> = vec![];
        let mut current_frame = vec![];
        // The first frame is always empty
        let mut unique_frames: Vec<Vec<Rect>> = vec![vec![]];
        let mut current_frame_index = 0;

        for (renderable, aabb, frame) in node.iter_renderables() {
            if frame != current_frame {
                // Find or add the frame to unique_frames
                current_frame_index = unique_frames
                    .iter()
                    .position(|f| f == &frame)
                    .unwrap_or_else(|| {
                        unique_frames.push(frame.clone());
                        unique_frames.len() - 1
                    });
                current_frame = frame;
            }
            renderables.push(RenderableWithFrame {
                renderable,
                aabb,
                frame_index: current_frame_index,
            });
        }

        // Compute masks for each unique frame once
        let mut frame_masks: Vec<Option<Mask>> = vec![None; unique_frames.len()];
        for (i, frame) in unique_frames.iter().enumerate() {
            if frame.is_empty() {
                frame_masks[i] = None;
            } else {
                let mut mask = Some(Mask::new(size.width, size.height).unwrap());
                update_mask_from_frames(&size, frame, &mut mask);
                frame_masks[i] = mask;
            }
        }

        // Sort all renderables by z-index (lowest to highest)
        renderables.sort_by(|a, b| {
            let z_a = a.renderable.z() + a.aabb.pos.z;
            let z_b = b.renderable.z() + b.aabb.pos.z;
            z_a.partial_cmp(&z_b).unwrap()
        });

        // Render all renderables in sorted order
        for renderable_with_frame in renderables {
            let mask = frame_masks[renderable_with_frame.frame_index].as_ref();
            match renderable_with_frame.renderable {
                Renderable::Rectangle(rect) => {
                    rect.render(renderable_with_frame.aabb, mask, &mut self.pixmap);
                }
                Renderable::Shape(shape) => {
                    shape.render(renderable_with_frame.aabb, mask, &mut self.pixmap);
                }
                Renderable::Text(text) => {
                    text.render(renderable_with_frame.aabb, mask, &mut self.pixmap, caches);
                }
                Renderable::Raster(raster) => {
                    raster.render(renderable_with_frame.aabb, mask, &mut self.pixmap, caches);
                }
                #[cfg(test)]
                _ => panic!(
                    "Unsupported renderable: {:?}",
                    renderable_with_frame.renderable
                ),
            }
        }

        // Draw the pixmap to the draw target
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

fn update_mask_from_frames(size: &PixelSize, frames: &[Rect], mask: &mut Option<Mask>) {
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
        tiny_skia::FillRule::default(),
        false,
        Transform::identity(),
    );
}

//------------------------------------------
// MARK: PixMapIterator
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
