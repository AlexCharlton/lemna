extern crate alloc;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::cmp::Ordering;
use core::hash::Hash;

use crate::TextSegment;
use crate::base_types::*;
use crate::component::{Component, ComponentHasher, Message, RenderContext};
use crate::event;
use crate::input::Key;
use crate::layout::{Layout, ScrollPosition};
use crate::renderable::{Caches, Rectangle, Renderable};
use crate::style::{HorizontalPosition, StyleVal, Styled};
use crate::time::Instant;
use crate::{Dirty, Node};
use lemna_macros::{component, state_component_impl};

use super::textbox::{TextBoxAction, TextBoxMessage};

const CURSOR_BLINK_PERIOD: i64 = 500;
const MIN_BAR_SIZE: f32 = 10.0;

// MARK: TextArea
#[derive(Debug, Default)]
struct TextAreaState {
    focused: bool,
}

#[component(State = "TextAreaState", Styled, Internal)]
pub struct TextArea {
    text: Option<String>,
    on_change: Option<Box<dyn Fn(String) -> Message + Send + Sync>>,
    on_commit: Option<Box<dyn Fn(String) -> Message + Send + Sync>>,
    on_focus: Option<Box<dyn Fn() -> Message + Send + Sync>>,
    commit_on_blur: bool,
    limit: Option<usize>,
}

impl core::fmt::Debug for TextArea {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_struct("TextArea")
            .field("text", &self.text)
            .finish()
    }
}

impl TextArea {
    pub fn new(default: Option<String>) -> Self {
        Self {
            text: default,
            on_change: None,
            on_commit: None,
            on_focus: None,
            commit_on_blur: false,
            limit: None,
            state: Some(TextAreaState::default()),
            dirty: Dirty::No,
            class: Default::default(),
            style_overrides: Default::default(),
        }
    }

    pub fn on_change(mut self, change_fn: Box<dyn Fn(String) -> Message + Send + Sync>) -> Self {
        self.on_change = Some(change_fn);
        self
    }

    pub fn on_commit(mut self, commit_fn: Box<dyn Fn(String) -> Message + Send + Sync>) -> Self {
        self.on_commit = Some(commit_fn);
        self
    }

    pub fn on_focus(mut self, focus_fn: Box<dyn Fn() -> Message + Send + Sync>) -> Self {
        self.on_focus = Some(focus_fn);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn commit_on_blur(mut self) -> Self {
        self.commit_on_blur = true;
        self
    }
}

#[state_component_impl(TextAreaState, Internal)]
impl Component for TextArea {
    fn layout(&self) -> Option<Layout> {
        Some(lay!(axis_alignment: Stretch, cross_alignment: Stretch))
    }

    fn view(&self) -> Option<Node> {
        let background: Color = self.style_val("background_color").into();
        let border: Color = self.style_val("border_color").into();
        let border_width = self.style_val("border_width").unwrap().f32();
        let mut container = TextAreaContainer::new(
            background,
            border,
            border_width * if self.state_ref().focused { 2.0 } else { 1.0 },
        );
        for parameter in [
            "bar_width",
            "bar_background_color",
            "bar_color",
            "bar_highlight_color",
            "bar_active_color",
            "y_bar_position",
        ] {
            container = container.style(parameter, self.style_val(parameter).unwrap());
        }

        Some(node!(container,).push(node!(
            TextAreaText {
                default_text: self.text.clone().unwrap_or_default(),
                limit: self.limit,
                commit_on_blur: self.commit_on_blur,
                style_overrides: self.style_overrides.clone(),
                class: self.class,
                state: None,
                dirty: Dirty::No,
            },
            [size_pct: [100.0, Auto]]
        )))
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        match message.downcast::<TextBoxMessage>() {
            Ok(message) => match *message {
                TextBoxMessage::Open => {
                    self.state_mut().focused = true;
                    self.on_focus.as_ref().map_or_else(Vec::new, |f| vec![f()])
                }
                TextBoxMessage::Close => {
                    self.state_mut().focused = false;
                    vec![]
                }
                TextBoxMessage::Change(value) => self
                    .on_change
                    .as_ref()
                    .map_or_else(Vec::new, |f| vec![f(value)]),
                TextBoxMessage::Commit(value) => self
                    .on_commit
                    .as_ref()
                    .map_or_else(Vec::new, |f| vec![f(value)]),
            },
            Err(message) => vec![message],
        }
    }

