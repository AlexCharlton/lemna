use crate::base_types::{BorderRadii, Point, Rect};

#[derive(Debug)]
pub enum PathBuilderError {
    InvalidPath,
}

#[cfg(feature = "cpu_renderer")]
mod cpu_path {
    use super::*;

    use tiny_skia::Path as SkiaPath;
    use tiny_skia::PathBuilder as SkiaPathBuilder;

    #[derive(Debug, PartialEq)]
    pub struct Path(SkiaPath);

    impl Path {
        pub(crate) fn native_path(&self) -> &SkiaPath {
            &self.0
        }
    }

    pub struct PathBuilder {
        builder: SkiaPathBuilder,
        current_point: Point,
    }

    impl Default for PathBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl PathBuilder {
        pub fn new() -> Self {
            Self {
                builder: SkiaPathBuilder::new(),
                current_point: Point::new(0.0, 0.0),
            }
        }

        #[allow(unused_mut)]
        pub fn build(mut self) -> Result<Path, PathBuilderError> {
            Ok(Path(
                self.builder.finish().ok_or(PathBuilderError::InvalidPath)?,
            ))
        }

        pub fn begin(&mut self, p: Point) {
            self.builder.move_to(p.x, p.y);
            self.current_point = p;
        }

        pub fn close(&mut self) {
            self.builder.close();
        }

        pub fn end(&mut self) {
            // No-op
        }

        pub fn line_to(&mut self, p: Point) {
            self.builder.line_to(p.x, p.y);
            self.current_point = p;
        }

        pub fn cubic_to(&mut self, cp1: Point, cp2: Point, dest: Point) {
            self.builder
                .cubic_to(cp1.x, cp1.y, cp2.x, cp2.y, dest.x, dest.y);
            self.current_point = dest;
        }

        pub fn quad_to(&mut self, cp: Point, dest: Point) {
            self.builder.quad_to(cp.x, cp.y, dest.x, dest.y);
            self.current_point = dest;
        }

        /// Get the current point of the path.
        /// Only valid after path has begun and before it has been closed.
        pub fn current_point(&self) -> Point {
            self.current_point
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

    #[derive(Debug)]
    pub struct Path(LyonPath);

    impl Path {
        pub(crate) fn native_path(&self) -> &LyonPath {
            &self.0
        }
    }

    impl PartialEq for Path {
        fn eq(&self, other: &Self) -> bool {
            self.0.iter().zip(other.0.iter()).all(|(a, b)| a == b)
        }
    }

    pub struct PathBuilder {
        builder: LyonPathBuilder,
        ended: bool,
        current_point: Point,
    }

    impl Default for PathBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl PathBuilder {
        pub fn new() -> Self {
            Self {
                builder: LyonPathBuilder::new(),
                ended: true,
                current_point: Point::new(0.0, 0.0),
            }
        }

        pub fn build(mut self) -> Result<Path, PathBuilderError> {
            if !self.ended {
                self.builder.end(false);
            }
            Ok(Path(self.builder.build()))
        }

        pub fn begin(&mut self, p: Point) {
            self.builder.begin(lyon_math::point(p.x, p.y));
            self.ended = false;
            self.current_point = p;
        }

        pub fn close(&mut self) {
            self.builder.close();
            self.ended = true;
        }

        pub fn end(&mut self) {
            // End without closing the path
            self.builder.end(false);
            self.ended = true;
        }

        pub fn line_to(&mut self, p: Point) {
            self.builder.line_to(lyon_math::point(p.x, p.y));
            self.current_point = p;
        }

        pub fn cubic_to(&mut self, cp1: Point, cp2: Point, dest: Point) {
            self.builder.cubic_bezier_to(
                lyon_math::point(cp1.x, cp1.y),
                lyon_math::point(cp2.x, cp2.y),
                lyon_math::point(dest.x, dest.y),
            );
            self.current_point = dest;
        }

        pub fn quad_to(&mut self, cp: Point, dest: Point) {
            self.builder.quadratic_bezier_to(
                lyon_math::point(cp.x, cp.y),
                lyon_math::point(dest.x, dest.y),
            );
            self.current_point = dest;
        }

        /// Get the current point of the path.
        /// Only valid after path has begun and before it has been closed.
        pub fn current_point(&self) -> Point {
            self.current_point
        }
    }
}

#[cfg(feature = "wgpu_renderer")]
pub use gpu_path::*;

// Shared implementation
// Adapted from https://github.com/nical/lyon/blob/main/crates/path/src/builder.rs
impl Path {
    // https://spencermortensen.com/articles/bezier-circle/
    const CONSTANT_FACTOR: f32 = 0.55191505;

    pub fn builder() -> PathBuilder {
        PathBuilder::new()
    }

