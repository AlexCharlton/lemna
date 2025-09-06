extern crate alloc;

use alloc::string::String;

use super::Renderer;
use crate::base_types::PixelSize;
use crate::font_cache::FontCache;
use crate::node::Node;
use crate::window::Window;

#[derive(Debug, PartialEq)]
pub enum Renderable {
    Rect,   // (Rect),
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
    fn new<W: Window>(window: &W) -> Self {
        Self {}
    }

    fn render(&mut self, _node: &Node, _caches: &mut Caches, _physical_size: PixelSize) {
        // TODO
    }
}