    fn on_focus(&mut self, event: &mut event::Event<event::Focus>) {
        event.focus_child(vec![0, 0]);
    }
}

// MARK: TextAreaContainer
enum TextAreaContainerMessage {
    SetFocusedYRange((f32, f32)),
}

#[derive(Debug, Default)]
struct TextAreaContainerState {
    scroll_position: f32,
    border_width_px: f32,
    y_scroll_bar: Option<Rect>,
    over_y_bar: bool,
    y_bar_pressed: bool,
    drag_start_position: f32,
    scaled_scroll_bar_width: f32,
    // Set by the TextAreaText component
    // Removed when scrolling manually
    focused_y_range: Option<(f32, f32)>,
}

#[component(State = "TextAreaContainerState", Styled = "Scroll", Internal, NoView)]
#[derive(Debug)]
struct TextAreaContainer {
    background_color: Color,
    border_color: Color,
    border_width: f32,
}

impl TextAreaContainer {
    fn new(background_color: Color, border_color: Color, border_width: f32) -> Self {
        Self {
            background_color,
            border_color,
            border_width,
            state: Some(Default::default()),
            dirty: Dirty::No,
            class: Default::default(),
            style_overrides: Default::default(),
        }
    }

    fn border_width_px(&self, scale_factor: f32) -> f32 {
        (self.border_width * scale_factor.floor()).round()
    }

    fn max_scroll(&self, event: &event::Event<event::Scroll>) -> f32 {
        let size = event.current_physical_aabb().size();
        let inner_height = event.current_physical_inner_scale().unwrap().height;
        let visible_height = (size.height - self.state_ref().border_width_px * 2.0).max(0.0);
        (inner_height - visible_height).max(0.0)
    }
}

#[state_component_impl(TextAreaContainerState, Internal)]
impl Component for TextAreaContainer {
    fn full_control(&self) -> bool {
        true
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        match message.downcast::<TextAreaContainerMessage>() {
            Ok(message) => match *message {
                TextAreaContainerMessage::SetFocusedYRange(range) => {
                    self.state_mut().focused_y_range = Some(range);
                    vec![]
                }
            },
            Err(m) => vec![m],
        }
    }

    fn set_aabb(
        &mut self,
        aabb: &mut Rect,
        _parent_aabb: Rect,
        _children: Vec<(&mut Rect, Option<Scale>, Option<Point>)>,
        _frame: Rect,
        scale_factor: f32,
    ) {
        if let Some((y_start, y_end)) = self.state_ref().focused_y_range {
            let border = self.border_width_px(scale_factor);

            let visible_height = aabb.height() - border * 2.0;
            let scroll = self.state_ref().scroll_position;
            if y_end > visible_height + scroll {
                self.state_mut().scroll_position = y_end - visible_height;
            } else if y_start < scroll {
                self.state_mut().scroll_position = y_start - border;
            }
        }
    }

    fn frame_bounds(&self, mut aabb: Rect, _inner_scale: Option<Scale>) -> Rect {
        let border = self.state_ref().border_width_px;
        aabb.pos.x += border;
        aabb.pos.y += border;
        aabb.bottom_right.x -= border;
        aabb.bottom_right.y -= border;
        if self.state_ref().y_scroll_bar.is_some() {
            if self.style_val("y_bar_position")
                == Some(StyleVal::HorizontalPosition(HorizontalPosition::Left))
            {
                aabb.pos.x += self.state_ref().scaled_scroll_bar_width;
            } else {
                aabb.bottom_right.x -= self.state_ref().scaled_scroll_bar_width;
            }
        }
        aabb
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.background_color.hash(hasher);
        self.border_color.hash(hasher);
        self.border_width.to_bits().hash(hasher);
        self.state_ref().scroll_position.to_bits().hash(hasher);
        self.state_ref().over_y_bar.hash(hasher);
        self.state_ref().y_bar_pressed.hash(hasher);
    }

    fn scroll_position(&self) -> Option<ScrollPosition> {
        Some(ScrollPosition {
            x: None,
            y: Some(self.state_ref().scroll_position),
        })
    }

    fn on_scroll(&mut self, event: &mut event::Event<event::Scroll>) {
        let old = self.state_ref().scroll_position;
        let max = self.max_scroll(event);
        self.state_mut().scroll_position = (old + event.input.y).max(0.0).min(max);
        self.state_mut().focused_y_range = None;
        if self.state_ref().scroll_position != old {
            event.stop_bubbling();
        }
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) {
        let over = self
            .state_ref()
            .y_scroll_bar
            .map(|bar| bar.is_under(event.relative_physical_position()))
            .unwrap_or(false);
        self.state_mut().over_y_bar = over;
        event.stop_bubbling();
    }

