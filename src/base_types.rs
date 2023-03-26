use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::mem;
use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Sub, SubAssign};

pub trait Scalable {
    fn scale(self, _scale_factor: f32) -> Self;

    fn unscale(self, scale_factor: f32) -> Self
    where
        Self: Sized,
    {
        self.scale(1.0 / scale_factor)
    }
}

fn clamp(x: f32, min: f32, max: f32) -> f32 {
    if min > max {
        panic!("`min` should not be greater than `max`");
    } else {
        if x < min {
            min
        } else if x > max {
            max
        } else {
            x
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Scale {
    pub width: f32,
    pub height: f32,
}

impl Hash for Scale {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.width as u64).hash(state);
        (self.height as u64).hash(state);
    }
}

impl Default for Scale {
    fn default() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

impl Scalable for Scale {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            width: self.width * scale_factor,
            height: self.height * scale_factor,
        }
    }
}

impl Scale {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl Sub for Scale {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Scale {
            width: self.width - other.width,
            height: self.height - other.height,
        }
    }
}

impl Mul<f32> for Scale {
    type Output = Self;
    fn mul(self, factor: f32) -> Scale {
        Scale {
            width: self.width * factor,
            height: self.height * factor,
        }
    }
}

impl From<[f32; 2]> for Scale {
    fn from(p: [f32; 2]) -> Self {
        unsafe { mem::transmute(p) }
    }
}

impl From<PixelSize> for Scale {
    fn from(s: PixelSize) -> Self {
        Self {
            width: s.width as f32,
            height: s.height as f32,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn clamp(self, aabb: AABB) -> Self {
        Self {
            x: clamp(self.x, aabb.pos.x, aabb.bottom_right.x),
            y: clamp(self.y, aabb.pos.y, aabb.bottom_right.y),
        }
    }
}

impl Scalable for Point {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            x: self.x * scale_factor,
            y: self.y * scale_factor,
        }
    }
}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.x as i32).hash(state);
        (self.y as i32).hash(state);
    }
}

impl Default for Point {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<[f32; 2]> for Point {
    fn from(p: [f32; 2]) -> Self {
        unsafe { mem::transmute(p) }
    }
}

impl From<Pos> for Point {
    fn from(p: Pos) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(self, other: Point) -> Point {
        Point {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Div<f32> for Point {
    type Output = Self;
    fn div(self, f: f32) -> Self {
        Self {
            x: self.x / f,
            y: self.y / f,
        }
    }
}

impl Mul<f32> for Point {
    type Output = Self;
    fn mul(self, f: f32) -> Self {
        Self {
            x: self.x * f,
            y: self.y * f,
        }
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
        };
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x - other.x,
            y: self.y - other.y,
        };
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Hash for Pos {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.x as i32).hash(state);
        (self.y as i32).hash(state);
        (self.z as i32).hash(state);
    }
}

impl From<[f32; 3]> for Pos {
    fn from(p: [f32; 3]) -> Self {
        unsafe { mem::transmute(p) }
    }
}

impl From<[f32; 2]> for Pos {
    fn from(p: [f32; 2]) -> Self {
        Self {
            x: p[0],
            y: p[1],
            z: 0.0,
        }
    }
}

impl From<Point> for Pos {
    fn from(p: Point) -> Self {
        Self {
            x: p.x,
            y: p.y,
            z: 0.0,
        }
    }
}

impl Default for Pos {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl Scalable for Pos {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            x: self.x * scale_factor,
            y: self.y * scale_factor,
            z: self.z,
        }
    }
}

impl Pos {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn round(&self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
            z: self.z.round(),
        }
    }
}

impl Add for Pos {
    type Output = Pos;

