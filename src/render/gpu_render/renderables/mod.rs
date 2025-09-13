//! The interface used by [`Component#render`][crate::Component#method.render].
//!
#![doc = include_str!("../../../../docs/renderables.md")]

mod buffer_cache;
pub mod raster;
mod raster_cache;
pub mod rect;
pub mod shape;
pub mod text;

pub use buffer_cache::*;
pub use raster::Raster;
pub use raster_cache::*;
pub use rect::Rect;
pub use shape::Shape;
pub use text::Text;