    fn on_mouse_leave(&mut self, _event: &mut event::Event<event::MouseLeave>) {
        self.state_mut().over_y_bar = false;
    }

    fn on_drag_start(&mut self, event: &mut event::Event<event::DragStart>) {
        if self.state_ref().over_y_bar {
            self.state_mut().y_bar_pressed = true;
            self.state_mut().drag_start_position = self.state_ref().scroll_position;
            self.state_mut().focused_y_range = None;
            event.stop_bubbling();
        }
    }

    fn on_drag_end(&mut self, _event: &mut event::Event<event::DragEnd>) {
        self.state_mut().y_bar_pressed = false;
    }

    fn on_drag(&mut self, event: &mut event::Event<event::Drag>) {
        if self.state_ref().y_bar_pressed {
            let size = event.current_physical_aabb().size();
            let inner = event.current_physical_inner_scale().unwrap().height;
            let visible_height = (size.height - self.state_ref().border_width_px * 2.0).max(0.0);
            let max = (inner - visible_height).max(0.0);
            let drag = event.physical_delta().y * inner / visible_height;
            self.state_mut().scroll_position = (self.state_ref().drag_start_position + drag)
                .min(max)
                .max(0.0);
        }
    }

    fn on_scroll_to(&mut self, target: Rect, aabb: Rect, inner: Option<Scale>) -> bool {
        let inner = inner.unwrap();
        let frame = self.frame_bounds(aabb, Some(inner));
        let old = self.state_ref().scroll_position;
        if target.pos.y < frame.pos.y {
            self.state_mut().scroll_position += target.pos.y - frame.pos.y;
        } else if target.size().height <= frame.size().height
            && target.bottom_right.y > frame.bottom_right.y
        {
            self.state_mut().scroll_position += target.bottom_right.y - frame.bottom_right.y;
        } else if target.pos.y > frame.bottom_right.y {
            self.state_mut().scroll_position += target.pos.y - frame.bottom_right.y;
        }
        let visible_height = (aabb.height() - self.state_ref().border_width_px * 2.0).max(0.0);
        let max = (inner.height - visible_height).max(0.0);
        self.state_mut().scroll_position = self.state_ref().scroll_position.min(max).max(0.0);
        self.state_ref().scroll_position != old
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let border = self.border_width_px(context.scale_factor);
        self.state_mut().border_width_px = border;
        let size = context.aabb.size();
        let mut output = vec![
            Renderable::Rectangle(Rectangle::new(
                Pos::new(border, border, 0.1),
                size - Scale::new(border * 2.0, border * 2.0),
                self.background_color,
            )),
            Renderable::Rectangle(Rectangle::new(Pos::default(), size, self.border_color)),
        ];

        let inner = context.inner_scale.unwrap_or_default();
        let bar_width = self.style_val("bar_width").unwrap().f32() * context.scale_factor;
        self.state_mut().scaled_scroll_bar_width = bar_width;
        let bar_height = (size.height - border * 2.0).max(0.0);
        let max = (inner.height - size.height).max(0.0);
        if max > 0.0 {
            let bar_x = if self.style_val("y_bar_position")
                == Some(StyleVal::HorizontalPosition(HorizontalPosition::Left))
            {
                border
            } else {
                size.width - bar_width - border
            };
            let background = Rectangle::new(
                Pos::new(bar_x, border, 0.2),
                Scale::new(bar_width, bar_height),
                self.style_val("bar_background_color").into(),
            );
            let height = (bar_height * bar_height / inner.height)
                .max(MIN_BAR_SIZE)
                .min(bar_height);
            let y = border + (bar_height - height) * (self.state_ref().scroll_position / max);
            let bar = Rect::new(
                Pos::new(bar_x + 2.0, y.min(border + bar_height - height), 0.3),
                Scale::new(bar_width - 4.0, height),
            );
            let color: Color = if self.state_ref().y_bar_pressed {
                self.style_val("bar_active_color").into()
            } else if self.state_ref().over_y_bar {
                self.style_val("bar_highlight_color").into()
            } else {
                self.style_val("bar_color").into()
            };
            self.state_mut().y_scroll_bar = Some(bar);
            output.push(Renderable::Rectangle(background));
            output.push(Renderable::Rectangle(Rectangle::new(
                bar.pos,
                bar.size(),
                color,
            )));
        } else {
            self.state_mut().y_scroll_bar = None;
        }
        Some(output)
    }
}

// MARK: TextAreaText
#[derive(Debug)]
struct TextAreaTextState {
    focused: bool,
    text: String,
    cursor_pos: usize,
    selection_from: Option<usize>,
    dragging: bool,
    activated_at: Instant,
    cursor_visible: bool,
    glyphs: Vec<crate::font_cache::PositionedGlyph>,
    lines: Vec<crate::font_cache::GlyphLines>,
    padding_offset_px: f32,
    line_height: f32,
    layout_width: Option<f32>,
    dirty: bool,
}

#[component(State = "TextAreaTextState", Styled = "TextArea", Internal)]
#[derive(Debug)]
struct TextAreaText {
    default_text: String,
    limit: Option<usize>,
    commit_on_blur: bool,
}

impl TextAreaText {
    fn reset_state(&mut self) {
        let mut text = self.default_text.clone();
        if let Some(limit) = self.limit {
            let end = text
                .char_indices()
                .take_while(|(i, _)| *i < limit)
                .last()
                .map_or(0, |(i, c)| i + c.len_utf8())
                .min(limit);
            text.truncate(end);
        }
        let focused = self.state.as_ref().is_some_and(|s| s.focused);
        let cursor_pos = self
            .state
            .as_ref()
            .map_or(0, |s| s.cursor_pos.min(text.len()));
        self.state = Some(TextAreaTextState {
            focused,
            text,
            cursor_pos,
            selection_from: None,
            dragging: false,
            activated_at: Instant::now(),
            cursor_visible: false,
            glyphs: vec![],
            lines: vec![],
            padding_offset_px: 0.0,
            line_height: 0.0,
            layout_width: None,
            dirty: true,
        });
    }

