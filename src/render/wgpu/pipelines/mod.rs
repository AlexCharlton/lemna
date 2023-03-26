mod buffer_cache;
mod glyph_brush_draw_cache;
mod shared;

pub mod rect;
pub use rect::{Rect, RectPipeline};
pub mod shape;
pub use shape::{Shape, ShapePipeline};
pub mod text;
pub use text::{Text, TextPipeline};

pub(crate) mod msaa;
pub(crate) mod stencil;
