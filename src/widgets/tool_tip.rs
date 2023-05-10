use crate::base_types::*;
use crate::component::Component;
use crate::font_cache::HorizontalAlign;
use crate::{node, txt, Node};

#[derive(Debug, Clone)]
pub struct ToolTipStyle {
    pub text_color: Color,
    pub font_size: f32,
    pub font: Option<String>,
    pub background_color: Color,
    pub border_color: Color,
    pub border_width: f32,
}

impl Default for ToolTipStyle {
    fn default() -> Self {
        Self {
            text_color: Color::BLACK,
            font_size: 12.0,
            font: None,
            background_color: Color::WHITE,
            border_color: Color::BLACK,
            border_width: 1.0,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ToolTip {
    pub tool_tip: String,
    pub style: ToolTipStyle,
}

impl ToolTip {
    const MAX_WIDTH: f32 = 300.0;
    pub(crate) const MOUSE_OFFSET: Point = Point { x: 14.0, y: 0.0 };
    pub(crate) const DELAY: u128 = 1000; // millis
}

impl Component for ToolTip {
    fn view(&self) -> Option<Node> {
        Some(
            node!(
                super::Div::new()
                    .bg(self.style.background_color)
                    .border(self.style.border_color, self.style.border_width),
                lay!(
                    padding: rect!(2.0),
                    max_size: size!(ToolTip::MAX_WIDTH, Auto),
                )
            )
            .push(node!(super::Text::new(
                txt!(self.tool_tip.clone()),
                super::TextStyle {
                    size: self.style.font_size,
                    color: self.style.text_color,
                    font: self.style.font.clone(),
                    h_alignment: HorizontalAlign::Left,
                }
            ))),
        )
    }

    fn full_control(&self) -> bool {
        true
    }

    fn set_aabb(
        &mut self,
        aabb: &mut AABB,
        _parent_aabb: AABB,
        _children: Vec<(&mut AABB, Option<Scale>, Option<Point>)>,
        frame: AABB,
        _scale_factor: f32,
    ) {
        if aabb.bottom_right.y > frame.bottom_right.y {
            // Flip up if there isn't enough room underneath
            aabb.translate_mut(0.0, -aabb.height());
        }

        if aabb.bottom_right.x > frame.bottom_right.x {
            // Flip left if there isn't enough room to the right
            aabb.translate_mut(-aabb.width() - Self::MOUSE_OFFSET.x * 2.0, 0.0);
        }
    }
}
