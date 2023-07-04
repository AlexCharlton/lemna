pub mod buffer_cache;
pub mod raster;
pub mod raster_cache;
pub mod rect;
pub mod shape;
pub mod text;

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
    Inc { repr: String, i: usize },
}
