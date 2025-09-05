extern crate alloc;

use alloc::string::String;

use crate::base_types::PixelSize;
use crate::font_cache::FontCache;
use crate::node::Node;

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

pub struct Renderer {}

impl Renderer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self, node: &Node, caches: &Caches, size: PixelSize) {
        // TODO
    }
}
