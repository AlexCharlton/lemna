extern crate alloc;
use alloc::string::String;

use core::ops::{Add, AddAssign, Div, DivAssign, Sub, SubAssign};

pub const MIN_SIZE: Dimension = Dimension::Px(10.0);

//--------------------------------
// MARK: Types
//--------------------------------

#[derive(Clone, Copy, Debug, Default)]
pub struct ScrollPosition {
    pub x: Option<f32>,
    pub y: Option<f32>,
}

impl Div<f32> for ScrollPosition {
    type Output = Self;
    fn div(self, f: f32) -> Self {
        Self {
            x: self.x.map(|x| x / f),
            y: self.y.map(|y| y / f),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Default)]
pub enum Dimension {
    #[default]
    Auto,
    Px(f64),
    Pct(f64),
}

impl core::fmt::Debug for Dimension {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::Auto => write!(f, "Auto"),
            Self::Px(x) => write!(f, "{} px", x),
            Self::Pct(x) => write!(f, "{} %", x),
        }
    }
}

impl Dimension {
    /// Between two dimensions, return the most specific value
    pub fn most_specific(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Auto, _) => *other,
            (_, Self::Auto) => *self,
            (Self::Px(_), _) => *self,
            (_, Self::Px(_)) => *other,
            _ => *self,
        }
    }

    /// Between two dimensions, return the value of the second if the first is Auto, otherwise return the first value
    pub fn more_specific(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Auto, _) => *other,
            _ => *self,
        }
    }

    pub fn resolved(&self) -> bool {
        matches!(self, Self::Px(_))
    }

    pub fn min(&self, other: Self) -> Self {
        match (self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a.min(b)),
            (Self::Px(a), _) => Self::Px(*a),
            (_, Self::Px(b)) => Self::Px(b),
            _ => Dimension::Auto,
        }
    }

    pub(crate) fn maybe_resolve(&self, relative_to: &Self) -> Self {
        match self {
            Dimension::Px(px) => Dimension::Px(*px),
            Dimension::Pct(pct) => {
                if let Dimension::Px(px) = relative_to {
                    Dimension::Px(px * pct / 100.0)
                } else {
                    Dimension::Pct(*pct)
                }
            }
            Dimension::Auto => Dimension::Auto,
        }
    }

    pub fn max(&self, other: Self) -> Self {
        match (self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a.max(b)),
            (Self::Px(a), _) => Self::Px(*a),
            (_, Self::Px(b)) => Self::Px(b),
            _ => Dimension::Auto,
        }
    }

    pub fn maybe_px(&self) -> Option<f32> {
        match self {
            Self::Px(x) => Some(*x as f32),
            _ => None,
        }
    }

    pub fn is_pct(&self) -> bool {
        matches!(self, Self::Pct(_))
    }

    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }
}

impl Sub for Dimension {
    type Output = Dimension;

    fn sub(self, other: Self) -> Self {
        match (self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a - b),
            (Self::Pct(a), Self::Pct(b)) => Self::Pct(a - b),
            (s, _) => s,
        }
    }
}

impl SubAssign for Dimension {
    fn sub_assign(&mut self, other: Self) {
        let val = match (*self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a - b),
            (Self::Pct(a), Self::Pct(b)) => Self::Pct(a - b),
            (s, _) => s,
        };
        *self = val;
    }
}

impl Add for Dimension {
    type Output = Dimension;

    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a + b),
            (Self::Pct(a), Self::Pct(b)) => Self::Pct(a + b),
            (s, _) => s,
        }
    }
}

impl AddAssign for Dimension {
    fn add_assign(&mut self, other: Self) {
        let val = match (*self, other) {
            (Self::Px(a), Self::Px(b)) => Self::Px(a + b),
            (Self::Pct(a), Self::Pct(b)) => Self::Pct(a + b),
            (s, _) => s,
        };
        *self = val;
    }
}

impl DivAssign<f64> for Dimension {
    fn div_assign(&mut self, b: f64) {
        let val = match *self {
            Self::Px(a) => Self::Px(a / b),
            Self::Pct(a) => Self::Pct(a / b),
            s => s,
        };
        *self = val;
    }
}

