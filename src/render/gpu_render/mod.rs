use std::fmt;
use std::sync::{Arc, RwLock};

use crate::base_types::*;
use crate::font_cache::FontCache;
use crate::node::Node;
use crate::window::Window;

pub(crate) mod glyph_brush_draw_cache;
pub mod renderables;
pub(crate) mod wgpu;

use crate::render::renderables::BufferCache;
use crate::render::renderables::RasterCache;
pub use renderables::Renderable;

/// The caches used by the Renderer. Passed to [`Component#render`][crate::Component#method.render] in a [`RenderContext`][crate::RenderContext].
#[derive(Clone, Default)]
pub struct Caches {
    /// Cache for shape renderable data
    pub shape_buffer: Arc<RwLock<BufferCache<renderables::shape::Vertex, u16>>>,
    /// Cache for text renderable data
    pub text_buffer: Arc<RwLock<BufferCache<renderables::text::Vertex, u16>>>,
    /// Cache for image renderable data
    pub image_buffer: Arc<RwLock<BufferCache<renderables::raster::Vertex, u16>>>,
    /// Cache for raster data
    pub raster: Arc<RwLock<RasterCache>>,
    /// Font cache
    pub font: Arc<RwLock<FontCache>>,
}

pub(crate) trait Renderer: fmt::Debug + std::marker::Sized + Send + Sync {
    fn new<W: Window>(window: &W) -> Self;
    fn render(&mut self, _node: &Node, _physical_size: PixelSize) {}
    /// This default is provided for tests, it should be overridden
    fn caches(&self) -> Caches {
        Default::default()
        // Caches {
        //     shape_buffer: Arc::new(RwLock::new(BufferCache::new())),
        //     text_buffer: Arc::new(RwLock::new(BufferCache::new())),
        //     image_buffer: Arc::new(RwLock::new(BufferCache::new())),
        //     raster: Arc::new(RwLock::new(RasterCache::new())),
        //     font: Default
        // }
    }
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
