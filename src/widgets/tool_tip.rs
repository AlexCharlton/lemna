use crate::base_types::*;
use crate::component::Component;
use crate::font_cache::HorizontalAlign;
use crate::style::Styled;
use crate::{node, txt, Node};
use lemna_macros::component;

#[component(Styled, Internal)]
#[derive(Debug)]
pub struct ToolTip {
    pub tool_tip: String,
}

impl ToolTip {
    const MAX_WIDTH: f32 = 300.0;
    pub(crate) const MOUSE_OFFSET: Point = Point { x: 14.0, y: 0.0 };
    pub(crate) const DELAY: u128 = 1000; // millis

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
                    padding: rect!(padding),
                    max_size: size!(ToolTip::MAX_WIDTH, Auto),
                )
            )
            .push(node!(super::Text::new(txt!(self.tool_tip.clone()))
                .style("size", self.style_val("font_size").unwrap())
                .style("color", self.style_val("text_color").unwrap())
                .style("h_alignment", HorizontalAlign::Left.into())
                .maybe_style("font", self.style_val("font")))),
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
