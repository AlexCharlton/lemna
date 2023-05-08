use std::fmt;
use std::sync::{Arc, RwLock};

use crate::base_types::*;
use crate::font_cache::FontCache;
use crate::node::Node;
use crate::window::Window;

pub mod glyph_brush_draw_cache;
pub mod renderables;
pub mod wgpu;

use crate::render::renderables::buffer_cache::BufferCache;
pub use renderables::Renderable;

#[derive(Clone)]
pub struct BufferCaches {
    pub shape_cache: Arc<RwLock<BufferCache<renderables::shape::Vertex, u16>>>,
    pub text_cache: Arc<RwLock<BufferCache<renderables::text::Vertex, u16>>>,
}

pub trait Renderer: fmt::Debug + std::marker::Sized + Send + Sync {
    fn new<W: Window>(window: &W) -> Self;
    fn render(&mut self, _node: &Node, _physical_size: PixelSize, _font_cache: &FontCache) {}
    fn buffer_caches(&self) -> BufferCaches {
        BufferCaches {
            shape_cache: Arc::new(RwLock::new(BufferCache::new())),
            text_cache: Arc::new(RwLock::new(BufferCache::new())),
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
