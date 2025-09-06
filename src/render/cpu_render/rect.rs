use crate::base_types::{Color, Pos, Scale};

#[derive(Debug, PartialEq)]
pub struct Rect {
    pos: Pos,
    scale: Scale,
    color: Color,
}

impl Rect {
    pub fn new(pos: Pos, scale: Scale, color: Color) -> Self {
        Self { pos, scale, color }
    }
}