    fn activate(&mut self) {
        self.state_mut().activated_at = Instant::now();
        self.state_mut().cursor_visible = true;
        self.state_mut().selection_from = None;
    }

    fn selection(&self) -> Option<(usize, usize)> {
        let pos = self.state_ref().cursor_pos;
        self.state_ref()
            .selection_from
            .and_then(|from| match pos.cmp(&from) {
                Ordering::Equal => None,
                Ordering::Greater => Some((from, pos)),
                Ordering::Less => Some((pos, from)),
            })
    }

    fn previous_boundary(text: &str, pos: usize) -> usize {
        text[..pos].char_indices().next_back().map_or(0, |(i, _)| i)
    }

    fn next_boundary(text: &str, pos: usize) -> usize {
        text[pos..]
            .chars()
            .next()
            .map_or(text.len(), |c| pos + c.len_utf8())
    }

    fn insert_text(&mut self, text: &str) -> bool {
        let current_len = self.state_ref().text.len();
        let selected_len = self.selection().map_or(0, |(a, b)| b - a);
        let available = self.limit.map_or(text.len(), |limit| {
            limit.saturating_sub(current_len - selected_len)
        });
        let end = text
            .char_indices()
            .take_while(|(i, c)| *i + c.len_utf8() <= available)
            .last()
            .map_or(0, |(i, c)| i + c.len_utf8());
        let text = &text[..end];
        if text.is_empty() {
            return false;
        }
        if let Some((a, b)) = self.selection() {
            self.state_mut().text.replace_range(a..b, text);
            self.state_mut().cursor_pos = a + text.len();
            self.state_mut().selection_from = None;
        } else {
            let pos = self.state_ref().cursor_pos;
            self.state_mut().text.insert_str(pos, text);
            self.state_mut().cursor_pos += text.len();
        }
        self.state_mut().dirty = true;
        true
    }

    fn line_y(&self, line_index: usize) -> f32 {
        self.state_ref()
            .lines
            .get(line_index)
            .map_or(0.0, |line| line.baseline_y - line.max_ascent)
    }

    fn line_for_glyph(&self, glyph_index: usize) -> Option<usize> {
        let st = self.state_ref();
        if glyph_index >= st.glyphs.len() {
            Some(st.lines.len().saturating_sub(1))
        } else {
            self.state_ref()
                .lines
                .iter()
                .position(|line| line.glyph_start <= glyph_index && glyph_index <= line.glyph_end)
        }
    }

