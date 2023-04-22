use std::fmt;
use std::hash::Hash;
use std::time::Instant;

use super::{ButtonStyle, TextSegment, ToolTip};
use crate::base_types::*;
use crate::component::{Component, ComponentHasher, Message};
use crate::event;
use crate::font_cache::HorizontalAlign;
use crate::layout::*;
use crate::render::wgpu::WGPURenderer;
use crate::{node, Node};
use lemna_macros::{state_component, state_component_impl};

pub struct RadioButtons {
    buttons: Vec<Vec<TextSegment>>,
    tool_tips: Option<Vec<String>>,
    selected: Vec<usize>,
    style: ButtonStyle,
    direction: Direction,
    max_rows: Option<usize>,
    max_columns: Option<usize>,
    /// Can more than one button be selected at a time? Implies `nullable`
    multi_select: bool,
    /// Does clicking on a selected button clear it?
    nullable: bool,
    on_change: Option<Box<dyn Fn(Vec<usize>) -> Message + Send + Sync>>,
}

impl fmt::Debug for RadioButtons {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RadioButtons")
            .field("buttons", &self.buttons)
            .field("style", &self.style)
            .field("selected", &self.selected)
            .finish()
    }
}

enum RadioButtonMsg {
    Clicked(usize),
}

impl RadioButtons {
    pub fn new(buttons: Vec<Vec<TextSegment>>, selected: Vec<usize>, style: ButtonStyle) -> Self {
        Self {
            buttons,
            tool_tips: None,
            selected,
            style,
            direction: Direction::Row,
            max_rows: None,
            max_columns: None,
            on_change: None,
            multi_select: false,
            nullable: false,
        }
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn max_rows(mut self, max_rows: usize) -> Self {
        self.max_rows = Some(max_rows);
        self
    }

    pub fn max_columns(mut self, max_columns: usize) -> Self {
        self.max_columns = Some(max_columns);
        self
    }

    pub fn nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    pub fn multi_select(mut self, multi_select: bool) -> Self {
        self.multi_select = multi_select;
        self.nullable = multi_select;
        self
    }

    pub fn on_change(
        mut self,
        change_fn: Box<dyn Fn(Vec<usize>) -> Message + Send + Sync>,
    ) -> Self {
        self.on_change = Some(change_fn);
        self
    }

    pub fn tool_tips(mut self, t: Vec<String>) -> Self {
        if t.len() != self.buttons.len() {
            panic!("RadioButtons tool_tips must have an equal length as there are buttons. Got {:?} tool_tips but {:?} buttons", t, &self.buttons);
        }
        self.tool_tips = Some(t);
        self
    }
}

impl Component<WGPURenderer> for RadioButtons {
    fn view(&self) -> Option<Node<WGPURenderer>> {
        let mut base = node!(
            super::Div::new(),
            lay!(direction: match self.direction {
                Direction::Row => Direction::Column,
                Direction::Column => Direction::Row,
            })
        );

        let limit = match self.direction {
            Direction::Row => self.max_columns.unwrap_or(10000),
            Direction::Column => self.max_rows.unwrap_or(10000),
        };
        let len = self.buttons.len();
        let n_rows = match self.direction {
            Direction::Column => {
                if len > limit {
                    limit
                } else {
                    len
                }
            }
            Direction::Row => (len + limit - 1) / limit,
        };
        let n_columns = match self.direction {
            Direction::Column => (len + limit - 1) / limit,
            Direction::Row => {
                if len > limit {
                    limit
                } else {
                    len
                }
            }
        };

        let mut i: usize = 0;
        let mut j: usize = 0;
        let mut container = node!(super::Div::new(), lay!(direction: self.direction), i as u64);
        for (position, b) in self.buttons.iter().enumerate() {
            if j >= limit {
                j = 0;
                i += 1;
                let old_container = container;
                container = node!(
                    super::Div::new(),
                    lay!(direction: self.direction,
                         cross_alignment: Alignment::Stretch,
                         // axis_alignment: Alignment::Stretch, // TODO: This is broken
                    ),
                    i as u64
                );
                base = base.push(old_container);
            }
            let row = match self.direction {
                Direction::Row => i,
                Direction::Column => j,
            };
            let col = match self.direction {
                Direction::Row => j,
                Direction::Column => i,
            };

            let selected = self.selected.contains(&position);
            container = container.push(node!(
                RadioButton {
                    label: b.clone(),
                    tool_tip: self.tool_tips.as_ref().map(|tt| tt[position].clone()),
                    position,
                    selected,
                    style: self.style.clone(),
                    radius: (
                        if row == 0 && col == 0 {
                            self.style.radius
                        } else {
                            0.0
                        },
                        if row == 0 && (col + 1 == n_columns || position + 1 == len) {
                            self.style.radius
                        } else {
                            0.0
                        },
                        if position + 1 == len {
                            self.style.radius
                        } else {
                            0.0
                        },
                        if col == 0 && (row + 1 == n_rows || position + 1 == len) {
                            self.style.radius
                        } else {
                            0.0
                        },
                    ),
                    state: Some(RadioButtonState {
                        selected,
                        ..Default::default()
                    }),
                },
                lay!(),
                j as u64
            ));

            j += 1;
        }

        Some(base.push(container))
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        let mut m: Vec<Message> = vec![];

        match message.downcast_ref::<RadioButtonMsg>() {
            Some(RadioButtonMsg::Clicked(n)) => {
                if let Some(change_fn) = &self.on_change {
                    if self.selected.contains(n) {
                        if self.nullable {
                            m.push(change_fn(
                                self.selected.iter().cloned().filter(|x| x != n).collect(),
                            ));
                        }
                    } else if self.multi_select {
                        let mut selected = vec![*n];
                        selected.extend(self.selected.iter());
                        m.push(change_fn(selected));
                    } else {
                        m.push(change_fn(vec![*n]));
                    }
                }
            }
            None => panic!(),
        }
        m
    }
}

#[derive(Debug, Default)]
struct RadioButtonState {
    hover: bool,
    selected: bool,
    tool_tip_open: Option<Point>,
    hover_start: Option<Instant>,
}

#[state_component(RadioButtonState)]
#[derive(Debug)]
struct RadioButton {
    label: Vec<TextSegment>,
    tool_tip: Option<String>,
    position: usize,
    selected: bool,
    style: ButtonStyle,
    radius: (f32, f32, f32, f32),
}

#[state_component_impl(RadioButtonState)]
impl Component<WGPURenderer> for RadioButton {
    fn props_hash(&self, hasher: &mut ComponentHasher) {
        self.selected.hash(hasher);
    }

