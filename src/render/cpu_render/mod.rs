extern crate alloc;

use alloc::string::String;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::prelude::RgbColor;

use super::Renderer;
use crate::base_types::PixelSize;
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
pub struct CPURenderer {}

impl Renderer for CPURenderer {
    fn new<W: Window>(_window: &W) -> Self {
        Self {}
    }

    fn render<D: DrawTarget<Color = C, Error = E>, C: RgbColor, E: core::fmt::Debug>(
        &mut self,
        draw_target: &mut D,
        _node: &Node,
        _caches: &mut Caches,
        size: PixelSize,
    ) {
        let colors = vec![C::BLUE; (size.width * size.height) as usize];
        // TODO
        if let Err(e) = draw_target.fill_contiguous(
            &embedded_graphics::primitives::Rectangle::new(
                embedded_graphics::geometry::Point::new(0, 0),
                embedded_graphics::geometry::Size::new(size.width, size.height),
            ),
            colors,
        ) {
            log::error!("Failed to fill draw target: {:?}", e);
        }
    }
}
