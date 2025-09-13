extern crate alloc;

use alloc::vec::Vec;
use core::fmt;

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

#[cfg(feature = "cpu_renderer")]
mod rgb_color {
    use embedded_graphics::pixelcolor;
    pub trait RgbColor: embedded_graphics::prelude::RgbColor {
        fn new(r: u8, g: u8, b: u8) -> Self;
    }

    impl RgbColor for pixelcolor::Rgb888 {
        fn new(r: u8, g: u8, b: u8) -> Self {
            pixelcolor::Rgb888::new(r, g, b)
        }
    }

    impl RgbColor for pixelcolor::Rgb565 {
        fn new(r: u8, g: u8, b: u8) -> Self {
            pixelcolor::Rgb565::new(r, g, b)
        }
    }

    impl RgbColor for pixelcolor::Rgb666 {
        fn new(r: u8, g: u8, b: u8) -> Self {
            pixelcolor::Rgb666::new(r, g, b)
        }
    }

    impl RgbColor for pixelcolor::Rgb555 {
        fn new(r: u8, g: u8, b: u8) -> Self {
            pixelcolor::Rgb555::new(r, g, b)
        }
    }

    impl RgbColor for pixelcolor::Bgr888 {
        fn new(r: u8, g: u8, b: u8) -> Self {
            pixelcolor::Bgr888::new(r, g, b)
        }
    }

    impl RgbColor for pixelcolor::Bgr565 {
        fn new(r: u8, g: u8, b: u8) -> Self {
            pixelcolor::Bgr565::new(r, g, b)
        }
    }

    impl RgbColor for pixelcolor::Bgr666 {
        fn new(r: u8, g: u8, b: u8) -> Self {
            pixelcolor::Bgr666::new(r, g, b)
        }
    }

    impl RgbColor for pixelcolor::Bgr555 {
        fn new(r: u8, g: u8, b: u8) -> Self {
            pixelcolor::Bgr555::new(r, g, b)
        }
    }
}

#[cfg(feature = "cpu_renderer")]
pub use rgb_color::*;

pub(crate) trait Renderer: core::fmt::Debug + core::marker::Sized + Send + Sync {
    fn new<W: Window>(window: &W) -> Self;
    #[cfg(feature = "cpu_renderer")]
    fn render<
        D: embedded_graphics::draw_target::DrawTarget<Color = C, Error = E>,
        C: RgbColor,
        E: core::fmt::Debug,
    >(
        &mut self,
        _draw_target: &mut D,
        _node: &Node,
        _caches: &mut Caches,
        _physical_size: PixelSize,
    ) {
    }
    #[cfg(not(feature = "cpu_renderer"))]
    fn render(&mut self, _node: &Node, _caches: &mut Caches, _physical_size: PixelSize) {}
}

#[cfg(feature = "wgpu_renderer")]
pub(crate) type ActiveRenderer = crate::render::wgpu::WGPURenderer;
#[cfg(feature = "cpu_renderer")]
pub(crate) type ActiveRenderer = crate::render::cpu_render::CPURenderer;

pub enum RasterData {
    Vec(Vec<u8>),
    Slice(&'static [u8]),
}

impl fmt::Debug for RasterData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (t, len) = match self {
            RasterData::Slice(d) => ("Slice", d.len()),
            RasterData::Vec(d) => ("Vec", d.len()),
        };
        write!(f, "RasterData::{}<len: {}>", t, len)?;
        Ok(())
    }
}

impl From<&'static [u8]> for RasterData {
    fn from(d: &'static [u8]) -> Self {
        RasterData::Slice(d)
    }
}

impl From<Vec<u8>> for RasterData {
    fn from(d: Vec<u8>) -> Self {
        RasterData::Vec(d)
    }
}

impl<'a> From<&'a RasterData> for &'a [u8] {
    fn from(d: &'a RasterData) -> &'a [u8] {
        match d {
            RasterData::Vec(v) => &v[..],
            RasterData::Slice(s) => s,
        }
    }
}