    fn new_props(&mut self) {
        self.state_mut().selected = self.selected;
    }

    fn view(&self) -> Option<Node<WGPURenderer>> {
        let mut base = node!(
            super::RoundedRect {
                background_color: if self.state_ref().selected {
                    self.style.active_color
                } else if self.state_ref().hover {
                    self.style.highlight_color
                } else {
                    self.style.background_color
                },
                border_color: self.style.border_color,
                border_width: self.style.border_width,
                radius: self.radius,
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
        event.dirty();
        vec![]
    }

    fn on_mouse_leave(&mut self, event: &mut event::Event<event::MouseLeave>) -> Vec<Message> {
        self.state = Some(RadioButtonState::default());
        event.dirty();
        vec![]
    }

    fn on_tick(&mut self, event: &mut event::Event<event::Tick>) -> Vec<Message> {
        if self.state_ref().hover_start.is_some()
            && self
                .state_ref()
                .hover_start
                .map(|s| s.elapsed().as_millis() > super::ToolTip::DELAY)
                .unwrap_or(false)
        {
            self.state_mut().tool_tip_open = Some(event.relative_logical_position());
            event.dirty();
        }
        vec![]
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) -> Vec<Message> {
        self.state_mut().selected = true;
        event.dirty();
        event.stop_bubbling();
        vec![msg!(RadioButtonMsg::Clicked(self.position))]
    }
}
