use crate::base_types::Point;

#[derive(Debug)]
pub enum PathBuilderError {
    InvalidPath,
}

#[cfg(feature = "cpu_renderer")]
mod cpu_path {
    use super::*;

    use tiny_skia::Path as SkiaPath;
    use tiny_skia::PathBuilder as SkiaPathBuilder;

    pub struct Path(SkiaPath);

    pub struct PathBuilder(SkiaPathBuilder);

    impl PathBuilder {
        pub fn new() -> Self {
            Self(SkiaPathBuilder::new())
        }

        pub fn build(self) -> Result<Path, PathBuilderError> {
            Ok(Path(self.0.finish().ok_or(PathBuilderError::InvalidPath)?))
        }

        pub fn begin(&mut self, p: Point) {
            self.0.move_to(p.x, p.y);
        }

        pub fn end(&mut self) {
            self.0.close();
        }

        pub fn line_to(&mut self, x: f32, y: f32) {
            self.0.line_to(x, y);
        }

        pub fn cubic_to(&mut self, cp1: Point, cp2: Point, dest: Point) {
            self.0.cubic_to(cp1.x, cp1.y, cp2.x, cp2.y, dest.x, dest.y);
        }

        pub fn quad_to(&mut self, cp: Point, dest: Point) {
            self.0.quad_to(cp.x, cp.y, dest.x, dest.y);
        }
    }
}

#[cfg(feature = "cpu_renderer")]
pub use cpu_path::*;

#[cfg(feature = "wgpu_renderer")]
mod gpu_path {
    use super::*;

    use lyon::path::Builder as LyonPathBuilder;
    use lyon::path::Path as LyonPath;
    use lyon::tessellation::math as lyon_math;

    pub struct Path(LyonPath);

    pub struct PathBuilder(LyonPathBuilder);

    impl PathBuilder {
        pub fn new() -> Self {
            Self(LyonPathBuilder::new())
        }

        pub fn build(self) -> Result<Path, PathBuilderError> {
            Ok(Path(self.0.build()))
        }

        pub fn begin(&mut self, p: Point) {
            self.0.begin(lyon_math::point(p.x, p.y));
        }

        pub fn end(&mut self) {
            self.0.close();
        }

        pub fn line_to(&mut self, x: f32, y: f32) {
            self.0.line_to(lyon_math::point(x, y));
        }
    }
}

#[cfg(feature = "wgpu_renderer")]
pub use gpu_path::*;

// Shared implementation
impl PathBuilder {
    pub fn arc_to(&mut self, dest: Point, radius: f32) {
        todo!()
    }
}