    fn position_on_line(&self, x: f32, line_index: usize) -> usize {
        let offset = self.state_ref().padding_offset_px;
        let Some(line) = self.state_ref().lines.get(line_index) else {
            return self.state_ref().text.len();
        };
        let Some(glyphs) = self
            .state_ref()
            .glyphs
            .get(line.glyph_start..=line.glyph_end)
        else {
            return 0;
        };

        let x = x - offset;
        let max_pos = if line_index == self.state_ref().lines.len().saturating_sub(1) {
            self.state_ref().text.len()
        } else {
            line.glyph_end
        };
        let mut end = glyphs.first().map_or(0, |glyph| glyph.byte_offset);
        for glyph in glyphs {
            if x < glyph.x + glyph.width as f32 / 2.0 {
                return glyph.byte_offset;
            }
            end = glyph.byte_offset + glyph.parent.len_utf8();
            if x <= glyph.x + glyph.width as f32 {
                return end.min(max_pos);
            }
        }
        end.min(max_pos)
    }

    fn cursor_point(&self, pos: usize) -> Point {
        let glyphs = &self.state_ref().glyphs;
        let len = glyphs.len();
        let offset = self.state_ref().padding_offset_px;
        if pos == 0 || len == 0 {
            Point::new(offset, offset)
        } else if pos < len {
            let line_index = self.line_for_glyph(pos).expect("Glyph index out of bounds");
            let y = self.line_y(line_index) + offset;
            // Provide a 1px gap between the next glyph and the cursor
            let x = glyphs[pos].x + offset - 1.0;
            Point::new(x, y)
        } else if pos >= len && self.state_ref().text.ends_with('\n') {
            let line_index = self.line_for_glyph(pos).expect("Glyph index out of bounds");
            // We're at the last line and we end with a newline, advance to the next line
            let y = self.line_y(line_index) + offset + self.state_ref().line_height;
            Point::new(offset, y)
        } else {
            // Cursor is at the end of the text
            let line_index = self.state_ref().lines.len().saturating_sub(1);
            let y = self.line_y(line_index) + offset;
            let x = glyphs
                .last()
                .map_or(offset, |glyph| glyph.x + glyph.width as f32 + offset);
            Point::new(x, y)
        }
    }

    fn cursor_y_range(&self) -> (f32, f32) {
        let pos = self.state_ref().cursor_pos;
        let point = self.cursor_point(pos);
        let start = point.y;
        let end = point.y + self.state_ref().line_height;
        (start, end)
    }

    fn position(&self, x: f32, y: f32) -> usize {
        let offset = self.state_ref().padding_offset_px;
        let target_y = (y - offset).max(0.0);
        if let Some((last_line_index, last_line)) =
            self.state_ref().lines.iter().enumerate().next_back()
            && target_y > self.line_y(last_line_index) + last_line.max_new_line_size
        {
            return self.state_ref().glyphs.len();
        }
        let line_index = self
            .state_ref()
            .lines
            .iter()
            .enumerate()
            .min_by(|(a, _), (b, _)| {
                (self.line_y(*a) - target_y)
                    .abs()
                    .partial_cmp(&(self.line_y(*b) - target_y).abs())
                    .unwrap()
            })
            .map_or(0, |(line_index, _)| line_index);
        self.position_on_line(x, line_index)
    }

    fn select_word(&mut self) -> bool {
        let pos = self.state_ref().cursor_pos;
        let text = &self.state_ref().text;
        let mut start = pos;
        while start > 0
            && text[..start]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric)
        {
            start = Self::previous_boundary(text, start);
        }
        let mut end = pos;
        while end < text.len()
            && text[end..]
                .chars()
                .next()
                .is_some_and(char::is_alphanumeric)
        {
            end = Self::next_boundary(text, end);
        }
        if start != end {
            self.state_mut().selection_from = Some(start);
            self.state_mut().cursor_pos = end;
            true
        } else {
            false
        }
    }

    fn cut(&mut self) -> bool {
        if let Some((a, b)) = self.selection() {
            crate::window::put_on_clipboard(&self.state_ref().text[a..b].into());
            self.state_mut().text.replace_range(a..b, "");
            self.state_mut().cursor_pos = a;
            self.state_mut().selection_from = None;
            self.state_mut().dirty = true;
            true
        } else {
            false
        }
    }

    fn copy(&self) -> bool {
        if let Some((a, b)) = self.selection() {
            crate::window::put_on_clipboard(&self.state_ref().text[a..b].into());
            true
        } else {
            false
        }
    }

    fn paste(&mut self) -> bool {
        if let Some(crate::Data::String(text)) = crate::window::get_from_clipboard() {
            self.insert_text(&text)
        } else {
            false
        }
    }

