use std::fmt;
use std::hash::Hash;
use std::time::Instant;

use super::{TextSegment, ToolTip};
use crate::base_types::*;
use crate::component::{Component, ComponentHasher, Message};
use crate::event;
use crate::font_cache::HorizontalAlign;
use crate::layout::*;
use crate::style::Styled;
use crate::{node, Node};
use lemna_macros::{component, state_component_impl};

#[component(Styled = "RadioButton", Internal)]
pub struct RadioButtons {
    buttons: Vec<Vec<TextSegment>>,
    tool_tips: Option<Vec<String>>,
    selected: Vec<usize>,
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
            .field("selected", &self.selected)
            .finish()
    }
}

enum RadioButtonMsg {
    Clicked(usize),
}

impl RadioButtons {
    pub fn new(buttons: Vec<Vec<TextSegment>>, selected: Vec<usize>) -> Self {
        Self {
            buttons,
            tool_tips: None,
            selected,
            direction: Direction::Row,
            max_rows: None,
            max_columns: None,
            on_change: None,
            multi_select: false,
            nullable: false,
            class: Default::default(),
            style_overrides: Default::default(),
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

impl Component for RadioButtons {
    fn view(&self) -> Option<Node> {
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
        let mut container = node!(super::Div::new(), lay!(direction: self.direction)).key(i as u64);
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
                    )
                )
                .key(i as u64);
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
            let radius: f32 = self.style_val("radius").unwrap().f32();
            container = container.push(
                node!(RadioButton {
                    label: b.clone(),
                    tool_tip: self.tool_tips.as_ref().map(|tt| tt[position].clone()),
                    position,
                    selected,
                    radius: (
                        if row == 0 && col == 0 { radius } else { 0.0 },
                        if row == 0 && (col + 1 == n_columns || position + 1 == len) {
                            radius
                        } else {
                            0.0
                        },
                        if position + 1 == len { radius } else { 0.0 },
                        if col == 0 && (row + 1 == n_rows || position + 1 == len) {
                            radius
                        } else {
                            0.0
                        },
                    ),
                    state: Some(Default::default()),
                    dirty: true,
                    class: self.class.clone(),
                    style_overrides: self.style_overrides.clone(),
                })
                .key(j as u64),
            );

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
    tool_tip_open: Option<Point>,
    hover_start: Option<Instant>,
}

#[component(State = "RadioButtonState", Styled, Internal)]
#[derive(Debug)]
struct RadioButton {
    label: Vec<TextSegment>,
    tool_tip: Option<String>,
    position: usize,
    selected: bool,
    radius: (f32, f32, f32, f32),
}

#[state_component_impl(RadioButtonState)]
impl Component for RadioButton {
    fn props_hash(&self, hasher: &mut ComponentHasher) {
        self.selected.hash(hasher);
    }

    fn view(&self) -> Option<Node> {
        let padding: f64 = self.style_val("padding").unwrap().into();
        let active_color: Color = self.style_val("active_color").into();
        let highlight_color: Color = self.style_val("highlight_color").into();
        let background_color: Color = self.style_val("background_color").into();
        let border_color: Color = self.style_val("border_color").into();
        let border_width: f32 = self.style_val("border_width").unwrap().f32();

        let mut base = node!(
            super::RoundedRect {
                background_color: if self.selected {
                    active_color
                } else if self.state_ref().hover {
                    highlight_color
                } else {
                    background_color
                },
                border_color: border_color,
                border_width: border_width,
                radius: self.radius,
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
            .style("h_alignment", HorizontalAlign::Center)
            .maybe_style("font", self.style_val("font"))));

        if let (Some(p), Some(tt)) = (self.state_ref().tool_tip_open, self.tool_tip.as_ref()) {
            base = base.push(node!(
                ToolTip::new(tt.clone()),
                lay!(position_type: PositionType::Absolute,
                     z_index_increment: 1000.0,
                     position: (p + ToolTip::MOUSE_OFFSET).into(),
                )
            ));
        }

        Some(base)
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) {
        self.state_mut().hover_start = Some(Instant::now());
        event.stop_bubbling();
    }

    fn on_mouse_enter(&mut self, _event: &mut event::Event<event::MouseEnter>) {
        self.state_mut().hover = true;
    }

    fn on_mouse_leave(&mut self, _event: &mut event::Event<event::MouseLeave>) {
        *self.state_mut() = RadioButtonState::default();
    }

    fn on_tick(&mut self, event: &mut event::Event<event::Tick>) {
        if self.state_ref().hover_start.is_some()
            && self
                .state_ref()
                .hover_start
                .map(|s| s.elapsed().as_millis() > super::ToolTip::DELAY)
                .unwrap_or(false)
        {
            self.state_mut().tool_tip_open = Some(event.relative_logical_position());
        }
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) {
        event.stop_bubbling();
        event.emit(msg!(RadioButtonMsg::Clicked(self.position)));
    }
}
