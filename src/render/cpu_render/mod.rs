extern crate alloc;

use core::marker::PhantomData;

use alloc::vec;

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

#[derive(Default)]
pub struct Caches {
    pub(crate) raster: RasterCache,
    pub(crate) font: FontCache,
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
                Renderable::Rectangle(rect) => {
                    rect.render(aabb, current_mask.as_ref(), &mut self.pixmap);
                }
                Renderable::Shape(shape) => {
                    shape.render(aabb, current_mask.as_ref(), &mut self.pixmap);
                }
                Renderable::Text(text) => {
                    text.render(aabb, current_mask.as_ref(), &mut self.pixmap);
                }
                Renderable::Raster(raster) => {
                    raster.render(aabb, current_mask.as_ref(), &mut self.pixmap, caches);
                }
                #[cfg(test)]
                _ => panic!("Unsupported renderable: {:?}", renderable),
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
