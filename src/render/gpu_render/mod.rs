use std::fmt;

use crate::base_types::*;
use crate::font_cache::FontCache;
use crate::node::Node;
use crate::window::Window;

pub(crate) mod glyph_brush_draw_cache;
pub mod renderables;
pub(crate) mod wgpu;

use crate::render::renderables::BufferCache;
pub use renderables::{RasterCache, Renderable};

/// The caches used by the Renderer. Passed to [`Component#render`][crate::Component#method.render] in a [`RenderContext`][crate::RenderContext].
#[derive(Default)]
pub struct Caches {
    /// Cache for shape renderable data
    pub shape_buffer: BufferCache<renderables::shape::Vertex, u16>,
    /// Cache for image renderable data
    pub image_buffer: BufferCache<renderables::raster::Vertex, u16>,
    /// Cache for raster data
    pub raster: RasterCache,
    /// Cache for text renderable data
    pub text_buffer: BufferCache<renderables::text::Vertex, u16>,
    /// Font cache
    pub font: FontCache,
}

pub(crate) trait Renderer: fmt::Debug + std::marker::Sized + Send + Sync {
    fn new<W: Window>(window: &W) -> Self;
    fn render(&mut self, _node: &Node, _caches: &mut Caches, _physical_size: PixelSize) {}
}

/// Given an integer, return the next power of 2.
pub(crate) fn next_power_of_2(n: usize) -> usize {
    let mut n = n - 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n + 1
}
