//! The interface used by [`Component#render`][crate::Component#method.render].
//!
#![doc = include_str!("../../../../docs/renderables.md")]

mod buffer_cache;
pub(crate) mod raster;
pub(crate) mod rectangle;
pub(crate) mod shape;
pub(crate) mod text;

pub(crate) use buffer_cache::*;
pub use raster::Raster;
pub use rectangle::Rectangle;
pub use shape::Shape;
pub use text::Text;
