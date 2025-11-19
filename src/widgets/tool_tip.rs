extern crate alloc;

use alloc::{boxed::Box, string::String, vec, vec::Vec};

use crate::base_types::*;
use crate::component::Component;
use crate::style::{HorizontalPosition, Styled};
use crate::{Node, node, txt};
use lemna_macros::component;

#[component(Styled, Internal)]
#[derive(Debug)]
pub struct ToolTip {
    pub tool_tip: String,
}

impl ToolTip {
    const MAX_WIDTH: f32 = 300.0;
    pub(crate) const MOUSE_OFFSET: Point = Point { x: 14.0, y: 0.0 };
    pub(crate) const DELAY: i64 = 1000; // millis

    pub fn new(tool_tip: String) -> Self {
        Self {
            tool_tip,
            class: Default::default(),
            style_overrides: Default::default(),
        }
    }
}

impl Component for ToolTip {
    fn view(&self) -> Option<Node> {
        let background_color: Color = self.style_val("background_color").into();
        let border_color: Color = self.style_val("border_color").into();
        let border_width: f32 = self.style_val("border_width").unwrap().f32();
        let padding: f32 = self.style_val("padding").unwrap().f32();

        Some(
            node!(
                super::Div::new()
                    .bg(background_color)
                    .border(border_color, border_width),
                lay!(
                    padding: bounds!(padding),
                    max_size: size!(ToolTip::MAX_WIDTH, Auto),
                )
            )
            .push(node!(
                super::Text::new(txt!(self.tool_tip.clone()))
                    .style("size", self.style_val("font_size").unwrap())
                    .style("color", self.style_val("text_color").unwrap())
                    .style("h_alignment", HorizontalPosition::Left)
                    .maybe_style("font", self.style_val("font"))
            )),
        )
    }

    fn full_control(&self) -> bool {
        true
    }

    fn set_aabb(
        &mut self,
        aabb: &mut Rect,
        _parent_aabb: Rect,
        _children: Vec<(&mut Rect, Option<Scale>, Option<Point>)>,
        frame: Rect,
        _scale_factor: f32,
    ) {
        if aabb.bottom_right.y > frame.bottom_right.y {
            if aabb.pos.y - aabb.height() > 0.0 {
                // Flip up if there isn't enough room underneath, but there is sufficient room above
                aabb.translate_mut(0.0, -aabb.height());
            } else {
                // Otherwise, offset the tooltip from the bottom to as much as possible
                aabb.pos.y = (frame.pos.y - aabb.height()).max(0.0);
                aabb.bottom_right.y = frame.bottom_right.y;
            }
        }

        if aabb.bottom_right.x > frame.bottom_right.x {
            if aabb.pos.x - aabb.width() > 0.0 {
                // Flip left if there isn't enough room to the right
                aabb.translate_mut(-aabb.width() - Self::MOUSE_OFFSET.x * 2.0, 0.0);
            } else {
                // Otherwise, offset the tooltip from the right to as much as possible
                aabb.pos.x = (frame.pos.x - aabb.width()).max(0.0);
                aabb.bottom_right.x = frame.bottom_right.x;
            }
        }
    }
}