impl From<Dimension> for f32 {
    fn from(d: Dimension) -> Self {
        match d {
            Dimension::Px(p) => p as f32,
            _ => 0.0,
        }
    }
}
impl From<Dimension> for f64 {
    fn from(d: Dimension) -> Self {
        match d {
            Dimension::Px(p) => p,
            _ => 0.0,
        }
    }
}

#[derive(Default, Copy, Clone, PartialEq)]
pub struct Size {
    pub width: Dimension,
    pub height: Dimension,
}

impl core::fmt::Debug for Size {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Size[{:?}, {:?}]", self.width, self.height)
    }
}

impl Size {
    pub fn new(width: Dimension, height: Dimension) -> Self {
        Self { width, height }
    }

    pub fn resolved(&self) -> bool {
        self.width.resolved() && self.height.resolved()
    }

    pub fn most_specific(&self, other: &Self) -> Self {
        Self {
            width: self.width.most_specific(&other.width),
            height: self.height.most_specific(&other.height),
        }
    }

    pub fn more_specific(&self, other: &Self) -> Self {
        Self {
            width: self.width.more_specific(&other.width),
            height: self.height.more_specific(&other.height),
        }
    }

    pub fn main(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.width,
            Direction::Column => self.height,
        }
    }

    pub fn cross(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.height,
            Direction::Column => self.width,
        }
    }

    pub fn main_mut(&mut self, dir: Direction) -> &mut Dimension {
        match dir {
            Direction::Row => &mut self.width,
            Direction::Column => &mut self.height,
        }
    }

    pub fn cross_mut(&mut self, dir: Direction) -> &mut Dimension {
        match dir {
            Direction::Row => &mut self.height,
            Direction::Column => &mut self.width,
        }
    }

    pub fn maybe_resolve(&self, relative_to: &Self) -> Self {
        Self {
            width: self.width.maybe_resolve(&relative_to.width),
            height: self.height.maybe_resolve(&relative_to.height),
        }
    }

    pub fn minus_bounds(&self, bounds: &Bounds) -> Self {
        Self {
            width: self.width - bounds.left - bounds.right,
            height: self.height - bounds.top - bounds.bottom,
        }
    }

    pub fn plus_bounds(&self, bounds: &Bounds) -> Self {
        Self {
            width: self.width + bounds.left + bounds.right,
            height: self.height + bounds.top + bounds.bottom,
        }
    }

    pub fn min(&self, other: Self) -> Self {
        Self {
            width: self.width.min(other.width),
            height: self.height.min(other.height),
        }
    }
}

impl From<ScrollPosition> for Size {
    fn from(p: ScrollPosition) -> Self {
        Self {
            width: Dimension::Px(p.x.unwrap_or(0.0).into()),
            height: Dimension::Px(p.y.unwrap_or(0.0).into()),
        }
    }
}

#[derive(Default, Copy, Clone, PartialEq)]
pub struct Bounds {
    pub left: Dimension,
    pub right: Dimension,
    pub top: Dimension,
    pub bottom: Dimension,
}

impl core::fmt::Debug for Bounds {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "Rect[l:{:?}, r:{:?}, t:{:?}, b:{:?}]",
            self.left, self.right, self.top, self.bottom
        )
    }
}

impl Bounds {
    const ZERO: Self = Self {
        left: Dimension::Px(0.0),
        right: Dimension::Px(0.0),
        top: Dimension::Px(0.0),
        bottom: Dimension::Px(0.0),
    };

    pub(crate) fn maybe_resolve(&self, relative_to: &Size) -> Self {
        Self {
            left: self.left.maybe_resolve(&relative_to.width),
            right: self.right.maybe_resolve(&relative_to.width),
            top: self.top.maybe_resolve(&relative_to.height),
            bottom: self.bottom.maybe_resolve(&relative_to.height),
        }
    }

    pub fn width_total(&self) -> Dimension {
        self.left + self.right
    }

    pub fn height_total(&self) -> Dimension {
        self.top + self.bottom
    }