    fn handle_action(&mut self, action: TextBoxAction) -> Vec<Message> {
        let changed = match action {
            TextBoxAction::Cut => self.cut(),
            TextBoxAction::Copy => {
                self.copy();
                false
            }
            TextBoxAction::Paste => self.paste(),
        };
        if changed {
            vec![Box::new(TextBoxMessage::Change(
                self.state_ref().text.clone(),
            ))]
        } else {
            vec![]
        }
    }
}

#[state_component_impl(TextAreaTextState, Internal)]
impl Component for TextAreaText {
    fn init(&mut self) {
        self.reset_state();
    }

    fn props_hash(&self, hasher: &mut ComponentHasher) {
        self.default_text.hash(hasher);
    }

    fn new_props(&mut self) {
        self.reset_state();
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        message
            .downcast_ref::<TextBoxAction>()
            .map_or_else(Vec::new, |action| self.handle_action(*action))
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) {
        event.stop_bubbling();
    }

    fn on_mouse_enter(&mut self, _event: &mut event::Event<event::MouseEnter>) {
        crate::window::set_cursor("Ibeam");
    }

    fn on_mouse_leave(&mut self, _event: &mut event::Event<event::MouseLeave>) {
        crate::window::unset_cursor();
    }

    fn on_tick(&mut self, _event: &mut event::Event<event::Tick>) {
        if self.state_ref().focused {
            let visible =
                (self.state_ref().activated_at.elapsed().as_millis() / CURSOR_BLINK_PERIOD) % 2
                    == 0;
            if visible != self.state_ref().cursor_visible {
                self.state_mut().cursor_visible = visible;
                self.dirty = Dirty::RenderOnly;
            }
        }
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) {
        if event.input.button == crate::input::MouseButton::Left {
            if self.state_ref().dragging {
                return;
            }
            self.activate();
            let mouse = event.relative_physical_position();
            self.state_mut().cursor_pos = self.position(mouse.x, mouse.y);
            self.dirty = Dirty::RenderOnly;
            event.emit(msg!(TextAreaContainerMessage::SetFocusedYRange(
                self.cursor_y_range(),
            )));
        }
        event.focus();
    }

    fn on_double_click(&mut self, event: &mut event::Event<event::DoubleClick>) {
        event.focus();
        self.select_word();
        event.emit(msg!(TextAreaContainerMessage::SetFocusedYRange(
            self.cursor_y_range(),
        )));
        self.dirty = Dirty::RenderOnly;
    }

    fn on_focus(&mut self, event: &mut event::Event<event::Focus>) {
        self.state_mut().focused = true;
        self.state_mut().cursor_visible = true;
        self.state_mut().activated_at = Instant::now();
        event.emit(Box::new(TextBoxMessage::Open));
    }

    fn on_blur(&mut self, event: &mut event::Event<event::Blur>) {
        self.state_mut().focused = false;
        self.state_mut().cursor_visible = false;
        self.state_mut().selection_from = None;
        event.emit(Box::new(TextBoxMessage::Close));
        if self.commit_on_blur {
            event.emit(Box::new(TextBoxMessage::Commit(
                self.state_ref().text.clone(),
            )));
        }
    }

