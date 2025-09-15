use crate::font_cache::FontCache;
use crate::render::raster_cache::RasterCache;

pub(crate) mod glyph_brush_draw_cache;
mod wgpu;
pub(crate) use wgpu::*;

mod renderables;
pub use renderables::*;

/// The caches used by the Renderer. Passed to [`Component#render`][crate::Component#method.render] in a [`RenderContext`][crate::RenderContext].
#[derive(Default)]
pub struct Caches {
    /// Cache for shape renderable data
    pub(crate) shape_buffer: BufferCache<renderables::shape::Vertex, u16>,
    /// Cache for image renderable data
    pub(crate) image_buffer: BufferCache<renderables::raster::Vertex, u16>,
    /// Cache for raster data
    pub(crate) raster: RasterCache,
    /// Cache for text renderable data
    pub(crate) text_buffer: BufferCache<renderables::text::Vertex, u16>,
    /// Font cache
    pub font: FontCache,
}
