mod buffer_cache;
pub(crate) mod shared;

pub mod rect;
pub use rect::RectPipeline;
pub mod shape;
pub use shape::ShapePipeline;
pub mod text;
pub use text::TextPipeline;

pub(crate) mod msaa;
pub(crate) mod stencil;
