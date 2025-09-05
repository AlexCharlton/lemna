#[cfg(feature = "std")]
mod gpu_render;
#[cfg(feature = "std")]
pub use gpu_render::*;

#[cfg(not(feature = "std"))]
mod cpu_render;
#[cfg(not(feature = "std"))]
pub use cpu_render::*;