    pub fn rounded_rectangle(rect: &Rect, radii: &BorderRadii) -> Result<Path, PathBuilderError> {
        let mut builder = PathBuilder::new();

        let w = rect.width();
        let h = rect.height();
        let x_min = rect.pos.x;
        let y_min = rect.pos.y;
        let x_max = rect.bottom_right.x;
        let y_max = rect.bottom_right.y;
        let min_wh = w.min(h);
        let mut tl = radii.top_left.abs().min(min_wh);
        let mut tr = radii.top_right.abs().min(min_wh);
        let mut bl = radii.bottom_left.abs().min(min_wh);
        let mut br = radii.bottom_right.abs().min(min_wh);

        // clamp border radii if they don't fit in the rectangle.
        if tl + tr > w {
            let x = (tl + tr - w) * 0.5;
            tl -= x;
            tr -= x;
        }
        if bl + br > w {
            let x = (bl + br - w) * 0.5;
            bl -= x;
            br -= x;
        }
        if tr + br > h {
            let x = (tr + br - h) * 0.5;
            tr -= x;
            br -= x;
        }
        if tl + bl > h {
            let x = (tl + bl - h) * 0.5;
            tl -= x;
            bl -= x;
        }

        let tl_d = tl * Self::CONSTANT_FACTOR;
        let tl_corner = Point::new(x_min, y_min);

        let tr_d = tr * Self::CONSTANT_FACTOR;
        let tr_corner = Point::new(x_max, y_min);

        let br_d = br * Self::CONSTANT_FACTOR;
        let br_corner = Point::new(x_max, y_max);

        let bl_d = bl * Self::CONSTANT_FACTOR;
        let bl_corner = Point::new(x_min, y_max);

        let points = [
            Point::new(x_min, y_min + tl),          // begin
            tl_corner + Point::new(0.0, tl - tl_d), // control
            tl_corner + Point::new(tl - tl_d, 0.0), // control
            tl_corner + Point::new(tl, 0.0),        // end
            Point::new(x_max - tr, y_min),
            tr_corner + Point::new(-tr + tr_d, 0.0),
            tr_corner + Point::new(0.0, tr - tr_d),
            tr_corner + Point::new(0.0, tr),
            Point::new(x_max, y_max - br),
            br_corner + Point::new(0.0, -br + br_d),
            br_corner + Point::new(-br + br_d, 0.0),
            br_corner + Point::new(-br, 0.0),
            Point::new(x_min + bl, y_max),
            bl_corner + Point::new(bl - bl_d, 0.0),
            bl_corner + Point::new(0.0, -bl + bl_d),
            bl_corner + Point::new(0.0, -bl),
        ];

        builder.begin(points[0]);
        if tl > 0.0 {
            builder.cubic_to(points[1], points[2], points[3]);
        }
        builder.line_to(points[4]);
        if tr > 0.0 {
            builder.cubic_to(points[5], points[6], points[7]);
        }
        builder.line_to(points[8]);
        if br > 0.0 {
            builder.cubic_to(points[9], points[10], points[11]);
        }
        builder.line_to(points[12]);
        if bl > 0.0 {
            builder.cubic_to(points[13], points[14], points[15]);
        }

        builder.close();
        builder.build()
    }

    pub fn circle(center: Point, radius: f32) -> Result<Path, PathBuilderError> {
        let mut builder = PathBuilder::new();

        let radius = radius.abs();
        let d = radius * Self::CONSTANT_FACTOR;

        builder.begin(center + Point::new(-radius, 0.0));

        let ctrl_0 = center + Point::new(-radius, -d);
        let ctrl_1 = center + Point::new(-d, -radius);
        let mid = center + Point::new(0.0, -radius);
        builder.cubic_to(ctrl_0, ctrl_1, mid);

        let ctrl_0 = center + Point::new(d, -radius);
        let ctrl_1 = center + Point::new(radius, -d);
        let mid = center + Point::new(radius, 0.0);
        builder.cubic_to(ctrl_0, ctrl_1, mid);

        let ctrl_0 = center + Point::new(radius, d);
        let ctrl_1 = center + Point::new(d, radius);
        let mid = center + Point::new(0.0, radius);
        builder.cubic_to(ctrl_0, ctrl_1, mid);

        let ctrl_0 = center + Point::new(-d, radius);
        let ctrl_1 = center + Point::new(-radius, d);
        let mid = center + Point::new(-radius, 0.0);
        builder.cubic_to(ctrl_0, ctrl_1, mid);

        builder.close();
        builder.build()
    }

    pub fn from_kurbo(path: &kurbo::BezPath) -> Result<Path, PathBuilderError> {
        use kurbo::PathEl;

        let mut builder = PathBuilder::new();
        for el in path.iter() {
            match el {
                PathEl::MoveTo(p) => {
                    builder.begin(p.into());
                }
                PathEl::LineTo(p) => {
                    builder.line_to(p.into());
                }
                PathEl::QuadTo(p, c) => {
                    builder.quad_to(p.into(), c.into());
                }
                PathEl::CurveTo(p, c1, c2) => {
                    builder.cubic_to(p.into(), c1.into(), c2.into());
                }
                PathEl::ClosePath => {
                    builder.close();
                }
            }
        }

        builder.build()
    }

    pub fn ellipse(rect: &Rect) -> Result<Path, PathBuilderError> {
        use kurbo::Shape;

        Self::from_kurbo(&kurbo::Ellipse::from_rect(rect.into()).into_path(0.1))
    }
}

// Shared implementation
impl PathBuilder {
    // Adapted from https://github.com/iced-rs/iced/blob/master/tiny_skia/src/engine.rs
    pub fn arc_to(&mut self, dest: Point, radius: f32) {
        let current_point = self.current_point();
        let svg_arc = kurbo::SvgArc {
            from: kurbo::Point::new(f64::from(current_point.x), f64::from(current_point.y)),
            to: kurbo::Point::new(f64::from(dest.x), f64::from(dest.y)),
            radii: kurbo::Vec2::new(f64::from(radius), f64::from(radius)),
            x_rotation: 0.0,
            large_arc: false,
            sweep: true,
        };
        match kurbo::Arc::from_svg_arc(&svg_arc) {
            Some(arc) => {
                arc.to_cubic_beziers(0.1, |p1, p2, p| {
                    self.cubic_to(
                        Point::new(p1.x as f32, p1.y as f32),
                        Point::new(p2.x as f32, p2.y as f32),
                        Point::new(p.x as f32, p.y as f32),
                    );
                });
            }
            None => {
                self.line_to(dest);
            }
        }
    }
}
