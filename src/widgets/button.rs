use std::time::Instant;

use super::{TextSegment, ToolTip};
use crate::base_types::*;
use crate::component::{Component, Message};
use crate::event;
use crate::font_cache::HorizontalAlign;
use crate::layout::*;
use crate::style::Styled;
use crate::{node, Node};
use lemna_macros::{component, state_component_impl};

#[derive(Debug, Default)]
struct ButtonState {
    hover: bool,
    pressed: bool,
    tool_tip_open: Option<Point>,
    hover_start: Option<Instant>,
}

// #[derive(Debug, Clone)]
// pub struct ButtonStyle {
//     pub text_color: Color,
//     pub font_size: f32,
//     pub font: Option<String>,
//     pub background_color: Color,
//     pub highlight_color: Color,
//     pub active_color: Color,
//     pub border_color: Color,
//     pub border_width: f32,
//     pub radius: f32,
//     pub padding: f32,
//     pub tool_tip_style: ToolTipStyle,
// }

// impl Default for ButtonStyle {
//     fn default() -> Self {
//         Self {
//             text_color: Color::BLACK,
//             font_size: 12.0,
//             font: None,
//             background_color: Color::WHITE,
//             highlight_color: Color::LIGHT_GREY,
//             active_color: Color::MID_GREY,
//             border_color: Color::BLACK,
//             border_width: 2.0,
//             radius: 4.0,
//             padding: 2.0,
//             tool_tip_style: Default::default(),
//         }
//     }
// }

#[component(State = "ButtonState", Styled, Internal)]
pub struct Button {
    pub label: Vec<TextSegment>,
    pub on_click: Option<Box<dyn Fn() -> Message + Send + Sync>>,
    pub tool_tip: Option<String>,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("label", &self.label)
            .finish()
    }
}

impl Button {
    pub fn new(label: Vec<TextSegment>) -> Self {
        Self {
            label,
            on_click: None,
            tool_tip: None,
            state: Some(ButtonState::default()),
            class: Default::default(),
            style_overrides: Default::default(),
        }
    }

    pub fn on_click(mut self, f: Box<dyn Fn() -> Message + Send + Sync>) -> Self {
        self.on_click = Some(f);
        self
    }

    pub fn tool_tip(mut self, t: String) -> Self {
        self.tool_tip = Some(t);
        self
    }
}

#[state_component_impl(ButtonState)]
impl Component for Button {
    fn view(&self) -> Option<Node> {
        let radius: f32 = self.style_param("radius").unwrap().f32();
        let padding: f64 = self.style_param("padding").unwrap().into();
        let font_size: f32 = self.style_param("font_size").unwrap().f32();
        let active_color: Color = self.style_param("active_color").into();
        let highlight_color: Color = self.style_param("highlight_color").into();
        let background_color: Color = self.style_param("background_color").into();
        let border_color: Color = self.style_param("border_color").into();
        let text_color: Color = self.style_param("text_color").into();
        let border_width: f32 = self.style_param("border_width").unwrap().f32();
        let font = self.style_param("font").map(|p| p.str().to_string());

        let mut base = node!(
            super::RoundedRect {
                background_color: if self.state_ref().pressed {
                    active_color
                } else if self.state_ref().hover {
                    highlight_color
                } else {
                    background_color
                },
                border_color,
                border_width: border_width as f32,
                radius: (radius, radius, radius, radius),
            },
            lay!(
                size: size_pct!(100.0),
                padding: rect!(padding),
                cross_alignment: crate::layout::Alignment::Center,
                axis_alignment: crate::layout::Alignment::Center
            )
        )
        .push(node!(super::Text::new(
            self.label.clone(),
            super::TextStyle {
                size: font_size,
                color: text_color,
                font,
                h_alignment: HorizontalAlign::Center,
            }
        )));

        if let (Some(p), Some(tt)) = (self.state_ref().tool_tip_open, self.tool_tip.as_ref()) {
            base = base.push(node!(
                ToolTip {
                    tool_tip: tt.clone(),
                    style: Default::default(),
                },
                lay!(position_type: PositionType::Absolute,
                     z_index_increment: 1000.0,
                     position: (p + ToolTip::MOUSE_OFFSET).into(),
                ),
            ));
        }

        Some(base)
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) {
        self.state_mut().hover_start = Some(Instant::now());
        event.stop_bubbling();
    }

    fn on_mouse_enter(&mut self, event: &mut event::Event<event::MouseEnter>) {
        self.state_mut().hover = true;
        if let Some(w) = crate::current_window() {
            w.set_cursor("PointingHand");
        }
        event.dirty();
    }

    fn on_mouse_leave(&mut self, event: &mut event::Event<event::MouseLeave>) {
        self.state = Some(ButtonState::default());
        if let Some(w) = crate::current_window() {
            w.unset_cursor();
        }
        event.dirty();
    }

    fn on_tick(&mut self, event: &mut event::Event<event::Tick>) {
        if self.state_ref().hover_start.is_some()
            && self
                .state_ref()
                .hover_start
                .map(|s| s.elapsed().as_millis() > ToolTip::DELAY)
                .unwrap_or(false)
        {
            self.state_mut().tool_tip_open = Some(event.relative_logical_position());
            event.dirty();
        }
    }

    fn on_mouse_down(&mut self, event: &mut event::Event<event::MouseDown>) {
        self.state_mut().pressed = true;
        event.dirty();
    }

    fn on_mouse_up(&mut self, event: &mut event::Event<event::MouseUp>) {
        self.state_mut().pressed = false;
        event.dirty();
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) {
        if let Some(f) = &self.on_click {
            event.emit(f());
        }
    }
}