    pub fn main(&self, dir: Direction, align: Alignment) -> Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => self.right,
            (Direction::Row, _) => self.left,
            (Direction::Column, Alignment::End) => self.bottom,
            (Direction::Column, _) => self.top,
        }
    }

    pub fn main_total(&self, dir: Direction) -> Dimension {
        match dir {
            Direction::Row => self.left + self.right,
            Direction::Column => self.top + self.bottom,
        }
    }

    pub fn main_mut(&mut self, dir: Direction, align: Alignment) -> &mut Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => &mut self.right,
            (Direction::Row, _) => &mut self.left,
            (Direction::Column, Alignment::End) => &mut self.bottom,
            (Direction::Column, _) => &mut self.top,
        }
    }

    pub fn main_reverse(&self, dir: Direction, align: Alignment) -> Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => self.left,
            (Direction::Row, _) => self.right,
            (Direction::Column, Alignment::End) => self.top,
            (Direction::Column, _) => self.bottom,
        }
    }

    pub fn cross(&self, dir: Direction, align: Alignment) -> Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => self.bottom,
            (Direction::Row, _) => self.top,
            (Direction::Column, Alignment::End) => self.right,
            (Direction::Column, _) => self.left,
        }
    }

    pub fn cross_mut(&mut self, dir: Direction, align: Alignment) -> &mut Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => &mut self.bottom,
            (Direction::Row, _) => &mut self.top,
            (Direction::Column, Alignment::End) => &mut self.right,
            (Direction::Column, _) => &mut self.left,
        }
    }

    pub fn cross_reverse(&self, dir: Direction, align: Alignment) -> Dimension {
        match (dir, align) {
            (Direction::Row, Alignment::End) => self.top,
            (Direction::Row, _) => self.bottom,
            (Direction::Column, Alignment::End) => self.left,
            (Direction::Column, _) => self.right,
        }
    }

    pub fn most_specific(&self, other: &Self) -> Self {
        let top = if self.top.resolved() {
            self.top
        } else if other.top.resolved() && !self.bottom.resolved() {
            other.top
        } else {
            self.top
        };
        let bottom = if self.bottom.resolved() {
            self.bottom
        } else if other.bottom.resolved() && !self.top.resolved() {
            other.bottom
        } else {
            self.bottom
        };
        let left = if self.left.resolved() {
            self.left
        } else if other.left.resolved() && !self.right.resolved() {
            other.left
        } else {
            self.left
        };
        let right = if self.right.resolved() {
            self.right
        } else if other.right.resolved() && !self.left.resolved() {
            other.right
        } else {
            self.right
        };
        Self {
            top,
            left,
            bottom,
            right,
        }
    }

    // fn minus_size(&self, size: Size) -> Self {
    //     let top = if self.top.resolved() && size.height.resolved() {
    //         Dimension::Px(f32::from(self.top) - f32::from(size.height))
    //     } else {
    //         self.top
    //     };
    //     let bottom = if self.bottom.resolved() && size.height.resolved() {
    //         Dimension::Px(f32::from(self.bottom) - f32::from(size.height))
    //     } else {
    //         self.bottom
    //     };
    //     let left = if self.left.resolved() && size.width.resolved() {
    //         Dimension::Px(f32::from(self.left) - f32::from(size.width))
    //     } else {
    //         self.left
    //     };
    //     let right = if self.right.resolved() && size.width.resolved() {
    //         Dimension::Px(f32::from(self.right) - f32::from(size.width))
    //     } else {
    //         self.right
    //     };
    //     Self {
    //         top,
    //         left,
    //         bottom,
    //         right,
    //     }
    // }

    // fn plus_size(&self, size: Size) -> Self {
    //     let top = if self.top.resolved() && size.height.resolved() {
    //         Dimension::Px(f32::from(self.top) + f32::from(size.height))
    //     } else {
    //         self.top
    //     };
    //     let bottom = if self.bottom.resolved() && size.height.resolved() {
    //         Dimension::Px(f32::from(self.bottom) + f32::from(size.height))
    //     } else {
    //         self.bottom
    //     };
    //     let left = if self.left.resolved() && size.width.resolved() {
    //         Dimension::Px(f32::from(self.left) + f32::from(size.width))
    //     } else {
    //         self.left
    //     };
    //     let right = if self.right.resolved() && size.width.resolved() {
    //         Dimension::Px(f32::from(self.right) + f32::from(size.width))
    //     } else {
    //         self.right
    //     };
    //     Self {
    //         top,
    //         left,
    //         bottom,
    //         right,
    //     }
    // }
}

