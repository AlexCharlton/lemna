use crate::base_types::*;
use crate::node::Node;
use crate::window::Window;

// Compile-time check to ensure exactly one renderer feature is enabled
#[cfg(all(feature = "wgpu_renderer", feature = "cpu_renderer"))]
compile_error!(
    "Cannot enable both 'wgpu_renderer' and 'cpu_renderer' features simultaneously. Please choose only one renderer."
);

#[cfg(not(any(feature = "wgpu_renderer", feature = "cpu_renderer")))]
compile_error!(
    "Must enable exactly one renderer feature: either 'wgpu_renderer' or 'cpu_renderer'."
);

#[cfg(feature = "wgpu_renderer")]
mod gpu_render;
#[cfg(feature = "wgpu_renderer")]
pub use gpu_render::*;

#[cfg(feature = "cpu_renderer")]
mod cpu_render;
#[cfg(feature = "cpu_renderer")]
pub use cpu_render::*;

pub(crate) trait Renderer: core::fmt::Debug + core::marker::Sized + Send + Sync {
    fn new<W: Window>(window: &W) -> Self;
    fn render(&mut self, _node: &Node, _caches: &mut Caches, _physical_size: PixelSize) {}
}

#[cfg(feature = "wgpu_renderer")]
pub type ActiveRenderer = crate::render::wgpu::WGPURenderer;
#[cfg(feature = "cpu_renderer")]
pub type ActiveRenderer = crate::render::cpu_render::CPURenderer;