    fn on_key_down(&mut self, event: &mut event::Event<event::KeyDown>) {
        let pos = self.state_ref().cursor_pos;
        let len = self.state_ref().text.len();
        let mut changed = false;
        let mut handled = true;
        match event.input.key {
            Key::Backspace => {
                if let Some((a, b)) = self.selection() {
                    self.state_mut().text.replace_range(a..b, "");
                    self.state_mut().cursor_pos = a;
                    self.state_mut().selection_from = None;
                    changed = true;
                } else if pos > 0 {
                    let a = Self::previous_boundary(&self.state_ref().text, pos);
                    self.state_mut().text.replace_range(a..pos, "");
                    self.state_mut().cursor_pos = a;
                    changed = true;
                }
            }
            Key::Delete => {
                if let Some((a, b)) = self.selection() {
                    self.state_mut().text.replace_range(a..b, "");
                    self.state_mut().cursor_pos = a;
                    self.state_mut().selection_from = None;
                    changed = true;
                } else if pos < len {
                    let b = Self::next_boundary(&self.state_ref().text, pos);
                    self.state_mut().text.replace_range(pos..b, "");
                    changed = true;
                }
            }
            Key::Left | Key::Right => {
                let next = if event.input.key == Key::Left {
                    Self::previous_boundary(&self.state_ref().text, pos)
                } else {
                    Self::next_boundary(&self.state_ref().text, pos)
                };
                if event.modifiers_held.shift {
                    if self.state_ref().selection_from.is_none() {
                        self.state_mut().selection_from = Some(pos);
                    }
                    self.state_mut().cursor_pos = next;
                } else if self.state_ref().selection_from.is_some() {
                    self.state_mut().selection_from = None;
                } else {
                    self.state_mut().cursor_pos = next;
                }
            }
            Key::Up | Key::Down => {
                let cursor = self.cursor_point(pos);
                let current_line = self.line_for_glyph(pos).unwrap_or(0);
                let next_line = if event.input.key == Key::Up {
                    current_line.checked_sub(1)
                } else {
                    Some(current_line + 1)
                };
                let next = next_line.map_or(pos, |line| self.position_on_line(cursor.x, line));
                if event.modifiers_held.shift && self.state_ref().selection_from.is_none() {
                    self.state_mut().selection_from = Some(pos);
                } else if !event.modifiers_held.shift {
                    self.state_mut().selection_from = None;
                }
                self.state_mut().cursor_pos = next;
            }
            Key::Return => {
                changed = self.insert_text("\n");
            }
            Key::Escape => event.blur(),
            Key::X if event.modifiers_held.ctrl => changed = self.cut(),
            Key::C if event.modifiers_held.ctrl => {
                self.copy();
            }
            Key::V if event.modifiers_held.ctrl => changed = self.paste(),
            _ => handled = false,
        }
        if changed {
            self.state_mut().dirty = true;
            event.emit(Box::new(TextBoxMessage::Change(
                self.state_ref().text.clone(),
            )));
        } else {
            self.dirty = Dirty::Full;
        }
        if handled {
            event.emit(msg!(TextAreaContainerMessage::SetFocusedYRange(
                self.cursor_y_range(),
            )));

            event.stop_bubbling();
        }
        self.state_mut().activated_at = Instant::now();
    }

    fn on_text_entry(&mut self, event: &mut event::Event<event::TextEntry>) {
        if self.insert_text(&event.input.text) {
            event.emit(msg!(TextAreaContainerMessage::SetFocusedYRange(
                self.cursor_y_range(),
            )));

            event.emit(Box::new(TextBoxMessage::Change(
                self.state_ref().text.clone(),
            )));
        }
        event.stop_bubbling();
    }

    fn on_drag_start(&mut self, event: &mut event::Event<event::DragStart>) {
        self.activate();
        self.state_mut().selection_from = Some({
            let mouse = event.relative_physical_position();
            self.position(mouse.x, mouse.y)
        });
        self.state_mut().dragging = true;
        event.focus();
        self.dirty = Dirty::RenderOnly;
    }

    fn on_drag_end(&mut self, _event: &mut event::Event<event::DragEnd>) {
        self.state_mut().dragging = false;
        if self.selection().is_none() {
            self.state_mut().selection_from = None;
        }
        self.dirty = Dirty::RenderOnly;
    }

    fn on_drag(&mut self, event: &mut event::Event<event::Drag>) {
        let mouse = event.relative_physical_position();
        let pos = self.position(mouse.x, mouse.y);
        if pos != self.state_ref().cursor_pos {
            self.state_mut().cursor_pos = pos;
            self.dirty = Dirty::RenderOnly;
            event.emit(msg!(TextAreaContainerMessage::SetFocusedYRange(
                self.cursor_y_range(),
            )));
        }
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.style_val("font_size")
            .unwrap()
            .f32()
            .to_bits()
            .hash(hasher);
        self.style_val("text_color").unwrap().color().hash(hasher);
        self.style_val("padding")
            .unwrap()
            .f32()
            .to_bits()
            .hash(hasher);
        self.style_val("font")
            .map(|p| p.str().to_string())
            .hash(hasher);
        self.state_ref().focused.hash(hasher);
        self.state_ref().selection_from.hash(hasher);
        self.state_ref().text.hash(hasher);
        self.state_ref().cursor_pos.hash(hasher);
        self.state_ref().cursor_visible.hash(hasher);
    }

    fn focus(&self) -> Option<Point> {
        Some(self.cursor_point(self.state_ref().cursor_pos))
    }

