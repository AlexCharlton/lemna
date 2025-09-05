extern crate alloc;

use alloc::string::String;

#[derive(Debug, PartialEq)]
pub enum Renderable {
    Rect,   // (Rect),
    Shape,  //(Shape),
    Text,   //(Text),
    Raster, //(Raster),
    // Renderable that just holds a counter, used for tests
    Inc { repr: String, i: usize },
}

pub struct Caches {}
