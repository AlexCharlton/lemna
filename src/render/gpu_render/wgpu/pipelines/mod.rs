/// Load a SPIR-V shader compiled by `build.rs` into `$OUT_DIR/shaders/`.
macro_rules! include_shader {
    ($file:literal) => {
        ::wgpu::include_spirv!(concat!(env!("OUT_DIR"), "/shaders/", $file))
    };
}
pub(crate) use include_shader;

mod buffer_cache;
pub(crate) mod shared;
mod texture_cache;

pub mod raster;
pub use raster::RasterPipeline;
pub mod rect;
pub use rect::RectPipeline;
pub mod shape;
pub use shape::ShapePipeline;
pub mod text;
pub use text::TextPipeline;

pub(crate) mod msaa;
pub(crate) mod stencil;
