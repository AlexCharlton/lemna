use std::fmt;
use std::sync::{Arc, RwLock};

use crate::base_types::*;
use crate::font_cache::FontCache;
use crate::node::Node;
use crate::window::Window;

pub(crate) mod glyph_brush_draw_cache;
pub mod renderables;
pub(crate) mod wgpu;

use crate::render::renderables::buffer_cache::BufferCache;
use crate::render::renderables::raster_cache::RasterCache;
pub use crate::render::wgpu::WGPURenderer;
pub use renderables::Renderable;

#[derive(Clone)]
pub struct Caches {
    pub shape_buffer_cache: Arc<RwLock<BufferCache<renderables::shape::Vertex, u16>>>,
    pub text_buffer_cache: Arc<RwLock<BufferCache<renderables::text::Vertex, u16>>>,
    pub image_buffer_cache: Arc<RwLock<BufferCache<renderables::raster::Vertex, u16>>>,
    pub raster_cache: Arc<RwLock<RasterCache>>,
}

pub trait Renderer: fmt::Debug + std::marker::Sized + Send + Sync {
    fn new<W: Window>(window: &W) -> Self;
    fn render(&mut self, _node: &Node, _physical_size: PixelSize, _font_cache: &FontCache) {}
    fn caches(&self) -> Caches {
        Caches {
            shape_buffer_cache: Arc::new(RwLock::new(BufferCache::new())),
            text_buffer_cache: Arc::new(RwLock::new(BufferCache::new())),
            image_buffer_cache: Arc::new(RwLock::new(BufferCache::new())),
            raster_cache: Arc::new(RwLock::new(RasterCache::new())),
        }
    }
}

pub fn next_power_of_2(n: usize) -> usize {
    let mut n = n - 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n + 1
}