    fn fill_bounds(
        &mut self,
        width: Option<f32>,
        _height: Option<f32>,
        max_width: Option<f32>,
        _max_height: Option<f32>,
        caches: &Caches,
        scale_factor: f32,
    ) -> (Option<f32>, Option<f32>) {
        let padding = self.style_val("padding").unwrap().f32();
        let font_size = self.style_val("font_size").unwrap().f32();
        let border = self.style_val("border_width").unwrap().f32();
        let scroll_bar_width = self.style_val("bar_width").unwrap().f32();
        let available_width = width.or(max_width);
        let layout_width = available_width
            // Only subtract one padding, and let the scroll bar take up the other side if it's wider than the padding
            .map(|w| {
                (w - 2.0 * border - padding - scroll_bar_width.max(padding)).max(0.0) * scale_factor
            })
            .unwrap_or(f32::MAX);
        if self.state_ref().dirty || self.state_ref().layout_width != Some(layout_width) {
            let font = self.style_val("font").map(|p| p.str().to_string());
            let (glyphs, lines, _) = caches.layout_text(
                &[TextSegment {
                    text: alloc::borrow::Cow::Owned(self.state_ref().text.clone()),
                    size: Some(font_size),
                    font: font.clone(),
                }],
                font.as_deref(),
                font_size,
                scale_factor,
                HorizontalPosition::Left,
                (layout_width, f32::MAX),
            );
            self.state_mut().glyphs = glyphs;
            self.state_mut().lines = lines;
            self.state_mut().padding_offset_px = ((padding + border) * scale_factor).round();
            self.state_mut().line_height =
                caches.line_height(font.as_deref(), font_size, scale_factor);
            self.state_mut().layout_width = Some(layout_width);
            self.state_mut().dirty = false;
        }

        let intrinsic_width = self
            .state_ref()
            .glyphs
            .last()
            .map_or(0.0, |g| g.x + g.width as f32)
            + self.state_ref().padding_offset_px * 2.0;
        let trailing_newlines = self
            .state_ref()
            .text
            .chars()
            .rev()
            .take_while(|c| *c == '\n')
            .count();
        let height = self.state_ref().glyphs.last().map_or(
            self.state_ref().line_height * (trailing_newlines + 1) as f32,
            |g| g.y + self.state_ref().line_height * (trailing_newlines + 1) as f32,
        ) / scale_factor
            + padding * 2.0
            + border * 2.0;
        (
            Some(available_width.unwrap_or(intrinsic_width / scale_factor)),
            Some(height),
        )
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        use crate::renderable::Text;

        let offset = self.state_ref().padding_offset_px;
        let line_height = self.state_ref().line_height;
        let cursor = self.cursor_point(self.state_ref().cursor_pos);
        let text_color: Color = self.style_val("text_color").into();
        let cursor_color: Color = self.style_val("cursor_color").into();
        let selection_color: Color = self.style_val("selection_color").into();
        let mut output = vec![];
        if !self.state_ref().glyphs.is_empty() {
            output.push(Renderable::Text(Text::new(
                self.state_ref().glyphs.clone(),
                Pos::new(offset, offset, 5.0),
                text_color,
                context.caches,
                nth_prev_as_text!(context, 0),
            )));
        }
        if self.state_ref().cursor_visible && self.selection().is_none() {
            output.push(Renderable::Rectangle(Rectangle::new(
                Pos::new(cursor.x, cursor.y + 2.0, 2.0),
                Scale::new(1.0, (line_height - 2.0).max(1.0)),
                cursor_color,
            )));
        } else if let Some((a, b)) = self.selection() {
            let lines = &self.state_ref().lines;
            for (line_index, line) in lines.iter().enumerate() {
                let Some(glyphs) = self
                    .state_ref()
                    .glyphs
                    .get(line.glyph_start..=line.glyph_end)
                else {
                    continue;
                };
                let mut selected = glyphs
                    .iter()
                    .filter(|glyph| {
                        glyph.byte_offset < b && glyph.byte_offset + glyph.parent.len_utf8() > a
                    })
                    .peekable();
                let Some(first) = selected.peek() else {
                    continue;
                };
                let start_x = first.x + offset;
                let end_x = selected
                    .last()
                    .map_or(start_x, |glyph| glyph.x + glyph.width as f32 + offset);
                let top = line.baseline_y - line.max_ascent;
                let bottom = lines
                    .get(line_index + 1)
                    .map_or(top + line.max_new_line_size, |next| {
                        next.baseline_y - next.max_ascent
                    });
                output.push(Renderable::Rectangle(Rectangle::new(
                    Pos::new(start_x, top + offset, 2.0),
                    Scale::new(end_x - start_x, bottom - top),
                    selection_color,
                )));
            }
        }
        Some(output)
    }
}
