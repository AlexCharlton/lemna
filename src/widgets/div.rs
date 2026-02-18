extern crate alloc;

use alloc::{boxed::Box, vec, vec::Vec};
use core::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, RenderContext};
use crate::event;
use crate::layout::*;
use crate::renderable::{Rectangle, Renderable};
use crate::style::{HorizontalPosition, StyleVal, Styled, VerticalPosition};

use lemna_macros::{component, state_component_impl};

const MIN_BAR_SIZE: f32 = 10.0;

#[derive(Debug, Default)]
pub struct DivState {
    scroll_position: Point,
    x_scroll_bar: Option<Rect>,
    y_scroll_bar: Option<Rect>,
    over_y_bar: bool,
    y_bar_pressed: bool,
    over_x_bar: bool,
    x_bar_pressed: bool,
    drag_start_position: Point,
    scaled_scroll_bar_width: f32,
}

#[component(State = "DivState", Styled = "Scroll", Internal, NoView)]
#[derive(Debug, Default)]
pub struct Div {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_width: Option<f32>,
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

    pub fn scroll_x(mut self) -> Self {
        self = self.style("x", true);
        self.state = Some(DivState::default());
        self
    }

    pub fn scroll_y(mut self) -> Self {
        self = self.style("y", true);
        self.state = Some(DivState::default());
        self
    }

    fn x_scrollable(&self) -> bool {
        self.style_val("x").unwrap().into()
    }

    fn y_scrollable(&self) -> bool {
        self.style_val("y").unwrap().into()
    }

    fn scrollable(&self) -> bool {
        self.x_scrollable() || self.y_scrollable()
    }
}