impl Add for Bounds {
    type Output = Bounds;

    fn add(self, other: Self) -> Self {
        Self {
            top: self.top + other.top,
            left: self.left + other.left,
            bottom: self.bottom + other.bottom,
            right: self.right + other.right,
        }
    }
}

impl AddAssign for Bounds {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            top: self.top + other.top,
            left: self.left + other.left,
            bottom: self.bottom + other.bottom,
            right: self.right + other.right,
        };
    }
}

impl From<crate::base_types::Point> for Bounds {
    fn from(p: crate::base_types::Point) -> Self {
        Self {
            top: Dimension::Px(p.y.into()),
            left: Dimension::Px(p.x.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum Direction {
    #[default]
    Row,
    Column,
}

impl Direction {
    pub fn size(&self, main: Dimension, cross: Dimension) -> Size {
        match self {
            Self::Row => Size {
                width: main,
                height: cross,
            },
            Self::Column => Size {
                width: cross,
                height: main,
            },
        }
    }

    pub fn rect(
        &self,
        main: Dimension,
        cross: Dimension,
        axis_alignment: Alignment,
        cross_alignment: Alignment,
    ) -> Bounds {
        let mut rect = Bounds::default();

        match (self, axis_alignment) {
            (Direction::Row, Alignment::End) => rect.right = main,
            (Direction::Row, _) => rect.left = main,
            (Direction::Column, Alignment::End) => rect.bottom = main,
            (Direction::Column, _) => rect.top = main,
        }

        match (self, cross_alignment) {
            (Direction::Row, Alignment::End) => rect.bottom = cross,
            (Direction::Row, _) => rect.top = cross,
            (Direction::Column, Alignment::End) => rect.right = cross,
            (Direction::Column, _) => rect.left = cross,
        }

        rect
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum PositionType {
    Absolute,
    #[default]
    Relative,
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum Alignment {
    #[default]
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub direction: Direction,
    pub wrap: bool,
    pub position: Bounds,
    pub position_type: PositionType,
    pub axis_alignment: Alignment,
    pub cross_alignment: Alignment,
    pub margin: Bounds,
    pub padding: Bounds,
    pub size: Size,
    // TODO employ this more consistently
    pub max_size: Size,
    pub min_size: Size,
    pub flex_grow: f64,
    pub z_index: Option<f64>,
    pub z_index_increment: f64,
    pub debug: Option<String>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            direction: Default::default(),
            wrap: false,
            position: Default::default(),
            position_type: Default::default(),
            axis_alignment: Default::default(),
            cross_alignment: Default::default(),
            margin: Bounds::ZERO,
            padding: Bounds::ZERO,
            size: Default::default(),
            max_size: Default::default(),
            min_size: Size {
                width: MIN_SIZE,
                height: MIN_SIZE,
            },
            flex_grow: 1.0,
            z_index: None,
            z_index_increment: 0.0,
            debug: None,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub enum LayoutType {
    #[default]
    Auto,
    Fixed,
    Percent,
    Flex,
    Wrapping,
    Intrinsic,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct LayoutResult {
    pub size: Size,
    pub position: Bounds,
    // Direction of the main axis for this layout result, i.e. the parent's direction
    pub(crate) direction: Direction,
    pub(crate) main_layout_type: LayoutType,
    // Used by the layout engine to track if this node's layout has been resolved
    pub(crate) main_resolved: bool,
}

impl From<LayoutResult> for crate::base_types::Rect {
    fn from(p: LayoutResult) -> Self {
        Self::new(
            crate::base_types::Pos::new(p.position.left.into(), p.position.top.into(), 0.0),
            crate::base_types::Scale::new(p.size.width.into(), p.size.height.into()),
        )
    }
}
