use std::fmt;

use crate::base_types::*;
use crate::font_cache::FontCache;
use crate::node::Node;
use crate::window::Window;

pub mod wgpu;

pub trait Renderer: fmt::Debug + std::marker::Sized + Send + Sync {
    type Renderable: Send + Sync + fmt::Debug;

    fn new<W: Window>(window: &W) -> Self;
    fn render(&mut self, _node: &Node<Self>, _physical_size: PixelSize, _font_cache: &FontCache) {}
}
