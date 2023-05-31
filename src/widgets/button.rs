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
        let radius: f32 = self.style_val("radius").unwrap().f32();
        let padding: f64 = self.style_val("padding").unwrap().into();
        let active_color: Color = self.style_val("active_color").into();
        let highlight_color: Color = self.style_val("highlight_color").into();
        let background_color: Color = self.style_val("background_color").into();
        let border_color: Color = self.style_val("border_color").into();
        let border_width: f32 = self.style_val("border_width").unwrap().f32();

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
        .push(node!(super::Text::new(self.label.clone())
            .style("size", self.style_val("font_size").unwrap())
            .style("color", self.style_val("text_color").unwrap())
            .style("h_alignment", HorizontalAlign::Center.into())
            .maybe_style("font", self.style_val("font"))));

        if let (Some(p), Some(tt)) = (self.state_ref().tool_tip_open, self.tool_tip.as_ref()) {
            base = base.push(node!(
                ToolTip::new(tt.clone()),
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
