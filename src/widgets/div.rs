use std::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, Message, RenderContext};
use crate::event;
use crate::layout::*;
use crate::render::wgpu::{Rect, WGPURenderable, WGPURenderer};
use lemna_macros::{state_component, state_component_impl};

const MIN_BAR_SIZE: f32 = 10.0;

#[derive(Debug, Default)]
pub struct DivState {
    scroll_position: Point,
    x_scroll_bar: Option<AABB>,
    y_scroll_bar: Option<AABB>,
    over_y_bar: bool,
    y_bar_pressed: bool,
    over_x_bar: bool,
    x_bar_pressed: bool,
    drag_start_position: Point,
    scaled_scroll_bar_width: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerticalPosition {
    Bottom,
    Top,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HorizontalPosition {
    Left,
    Right,
}

impl Default for VerticalPosition {
    fn default() -> Self {
        Self::Bottom
    }
}

#[derive(Debug, Clone)]
pub struct ScrollDescriptor {
    pub scroll_x: bool,
    pub scroll_y: bool,
    pub x_bar_position: VerticalPosition,
    pub y_bar_position: HorizontalPosition,
    pub bar_width: f32,
    pub bar_background_color: Color,
    pub bar_color: Color,
    pub bar_highlight_color: Color,
    pub bar_active_color: Color,
}

impl Default for ScrollDescriptor {
    fn default() -> Self {
        Self {
            scroll_x: false,
            scroll_y: false,
            x_bar_position: VerticalPosition::Bottom,
            y_bar_position: HorizontalPosition::Right,
            bar_width: 12.0,
            bar_background_color: 0.9.into(),
            bar_color: 0.7.into(),
            bar_highlight_color: 0.5.into(),
            bar_active_color: 0.3.into(),
        }
    }
}

#[state_component(DivState)]
#[derive(Debug, Default)]
pub struct Div {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: Option<f32>,
    pub scroll: Option<ScrollDescriptor>,
}

impl Div {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bg<C: Into<Color>>(mut self, bg: C) -> Self {
        self.background = Some(bg.into());
        self
    }

    pub fn border<C: Into<Color>>(mut self, color: C, width: f32) -> Self {
        self.border_color = Some(color.into());
        self.border_width = Some(width);
        self
    }

    pub fn scroll(mut self, scroll: ScrollDescriptor) -> Self {
        if scroll.scroll_x || scroll.scroll_y {
            self.scroll = Some(scroll);
            self.state = Some(DivState::default());
        }
        self
    }
}

#[state_component_impl(DivState)]
impl Component<WGPURenderer> for Div {
    fn render_hash(&self, hasher: &mut ComponentHasher) {
        if self.state.is_some() {
            self.state_ref().scroll_position.hash(hasher);
            self.state_ref().over_y_bar.hash(hasher);
            self.state_ref().over_x_bar.hash(hasher);
            self.state_ref().y_bar_pressed.hash(hasher);
            self.state_ref().x_bar_pressed.hash(hasher);
        }
        if let Some(color) = self.background {
            color.hash(hasher);
        }
        // Maybe TODO: Should hash scroll_descriptor
    }

    fn on_scroll(&mut self, event: &mut event::Event<event::Scroll>) -> Vec<Message> {
        if let Some(scroll) = &self.scroll {
            let mut scroll_position = self.state_ref().scroll_position;
            let mut scrolled = false;
            let size = event.current_physical_aabb().size();
            let inner_scale = event.current_inner_scale.unwrap();

            if scroll.scroll_y {
                if event.input.y > 0.0 {
                    let max_position = inner_scale.height - size.height;
                    if scroll_position.y < max_position {
                        scroll_position.y += event.input.y;
                        scroll_position.y = scroll_position.y.min(max_position);
                        scrolled = true;
                    }
                } else if event.input.y < 0.0 && scroll_position.y > 0.0 {
                    if scroll_position.y + size.height > inner_scale.height {
                        scroll_position.y = inner_scale.height - size.height;
                    }
                    scroll_position.y += event.input.y;
                    scroll_position.y = scroll_position.y.max(0.0);
                    scrolled = true;
                }
            }

            if scroll.scroll_x {
                if event.input.x > 0.0 {
                    let max_position = inner_scale.width - size.width;
                    if scroll_position.x < max_position {
                        scroll_position.x += event.input.x;
                        scroll_position.x = scroll_position.x.min(max_position);
                        scrolled = true;
                    }
                } else if event.input.x < 0.0 && scroll_position.x > 0.0 {
                    if scroll_position.x + size.width > inner_scale.width {
                        scroll_position.x = inner_scale.width - size.width;
                    }
                    scroll_position.x += event.input.x;
                    scroll_position.x = scroll_position.x.max(0.0);
                    scrolled = true;
                }
            }

            if scrolled {
                self.state_mut().scroll_position = scroll_position;
                event.stop_bubbling();
                event.dirty();
            }
        }
        vec![]
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) -> Vec<Message> {
        if self.scroll.is_some() {
            let over_y_bar = self
                .state_ref()
                .y_scroll_bar
                .map(|b| b.is_under(event.relative_physical_position()))
                .unwrap_or(false);
            let over_x_bar = self
                .state_ref()
                .x_scroll_bar
                .map(|b| b.is_under(event.relative_physical_position()))
                .unwrap_or(false);

            if self.state_ref().over_y_bar != over_y_bar
                || self.state_ref().over_x_bar != over_x_bar
            {
                self.state_mut().over_y_bar = over_y_bar;
                self.state_mut().over_x_bar = over_x_bar;
                event.dirty();
            }
            event.stop_bubbling();
        }
        vec![]
    }

    fn on_mouse_leave(&mut self, event: &mut event::Event<event::MouseLeave>) -> Vec<Message> {
        if self.scroll.is_some() {
            self.state_mut().over_y_bar = false;
            self.state_mut().over_x_bar = false;
            event.dirty();
        }
        vec![]
    }

    fn on_drag_start(&mut self, event: &mut event::Event<event::DragStart>) -> Vec<Message> {
        if self.scroll.is_some() {
            let x_bar_pressed = self.state_ref().over_x_bar;
            let y_bar_pressed = self.state_ref().over_y_bar;
            if x_bar_pressed || y_bar_pressed {
                let drag_start = self.state_ref().scroll_position;
                self.state_mut().x_bar_pressed = x_bar_pressed;
                self.state_mut().y_bar_pressed = y_bar_pressed;
                self.state_mut().drag_start_position = drag_start;
                event.dirty();
                event.stop_bubbling();
            }
        }
        vec![]
    }

    fn on_drag_end(&mut self, event: &mut event::Event<event::DragEnd>) -> Vec<Message> {
        if self.scroll.is_some() {
            self.state_mut().x_bar_pressed = false;
            self.state_mut().y_bar_pressed = false;
            event.dirty();
        }
        vec![]
    }

    fn on_drag(&mut self, event: &mut event::Event<event::Drag>) -> Vec<Message> {
        if self.scroll.is_some() {
            let start_position = self.state_ref().drag_start_position;
            let size = event.current_physical_aabb().size();
            let inner_scale = event.current_inner_scale.unwrap();
            let mut scroll_position = self.state_ref().scroll_position;

            if self.state_ref().y_bar_pressed {
                let drag = event.physical_delta().y;
                let delta_position = drag * (inner_scale.height / size.height);
                let max_position = inner_scale.height - size.height;
                scroll_position.y = (start_position.y + delta_position)
                    .round()
                    .min(max_position)
                    .max(0.0);
            }

            if self.state_ref().x_bar_pressed {
                let drag = event.physical_delta().x;
                let delta_position = drag * (inner_scale.width / size.width);
                let max_position = inner_scale.width - size.width;
                scroll_position.x = (start_position.x + delta_position)
                    .round()
                    .min(max_position)
                    .max(0.0);
            }

            self.state_mut().scroll_position = scroll_position;
            event.dirty();
        }
        vec![]
    }

    fn scroll_position(&self) -> Option<ScrollPosition> {
        if let Some(scroll) = &self.scroll {
            let p = self.state_ref().scroll_position;
            Some(ScrollPosition {
                x: if scroll.scroll_x { Some(p.x) } else { None },
                y: if scroll.scroll_y { Some(p.y) } else { None },
            })
        } else {
            None
        }
    }

    fn frame_bounds(&self, aabb: AABB, inner_scale: Option<Scale>) -> AABB {
        let mut aabb = aabb;
        if let Some(scroll) = &self.scroll {
            let inner_scale = inner_scale.unwrap();
            let scaled_width = self.state_ref().scaled_scroll_bar_width;
            let size = aabb.size();
            let max_position = inner_scale - size;

            if scroll.scroll_y && max_position.height > 0.0 {
                if scroll.y_bar_position == HorizontalPosition::Left {
                    aabb.pos.x += scaled_width;
                } else {
                    aabb.bottom_right.x -= scaled_width;
                }
            }

            if scroll.scroll_x && max_position.width > 0.0 {
                if scroll.x_bar_position == VerticalPosition::Top {
                    aabb.pos.y += scaled_width;
                } else {
                    aabb.bottom_right.y -= scaled_width;
                }
            }
        }

        aabb
    }

    fn render<'a>(
        &mut self,
        context: RenderContext<'a, WGPURenderer>,
    ) -> Option<Vec<WGPURenderable>> {
        let mut rs = vec![];
        let border_width = self
            .border_width
            .map_or(0.0, |x| (x * context.scale_factor.floor()).round());

        if let Some(bg) = self.background {
            rs.push(WGPURenderable::Rect(Rect::new(
                Pos {
                    x: border_width,
                    y: border_width,
                    z: 0.1,
                },
                context.aabb.size() - Scale::new(border_width * 2.0, border_width * 2.0),
                bg,
            )))
        }

        if let (Some(color), Some(_width)) = (self.border_color, self.border_width) {
            rs.push(WGPURenderable::Rect(Rect::new(
                Pos::default(),
                context.aabb.size(),
                color,
            )))
        }

        if self.scroll.is_some() {
            let scroll = self.scroll.as_ref().unwrap().clone();
            let scroll_position = self.state_ref().scroll_position;
            let inner_scale = context.inner_scale.unwrap();
            let size = context.aabb.size();
            let scaled_width = scroll.bar_width * context.scale_factor;
            self.state_mut().scaled_scroll_bar_width = scaled_width;

            let max_position = inner_scale - size;

            if scroll.scroll_y {
                if max_position.height > 0.0 {
                    let x = if scroll.y_bar_position == HorizontalPosition::Left {
                        0.0
                    } else {
                        size.width - scaled_width
                    };

                    let x_scroll_bar = scroll.scroll_x && max_position.width > 0.0;
                    let bar_background_height =
                        size.height - if x_scroll_bar { scaled_width } else { 0.0 };
                    let bar_y_offset =
                        if x_scroll_bar && scroll.x_bar_position == VerticalPosition::Top {
                            scaled_width
                        } else {
                            0.0
                        };

                    let bar_background = Rect::new(
                        Pos {
                            x,
                            y: bar_y_offset,
                            z: 0.1, // above background
                        },
                        Scale {
                            width: scaled_width,
                            height: bar_background_height,
                        },
                        scroll.bar_background_color,
                    );

                    let height = (bar_background_height * (size.height / inner_scale.height))
                        .max(MIN_BAR_SIZE);
                    let mut y = (bar_background_height - height)
                        * (scroll_position.y / max_position.height)
                        + bar_y_offset;
                    if height + y > bar_background_height {
                        y = bar_background_height - height;
                    }

                    let bar_aabb = AABB::new(
                        Pos {
                            x: x + 2.0,
                            y,
                            z: 0.2, // above bar background
                        },
                        Scale {
                            width: scaled_width - 4.0,
                            height,
                        },
                    );
                    let color = if self.state_ref().y_bar_pressed {
                        scroll.bar_active_color
                    } else if self.state_ref().over_y_bar {
                        scroll.bar_highlight_color
                    } else {
                        scroll.bar_color
                    };
                    let bar = Rect::new(bar_aabb.pos, bar_aabb.size(), color);
                    self.state_mut().y_scroll_bar = Some(bar_aabb);
                    rs.push(WGPURenderable::Rect(bar_background));
                    rs.push(WGPURenderable::Rect(bar));
                } else {
                    self.state_mut().y_scroll_bar = None;
                }
            }

            if scroll.scroll_x {
                if max_position.width > 0.0 {
                    let y = if scroll.x_bar_position == VerticalPosition::Top {
                        0.0
                    } else {
                        size.height - scaled_width
                    };

                    let y_scroll_bar = scroll.scroll_y && max_position.height > 0.0;
                    let bar_background_width =
                        size.width - if y_scroll_bar { scaled_width } else { 0.0 };
                    let bar_x_offset =
                        if y_scroll_bar && scroll.y_bar_position == HorizontalPosition::Left {
                            scaled_width
                        } else {
                            0.0
                        };

                    let bar_background = Rect::new(
                        Pos {
                            x: bar_x_offset,
                            y,
                            z: 0.1, // above background
                        },
                        Scale {
                            width: bar_background_width,
                            height: scaled_width,
                        },
                        scroll.bar_background_color,
                    );

                    let width =
                        (bar_background_width * (size.width / inner_scale.width)).max(MIN_BAR_SIZE);
                    let mut x = (bar_background_width - width)
                        * (scroll_position.x / max_position.width)
                        + bar_x_offset;
                    if width + x > bar_background_width {
                        x = bar_background_width - width;
                    }

                    let bar_aabb = AABB::new(
                        Pos {
                            x,
                            y: y + 2.0,
                            z: 0.2, // above bar background
                        },
                        Scale {
                            width,
                            height: scaled_width - 4.0,
                        },
                    );
                    let color = if self.state_ref().x_bar_pressed {
                        scroll.bar_active_color
                    } else if self.state_ref().over_x_bar {
                        scroll.bar_highlight_color
                    } else {
                        scroll.bar_color
                    };
                    let bar = Rect::new(bar_aabb.pos, bar_aabb.size(), color);
                    self.state_mut().x_scroll_bar = Some(bar_aabb);
                    rs.push(WGPURenderable::Rect(bar_background));
                    rs.push(WGPURenderable::Rect(bar));
                } else {
                    self.state_mut().x_scroll_bar = None;
                }
            }
        }

        Some(rs)
    }
}
