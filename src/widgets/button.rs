use std::time::Instant;

use super::{TextSegment, ToolTip, ToolTipStyle};
use crate::base_types::*;
use crate::component::{Component, Message};
use crate::event;
use crate::font_cache::HorizontalAlign;
use crate::layout::*;
use crate::{node, Node};
use lemna_macros::{state_component, state_component_impl};

#[derive(Debug, Default)]
struct ButtonState {
    hover: bool,
    pressed: bool,
    tool_tip_open: Option<Point>,
    hover_start: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub text_color: Color,
    pub font_size: f32,
    pub font: Option<String>,
    pub background_color: Color,
    pub highlight_color: Color,
    pub active_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radius: f32,
    pub padding: f32,
    pub tool_tip_style: ToolTipStyle,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            text_color: 0.0.into(),
            font_size: 12.0,
            font: None,
            background_color: 1.0.into(),
            highlight_color: 0.9.into(),
            active_color: 0.7.into(),
            border_color: 0.0.into(),
            border_width: 2.0,
            radius: 4.0,
            padding: 2.0,
            tool_tip_style: Default::default(),
        }
    }
}

#[state_component(ButtonState)]
pub struct Button {
    pub label: Vec<TextSegment>,
    pub style: ButtonStyle,
    pub on_click: Option<Box<dyn Fn() -> Message + Send + Sync>>,
    pub tool_tip: Option<String>,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("label", &self.label)
            .field("style", &self.style)
            .finish()
    }
}

impl Button {
    pub fn new(label: Vec<TextSegment>, style: ButtonStyle) -> Self {
        Self {
            label,
            style,
            on_click: None,
            tool_tip: None,
            state: Some(ButtonState::default()),
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
        let mut base = node!(
            super::RoundedRect {
                background_color: if self.state_ref().pressed {
                    self.style.active_color
                } else if self.state_ref().hover {
                    self.style.highlight_color
                } else {
                    self.style.background_color
                },
                border_color: self.style.border_color,
                border_width: self.style.border_width,
                radius: (
                    self.style.radius,
                    self.style.radius,
                    self.style.radius,
                    self.style.radius
                ),
            },
            lay!(
                size: size_pct!(100.0),
                padding: rect!(self.style.padding),
                cross_alignment: crate::layout::Alignment::Center,
                axis_alignment: crate::layout::Alignment::Center
            )
        )
        .push(node!(super::Text::new(
            self.label.clone(),
            super::TextStyle {
                size: self.style.font_size,
                color: self.style.text_color,
                font: self.style.font.clone(),
                h_alignment: HorizontalAlign::Center,
            }
        )));

        if let (Some(p), Some(tt)) = (self.state_ref().tool_tip_open, self.tool_tip.as_ref()) {
            base = base.push(node!(
                ToolTip {
                    tool_tip: tt.clone(),
                    style: self.style.tool_tip_style.clone(),
                },
                lay!(position_type: PositionType::Absolute,
                     z_index_increment: 1000.0,
                     position: (p + ToolTip::MOUSE_OFFSET).into(),
                ),
                1
            ));
        }

        Some(base)
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) -> Vec<Message> {
        self.state_mut().hover_start = Some(Instant::now());
        event.stop_bubbling();
        vec![]
    }

    fn on_mouse_enter(&mut self, event: &mut event::Event<event::MouseEnter>) -> Vec<Message> {
        self.state_mut().hover = true;
        if let Some(w) = crate::current_window() {
            w.set_cursor("PointingHand");
        }
        event.dirty();
        vec![]
    }

    fn on_mouse_leave(&mut self, event: &mut event::Event<event::MouseLeave>) -> Vec<Message> {
        self.state = Some(ButtonState::default());
        if let Some(w) = crate::current_window() {
            w.unset_cursor();
        }
        event.dirty();
        vec![]
    }

    fn on_tick(&mut self, event: &mut event::Event<event::Tick>) -> Vec<Message> {
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
        vec![]
    }

    fn on_mouse_down(&mut self, event: &mut event::Event<event::MouseDown>) -> Vec<Message> {
        self.state_mut().pressed = true;
        event.dirty();
        vec![]
    }

    fn on_mouse_up(&mut self, event: &mut event::Event<event::MouseUp>) -> Vec<Message> {
        self.state_mut().pressed = false;
        event.dirty();
        vec![]
    }

    fn on_click(&mut self, _event: &mut event::Event<event::Click>) -> Vec<Message> {
        let mut m: Vec<Message> = vec![];
        if let Some(f) = &self.on_click {
            m.push(f());
        }
        m
    }
}