    fn add(self, other: Pos) -> Pos {
        Pos {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Pos {
    type Output = Pos;

    fn sub(self, other: Pos) -> Pos {
        Pos {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl AddAssign for Pos {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        };
    }
}

impl SubAssign for Pos {
    fn sub_assign(&mut self, other: Self) {
        *self = Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        };
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[repr(C)]
pub struct AABB {
    pub pos: Pos,
    /// Top left + z
    pub bottom_right: Point,
}

impl AABB {
    pub fn new(pos: Pos, size: Scale) -> Self {
        Self {
            pos,
            bottom_right: Point {
                x: pos.x + size.width,
                y: pos.y + size.height,
            },
        }
    }

    pub fn width(&self) -> f32 {
        self.bottom_right.x - self.pos.x
    }

    pub fn height(&self) -> f32 {
        self.bottom_right.y - self.pos.y
    }

    pub fn size(&self) -> Scale {
        Scale {
            width: self.width(),
            height: self.height(),
        }
    }

    pub fn is_under(&self, p: Point) -> bool {
        p.x >= self.pos.x
            && p.x <= self.bottom_right.x
            && p.y >= self.pos.y
            && p.y <= self.bottom_right.y
    }

    pub fn translate_mut(&mut self, x: f32, y: f32) {
        self.pos.x += x;
        self.bottom_right.x += x;
        self.pos.y += y;
        self.bottom_right.y += y;
    }

    pub fn set_top_left_mut(&mut self, x: f32, y: f32) {
        let w = self.width();
        let h = self.height();
        self.pos.x = x;
        self.bottom_right.x = x + w;
        self.pos.y = y;
        self.bottom_right.y = y + h;
    }

    pub fn set_scale_mut(&mut self, w: f32, h: f32) {
        self.bottom_right.x = self.pos.x + w;
        self.bottom_right.y = self.pos.y + h;
    }

    pub fn round_mut(&mut self) {
        self.pos.x = self.pos.x.round();
        self.pos.y = self.pos.y.round();
        self.bottom_right.x = self.bottom_right.x.round();
        self.bottom_right.y = self.bottom_right.y.round();
    }

    pub fn translate(self, x: f32, y: f32) -> Self {
        Self {
            pos: Pos::new(self.pos.x + x, self.pos.y + y, self.pos.z),
            bottom_right: Point::new(self.bottom_right.x + x, self.bottom_right.y + y),
        }
    }

    pub fn set_top_left(self, x: f32, y: f32) -> Self {
        Self {
            pos: Pos::new(x, y, self.pos.z),
            bottom_right: Point::new(x + self.width(), y + self.height()),
        }
    }

    pub fn set_scale(self, w: f32, h: f32) -> Self {
        Self {
            pos: self.pos,
            bottom_right: Point::new(self.pos.x + w, self.pos.y + h),
        }
    }

    pub fn round(self) -> Self {
        Self {
            pos: Pos::new(self.pos.x.round(), self.pos.y.round(), self.pos.z),
            bottom_right: Point::new(self.bottom_right.x.round(), self.bottom_right.y.round()),
        }
    }

    pub fn to_origin(self) -> Self {
        Self {
            pos: Pos::default(),
            bottom_right: Point {
                x: self.width(),
                y: self.height(),
            },
        }
    }
}

impl Scalable for AABB {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            pos: self.pos.scale(scale_factor),
            bottom_right: self.bottom_right.scale(scale_factor),
        }
    }
}

impl MulAssign<f32> for AABB {
    fn mul_assign(&mut self, f: f32) {
        self.pos.x *= f;
        self.pos.y *= f;
        self.bottom_right.x *= f;
        self.bottom_right.y *= f;
    }
}

impl Mul<f32> for AABB {
    type Output = Self;
    fn mul(self, f: f32) -> Self {
        Self {
            pos: Pos {
                x: self.pos.x * f,
                y: self.pos.y * f,
                z: self.pos.z,
            },
            bottom_right: Point {
                x: self.bottom_right.x * f,
                y: self.bottom_right.y * f,
            },
        }
    }
}

impl Div<f32> for AABB {
    type Output = Self;
    fn div(self, f: f32) -> Self {
        Self {
            pos: Pos {
                x: self.pos.x / f,
                y: self.pos.y / f,
                z: self.pos.z,
            },
            bottom_right: Point {
                x: self.bottom_right.x / f,
                y: self.bottom_right.y / f,
            },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

impl Hash for Color {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ((self.r * 100000.0) as i32).hash(state);
        ((self.g * 100000.0) as i32).hash(state);
        ((self.b * 100000.0) as i32).hash(state);
        ((self.a * 100000.0) as i32).hash(state);
    }
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const YELLOW: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const MAGENTA: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

impl From<[f32; 4]> for Color {
    fn from(c: [f32; 4]) -> Self {
        unsafe { mem::transmute(c) }
    }
}

impl From<f32> for Color {
    fn from(c: f32) -> Self {
        Color::rgb(c, c, c)
    }
}

fn u8_to_norm(x: u8) -> f32 {
    x as f32 / 255.0
}

impl From<u32> for Color {
    fn from(c: u32) -> Self {
        let a = u8_to_norm(c as u8);
        let b = u8_to_norm((c >> 8) as u8);
        let g = u8_to_norm((c >> 16) as u8);
        let r = u8_to_norm((c >> 24) as u8);
        Color::new(r, g, b, a)
    }
}

#[macro_export]
macro_rules! color {
    ($r:expr, $g:expr, $b:expr) => {
        $crate::Color {
            r: $r as f32 / 255.0,
            g: $g as f32 / 255.0,
            b: $b as f32 / 255.0,
            a: 1.0,
        }
    };
    ($r:expr, $g:expr, $b:expr, $a:expr) => {
        $crate::Color {
            r: $r as f32 / 255.0,
            g: $g as f32 / 255.0,
            b: $b as f32 / 255.0,
            a: $a as f32 / 255.0,
        }
    };
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> Self {
        unsafe { mem::transmute(c) }
    }
}

impl From<&[f32; 4]> for Color {
    fn from(c: &[f32; 4]) -> Self {
        unsafe { mem::transmute(*c) }
    }
}

impl From<[f32; 3]> for Color {
    fn from(c: [f32; 3]) -> Self {
        Self {
            r: c[0],
            g: c[1],
            b: c[2],
            a: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from() {
        assert_eq!(
            Pos::from([1.0, 2.0, 3.0]),
            Pos {
                x: 1.0,
                y: 2.0,
                z: 3.0
            }
        );
    }
}