#[state_component_impl(DivState, Internal)]
impl Component for Div {
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
        if let Some(width) = self.border_width {
            ((width * 10.0) as i32).hash(hasher);
        }
        if let Some(color) = self.border_color {
            color.hash(hasher);
        }
        // Maybe TODO: Should hash scroll_descriptor
    }

    fn on_scroll(&mut self, event: &mut event::Event<event::Scroll>) {
        if self.scrollable() {
            let mut scroll_position = self.state_ref().scroll_position;
            let mut scrolled = false;
            let size = event.current_physical_aabb().size();
            let inner_scale = event.current_physical_inner_scale().unwrap();

            if self.y_scrollable() {
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

            if self.x_scrollable() {
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
            }
        }
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) {
        if self.scrollable() {
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
            }
            event.stop_bubbling();
        }
    }

    fn on_mouse_leave(&mut self, _event: &mut event::Event<event::MouseLeave>) {
        if self.scrollable() {
            if self.state_ref().over_y_bar {
                self.state_mut().over_y_bar = false;
            }
            if self.state_ref().over_x_bar {
                self.state_mut().over_x_bar = false;
            }
        }
    }

    fn on_drag_start(&mut self, event: &mut event::Event<event::DragStart>) {
        if self.scrollable() {
            let x_bar_pressed = self.state_ref().over_x_bar;
            let y_bar_pressed = self.state_ref().over_y_bar;
            if x_bar_pressed || y_bar_pressed {
                let drag_start = self.state_ref().scroll_position;
                self.state_mut().x_bar_pressed = x_bar_pressed;
                self.state_mut().y_bar_pressed = y_bar_pressed;
                self.state_mut().drag_start_position = drag_start;
                event.stop_bubbling();
            }
        }
    }

    fn on_drag_end(&mut self, _event: &mut event::Event<event::DragEnd>) {
        if self.scrollable() {
            self.state_mut().x_bar_pressed = false;
            self.state_mut().y_bar_pressed = false;
        }
    }

    fn on_drag(&mut self, event: &mut event::Event<event::Drag>) {
        if self.scrollable() {
            let start_position = self.state_ref().drag_start_position;
            let size = event.current_physical_aabb().size();
            let inner_scale = event.current_physical_inner_scale().unwrap();
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
        }
    }

    fn scroll_position(&self) -> Option<ScrollPosition> {
        if self.scrollable() {
            let p = self.state_ref().scroll_position;
            Some(ScrollPosition {
                x: if self.x_scrollable() { Some(p.x) } else { None },
                y: if self.y_scrollable() { Some(p.y) } else { None },
            })
        } else {
            None
        }
    }

    fn on_scroll_to(&mut self, target_aabb: Rect, aabb: Rect, inner_scale: Option<Scale>) -> bool {
        let mut scrolled = false;
        // Calculate our own frame bounds
        let frame = self.frame_bounds(aabb, inner_scale);

        // Calculate how much to scroll to bring target into view
        // Only scroll if the target is outside the current frame

        if self.y_scrollable() {
            let target_height = target_aabb.size().height;
            let frame_height = frame.size().height;

            // Check if target is above the visible frame (same logic regardless of size)
            if target_aabb.pos.y < frame.pos.y {
                // Scroll up to show the top of the target
                self.state_mut().scroll_position.y += target_aabb.pos.y - frame.pos.y;
                scrolled = true;
            }
            // If target fits entirely in frame, try to show the whole target
            else if target_height <= frame_height
                && target_aabb.bottom_right.y > frame.bottom_right.y
            {
                // Scroll down to show the bottom of the target
                self.state_mut().scroll_position.y +=
                    target_aabb.bottom_right.y - frame.bottom_right.y;
                scrolled = true;
            } else if target_aabb.pos.y > frame.bottom_right.y {
                // Target is too large to fit, always prioritize showing the top
                // Check if target top is below the visible frame
                // Scroll down to show the top of the target
                self.state_mut().scroll_position.y += target_aabb.pos.y - frame.bottom_right.y;
                scrolled = true;
            }
        }

        if self.x_scrollable() {
            let target_width = target_aabb.size().width;
            let frame_width = frame.size().width;

            // Check if target is left of the visible frame (same logic regardless of size)
            if target_aabb.pos.x < frame.pos.x {
                // Scroll left to show the left of the target
                self.state_mut().scroll_position.x += target_aabb.pos.x - frame.pos.x;
                scrolled = true;
            }
            // If target fits entirely in frame, try to show the whole target
            else if target_width <= frame_width
                && target_aabb.bottom_right.x > frame.bottom_right.x
            {
                // Check if target is right of the visible frame
                // Scroll right to show the right of the target
                self.state_mut().scroll_position.x +=
                    target_aabb.bottom_right.x - frame.bottom_right.x;
                scrolled = true;
            } else if target_aabb.pos.x > frame.bottom_right.x {
                // Target is too large to fit, always prioritize showing the left
                // Check if target left is right of the visible frame
                // Scroll right to show the left of the target
                self.state_mut().scroll_position.x += target_aabb.pos.x - frame.bottom_right.x;
                scrolled = true;
            }
        }

        scrolled
    }

    fn frame_bounds(&self, rect: Rect, inner_scale: Option<Scale>) -> Rect {
        let mut rect = rect;
        if self.scrollable() {
            let inner_scale = inner_scale.unwrap();
            let scaled_width = self.state_ref().scaled_scroll_bar_width;
            let size = rect.size();
            let max_position = inner_scale - size;

            if self.y_scrollable() && max_position.height > 0.0 {
                if self.style_val("y_bar_position")
                    == Some(StyleVal::HorizontalPosition(HorizontalPosition::Left))
                {
                    rect.pos.x += scaled_width;
                } else {
                    rect.bottom_right.x -= scaled_width;
                }
            }

            if self.x_scrollable() && max_position.width > 0.0 {
                if self.style_val("x_bar_position")
                    == Some(StyleVal::VerticalPosition(VerticalPosition::Top))
                {
                    rect.pos.y += scaled_width;
                } else {
                    rect.bottom_right.y -= scaled_width;
                }
            }
        }

        rect
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let mut rs = vec![];
        let border_width = self
            .border_width
            .map_or(0.0, |x| (x * context.scale_factor.floor()).round())
            .max(0.0);

        if let Some(bg) = self.background {
            rs.push(Renderable::Rectangle(Rectangle::new(
                Pos {
                    x: border_width,
                    y: border_width,
                    z: 0.1,
                },
                context.aabb.size() - Scale::new(border_width * 2.0, border_width * 2.0),
                bg,
            )))
        }

        if let Some(color) = self.border_color
            && border_width > 0.0
        {
            rs.push(Renderable::Rectangle(Rectangle::new(
                Pos::default(),
                context.aabb.size(),
                color,
            )))
        }

        if self.scrollable() {
            let scroll_position = self.state_ref().scroll_position;
            let inner_scale = context.inner_scale.unwrap();
            let size = context.aabb.size();
            let scaled_width = self.style_val("bar_width").unwrap().f32() * context.scale_factor;
            self.state_mut().scaled_scroll_bar_width = scaled_width;

            let max_position = inner_scale - size;

            if self.y_scrollable() {
                if max_position.height > 0.0 {
                    let x = if self.style_val("y_bar_position")
                        == Some(StyleVal::HorizontalPosition(HorizontalPosition::Left))
                    {
                        0.0
                    } else {
                        size.width - scaled_width
                    };

                    let x_scroll_bar = self.x_scrollable() && max_position.width > 0.0;
                    let bar_background_height =
                        size.height - if x_scroll_bar { scaled_width } else { 0.0 };
                    let bar_y_offset = if x_scroll_bar
                        && self.style_val("x_bar_position")
                            == Some(StyleVal::VerticalPosition(VerticalPosition::Top))
                    {
                        scaled_width
                    } else {
                        0.0
                    };

                    let bar_background = Rectangle::new(
                        Pos {
                            x,
                            y: bar_y_offset,
                            z: 0.1, // above background
                        },
                        Scale {
                            width: scaled_width,
                            height: bar_background_height,
                        },
                        self.style_val("bar_background_color").into(),
                    );

                    let height = (bar_background_height * (size.height / inner_scale.height))
                        .max(MIN_BAR_SIZE);
                    let mut y = (bar_background_height - height)
                        * (scroll_position.y / max_position.height)
                        + bar_y_offset;
                    if height + y > bar_background_height {
                        y = bar_background_height - height;
                    }

                    let bar_rect = Rect::new(
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
                    let color: Color = if self.state_ref().y_bar_pressed {
                        self.style_val("bar_active_color").into()
                    } else if self.state_ref().over_y_bar {
                        self.style_val("bar_highlight_color").into()
                    } else {
                        self.style_val("bar_color").into()
                    };
                    let bar = Rectangle::new(bar_rect.pos, bar_rect.size(), color);
                    self.state_mut().y_scroll_bar = Some(bar_rect);
                    rs.push(Renderable::Rectangle(bar_background));
                    rs.push(Renderable::Rectangle(bar));
                } else {
                    self.state_mut().y_scroll_bar = None;
                }
            }

            if self.x_scrollable() {
                if max_position.width > 0.0 {
                    let y = if self.style_val("x_bar_position")
                        == Some(StyleVal::VerticalPosition(VerticalPosition::Top))
                    {
                        0.0
                    } else {
                        size.height - scaled_width
                    };

                    let y_scroll_bar = self.y_scrollable() && max_position.height > 0.0;
                    let bar_background_width =
                        size.width - if y_scroll_bar { scaled_width } else { 0.0 };
                    let bar_x_offset = if y_scroll_bar
                        && self.style_val("y_bar_position")
                            == Some(StyleVal::HorizontalPosition(HorizontalPosition::Left))
                    {
                        scaled_width
                    } else {
                        0.0
                    };

                    let bar_background = Rectangle::new(
                        Pos {
                            x: bar_x_offset,
                            y,
                            z: 0.1, // above background
                        },
                        Scale {
                            width: bar_background_width,
                            height: scaled_width,
                        },
                        self.style_val("bar_background_color").into(),
                    );

                    let width =
                        (bar_background_width * (size.width / inner_scale.width)).max(MIN_BAR_SIZE);
                    let mut x = (bar_background_width - width)
                        * (scroll_position.x / max_position.width)
                        + bar_x_offset;
                    if width + x > bar_background_width {
                        x = bar_background_width - width;
                    }

                    let bar_rect = Rect::new(
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
                        self.style_val("bar_active_color").into()
                    } else if self.state_ref().over_x_bar {
                        self.style_val("bar_highlight_color").into()
                    } else {
                        self.style_val("bar_color").into()
                    };
                    let bar = Rectangle::new(bar_rect.pos, bar_rect.size(), color);
                    self.state_mut().x_scroll_bar = Some(bar_rect);
                    rs.push(Renderable::Rectangle(bar_background));
                    rs.push(Renderable::Rectangle(bar));
                } else {
                    self.state_mut().x_scroll_bar = None;
                }
            }
        }

        Some(rs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_to_small_target_above_frame() {
        let mut div = Div::new().scroll_y();
        // Start with no scroll
        div.state_mut().scroll_position.y = 0.0;

        // Container aabb: 0,0 to 100,100, frame is also 0,0 to 100,100 (no scroll bars)
        // Target at y=-10 is above the frame
        let aabb = Rect::new(Pos::new(0.0, 0.0, 0.0), Scale::new(100.0, 100.0));
        let target_aabb = Rect::new(Pos::new(0.0, -10.0, 0.0), Scale::new(100.0, 20.0));
        let inner_scale = Some(Scale::new(100.0, 200.0));

        let scrolled = div.on_scroll_to(target_aabb, aabb, inner_scale);

        assert!(scrolled);
        // target.pos.y (-10) < frame.pos.y (0), so we scroll by -10 - 0 = -10
        // scroll_position becomes 0 + (-10) = -10
        assert_eq!(div.state_ref().scroll_position.y, -10.0);
    }

    #[test]
    fn test_scroll_to_small_target_below_frame() {
        let mut div = Div::new().scroll_y();
        div.state_mut().scroll_position.y = 0.0;

        // Frame: 0,0 to 100,100 (visible area)
        // Target: 0,150 to 100,170 (small target below visible frame)
        let aabb = Rect::new(Pos::new(0.0, 0.0, 0.0), Scale::new(100.0, 100.0));
        let target_aabb = Rect::new(Pos::new(0.0, 150.0, 0.0), Scale::new(100.0, 20.0));
        let inner_scale = Some(Scale::new(100.0, 200.0));

        let scrolled = div.on_scroll_to(target_aabb, aabb, inner_scale);

        assert!(scrolled);
        // Should scroll down to show bottom of target
        // Target bottom is at 170, frame bottom is at 100
        // So we need to scroll by 170 - 100 = 70
        assert_eq!(div.state_ref().scroll_position.y, 70.0);
    }

    #[test]
    fn test_scroll_to_large_target_above_frame() {
        let mut div = Div::new().scroll_y();
        div.state_mut().scroll_position.y = 50.0;

        // Container aabb: 0,0 to 100,100
        // Target at y=-10 is above the frame
        let aabb = Rect::new(Pos::new(0.0, 0.0, 0.0), Scale::new(100.0, 100.0));
        let target_aabb = Rect::new(Pos::new(0.0, -10.0, 0.0), Scale::new(100.0, 130.0));
        let inner_scale = Some(Scale::new(100.0, 200.0));

        let scrolled = div.on_scroll_to(target_aabb, aabb, inner_scale);

        assert!(scrolled);
        // target.pos.y (-10) < frame.pos.y (0), so we scroll by -10 - 0 = -10
        // scroll_position becomes 50 + (-10) = 40
        assert_eq!(div.state_ref().scroll_position.y, 40.0);
    }

    #[test]
    fn test_scroll_to_large_target_below_frame() {
        let mut div = Div::new().scroll_y();
        div.state_mut().scroll_position.y = 0.0;

        // Frame: 0,0 to 100,100 (visible area)
        // Target: 0,120 to 100,250 (large target, 130px tall, below visible frame)
        let aabb = Rect::new(Pos::new(0.0, 0.0, 0.0), Scale::new(100.0, 100.0));
        let target_aabb = Rect::new(Pos::new(0.0, 120.0, 0.0), Scale::new(100.0, 130.0));
        let inner_scale = Some(Scale::new(100.0, 250.0));

        let scrolled = div.on_scroll_to(target_aabb, aabb, inner_scale);

        assert!(scrolled);
        // Should scroll down to show top of target (not bottom, since it's too large)
        // Target top is at 120, frame bottom is at 100
        // So we scroll by 120 - 100 = 20
        assert_eq!(div.state_ref().scroll_position.y, 20.0);
    }

    #[test]
    fn test_scroll_to_target_already_visible() {
        let mut div = Div::new().scroll_y();
        div.state_mut().scroll_position.y = 50.0;

        // Frame: 50,0 to 150,100 (visible area at scroll position 50)
        // Target: 0,60 to 100,80 (small target already visible in frame)
        let aabb = Rect::new(Pos::new(0.0, 0.0, 0.0), Scale::new(100.0, 100.0));
        let target_aabb = Rect::new(Pos::new(0.0, 60.0, 0.0), Scale::new(100.0, 20.0));
        let inner_scale = Some(Scale::new(100.0, 200.0));

        let initial_scroll = div.state_ref().scroll_position.y;
        let scrolled = div.on_scroll_to(target_aabb, aabb, inner_scale);

        assert!(!scrolled);
        assert_eq!(div.state_ref().scroll_position.y, initial_scroll);
    }

    #[test]
    fn test_scroll_to_x_axis_small_target_left() {
        let mut div = Div::new().scroll_x();
        div.state_mut().scroll_position.x = 50.0;

        // Target at x=-10 is left of the frame
        let aabb = Rect::new(Pos::new(0.0, 0.0, 0.0), Scale::new(100.0, 100.0));
        let target_aabb = Rect::new(Pos::new(-10.0, 0.0, 0.0), Scale::new(20.0, 100.0));
        let inner_scale = Some(Scale::new(200.0, 100.0));

        let scrolled = div.on_scroll_to(target_aabb, aabb, inner_scale);

        assert!(scrolled);
        // target.pos.x (-10) < frame.pos.x (0), so we scroll by -10 - 0 = -10
        // scroll_position becomes 50 + (-10) = 40
        assert_eq!(div.state_ref().scroll_position.x, 40.0);
    }

    #[test]
    fn test_scroll_to_x_axis_small_target_right() {
        let mut div = Div::new().scroll_x();
        div.state_mut().scroll_position.x = 0.0;

        let aabb = Rect::new(Pos::new(0.0, 0.0, 0.0), Scale::new(100.0, 100.0));
        let target_aabb = Rect::new(Pos::new(150.0, 0.0, 0.0), Scale::new(20.0, 100.0));
        let inner_scale = Some(Scale::new(200.0, 100.0));

        let scrolled = div.on_scroll_to(target_aabb, aabb, inner_scale);

        assert!(scrolled);
        // Should scroll to show right edge: 170 - 100 = 70
        assert_eq!(div.state_ref().scroll_position.x, 70.0);
    }

    #[test]
    fn test_scroll_to_x_axis_large_target_right() {
        let mut div = Div::new().scroll_x();
        div.state_mut().scroll_position.x = 0.0;

        let aabb = Rect::new(Pos::new(0.0, 0.0, 0.0), Scale::new(100.0, 100.0));
        let target_aabb = Rect::new(Pos::new(120.0, 0.0, 0.0), Scale::new(130.0, 100.0));
        let inner_scale = Some(Scale::new(250.0, 100.0));

        let scrolled = div.on_scroll_to(target_aabb, aabb, inner_scale);

        assert!(scrolled);
        // Should scroll to show left edge (prioritize left for large targets)
        // Target left is at 120, frame right is at 100
        // So we scroll by 120 - 100 = 20
        assert_eq!(div.state_ref().scroll_position.x, 20.0);
    }

    #[test]
    fn test_scroll_to_both_axes() {
        let mut div = Div::new().scroll_x().scroll_y();
        div.state_mut().scroll_position = Point::new(50.0, 50.0);

        // Target at (-10, -10) is above and left of the frame
        let aabb = Rect::new(Pos::new(0.0, 0.0, 0.0), Scale::new(100.0, 100.0));
        let target_aabb = Rect::new(Pos::new(-10.0, -10.0, 0.0), Scale::new(20.0, 20.0));
        let inner_scale = Some(Scale::new(200.0, 200.0));

        let scrolled = div.on_scroll_to(target_aabb, aabb, inner_scale);

        assert!(scrolled);
        // Should scroll both axes
        // x: 50 + (-10 - 0) = 40
        // y: 50 + (-10 - 0) = 40
        assert_eq!(div.state_ref().scroll_position.x, 40.0);
        assert_eq!(div.state_ref().scroll_position.y, 40.0);
    }

    #[test]
    fn test_scroll_to_no_scroll_enabled() {
        let mut div = Div::new(); // No scrolling enabled
        // When scroll is not enabled, state is None, so we can't access it
        // But on_scroll_to should just return false without panicking

        let aabb = Rect::new(Pos::new(0.0, 0.0, 0.0), Scale::new(100.0, 100.0));
        let target_aabb = Rect::new(Pos::new(0.0, 20.0, 0.0), Scale::new(100.0, 20.0));
        let inner_scale = Some(Scale::new(100.0, 200.0));

        let scrolled = div.on_scroll_to(target_aabb, aabb, inner_scale);

        assert!(!scrolled);
    }
}
