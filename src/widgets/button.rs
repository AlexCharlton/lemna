extern crate alloc;

use crate::time::Instant;
use alloc::{boxed::Box, string::String, vec::Vec};

use super::ToolTip;
use crate::Node;
use crate::base_types::*;
use crate::component::{Component, Message};
use crate::event;
use crate::font_cache::TextSegment;
use crate::input::Key;
use crate::layout::*;
use crate::style::{HorizontalPosition, Styled};
use lemna_macros::{component, state_component_impl};

#[derive(Debug, Default)]
struct ButtonState {
    hover: bool,
    pressed: bool,
    focused: bool,
    tool_tip_open: Option<Point>,
    hover_start: Option<Instant>,
}

#[component(State = "ButtonState", Styled, Internal)]
pub struct Button {
    pub label: Vec<TextSegment>,
    pub on_click: Option<Box<dyn Fn() -> Message + Send + Sync>>,
    pub tool_tip: Option<String>,
    focus_on_click: bool,
    ignore_key_events: bool,
    disabled: bool,
    unfocus_on_escape: bool,
}

impl core::fmt::Debug for Button {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("Button")
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
            ignore_key_events: false,
            focus_on_click: false,
            disabled: false,
            state: Some(ButtonState::default()),
            dirty: crate::Dirty::No,
            class: Default::default(),
            style_overrides: Default::default(),
            unfocus_on_escape: true,
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

    pub fn focus_on_click(mut self) -> Self {
        self.focus_on_click = true;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn ignore_escape(mut self) -> Self {
        self.unfocus_on_escape = false;
        self
    }

    pub fn ignore_key_events(mut self) -> Self {
        self.ignore_key_events = true;
        self
    }
}

#[state_component_impl(ButtonState, Internal)]
impl Component for Button {
    fn view(&self) -> Option<Node> {
        let radius: f32 = self.style_val("radius").unwrap().f32();
        let padding: f64 = self.style_val("padding").unwrap().into();
        let active_color: Color = self.style_val("active_color").into();
        let highlight_color: Color = self.style_val("highlight_color").into();
        let background_color: Color = self.style_val("background_color").into();
        let border_color: Color = self.style_val("border_color").into();
        let border_width: f32 = self.style_val("border_width").unwrap().f32();
        let focus_border_width: f32 = self.style_val("focus_border_width").unwrap().f32();
        let text_color: Color = self.style_val("text_color").unwrap().into();

        let (bg_color, txt_color, brd_color) = if self.disabled {
            (
                self.style_val("disabled_background_color").unwrap().into(),
                self.style_val("disabled_text_color").unwrap().into(),
                self.style_val("disabled_border_color").unwrap().into(),
            )
        } else {
            let bg = if self.state_ref().pressed {
                active_color
            } else if self.state_ref().hover {
                highlight_color
            } else {
                background_color
            };
            (bg, text_color, border_color)
        };

        let mut base = node!(
            super::RoundedRect {
                background_color: bg_color,
                border_color: brd_color,
                border_width: if self.state_ref().focused && !self.disabled {
                    focus_border_width
                } else {
                    border_width
                },
                radii: BorderRadii::all(radius),
            },
            lay!(
                size: size_pct!(100.0),
                padding: bounds!(padding),
                cross_alignment: crate::layout::Alignment::Center,
                axis_alignment: crate::layout::Alignment::Center,
            )
        )
        .push(node!(
            super::Text::new(self.label.clone())
                .style("size", self.style_val("font_size").unwrap())
                .style("color", txt_color)
                .style("h_alignment", HorizontalPosition::Center)
                .maybe_style("font", self.style_val("font"))
        ));

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
        if self.disabled {
            return;
        }
        self.state_mut().hover_start = Some(Instant::now());
        // This state mutation should not trigger a redraw.
        self.dirty = crate::Dirty::No;
        event.stop_bubbling();
    }

    fn on_mouse_enter(&mut self, _event: &mut event::Event<event::MouseEnter>) {
        if self.disabled {
            return;
        }
        self.state_mut().hover = true;
        crate::window::set_cursor("PointingHand");
    }

    fn on_mouse_leave(&mut self, _event: &mut event::Event<event::MouseLeave>) {
        if self.disabled {
            return;
        }
        let focused = self.state_ref().focused;
        *self.state_mut() = ButtonState {
            focused,
            ..Default::default()
        };
        crate::window::unset_cursor();
    }

    fn on_tick(&mut self, event: &mut event::Event<event::Tick>) {
        if self.disabled {
            return;
        }
        if self.tool_tip.is_some()
            && self.state_ref().hover
            && self
                .state_ref()
                .hover_start
                .map(|s| s.elapsed().as_millis() > ToolTip::DELAY)
                .unwrap_or(false)
            && self.state_ref().tool_tip_open.is_none()
        {
            self.state_mut().tool_tip_open = Some(event.relative_logical_position());
        }
    }

    fn on_mouse_down(&mut self, _event: &mut event::Event<event::MouseDown>) {
        if self.disabled {
            return;
        }
        self.state_mut().pressed = true;
    }

    fn on_mouse_up(&mut self, _event: &mut event::Event<event::MouseUp>) {
        if self.disabled {
            return;
        }
        self.state_mut().pressed = false;
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) {
        if self.disabled {
            return;
        }
        if let Some(f) = &self.on_click {
            event.emit(f());
            event.stop_bubbling();
        }
        if self.focus_on_click {
            event.focus();
        }
    }

    fn on_double_click(&mut self, event: &mut event::Event<event::DoubleClick>) {
        if self.disabled {
            return;
        }
        if let Some(f) = &self.on_click {
            event.emit(f());
            event.stop_bubbling();
        }
        if self.focus_on_click {
            event.focus();
        }
    }

    fn on_focus(&mut self, event: &mut event::Event<event::Focus>) {
        if self.disabled {
            // Blur immediately if disabled
            event.blur();
            return;
        }
        self.state_mut().focused = true;
    }

    fn on_blur(&mut self, _event: &mut crate::Event<event::Blur>) {
        self.state_mut().focused = false;
    }

    fn on_key_down(&mut self, event: &mut crate::Event<event::KeyDown>) {
        if self.disabled || self.ignore_key_events {
            return;
        }
        match event.input.key {
            Key::Return => {
                if let Some(f) = &self.on_click {
                    event.emit(f());
                }
                event.stop_bubbling();
            }
            Key::Escape => {
                if self.unfocus_on_escape {
                    event.blur();
                    event.stop_bubbling();
                }
            }
            _ => {}
        }
    }
}
