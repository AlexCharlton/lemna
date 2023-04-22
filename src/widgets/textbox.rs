use std::hash::Hash;
use std::time::Instant;

use glyph_brush_layout::ab_glyph::{Font, ScaleFont};

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, Message, RenderContext};
use crate::event;
use crate::font_cache::{FontCache, HorizontalAlign, SectionText};
use crate::input::Key;
use crate::layout::ScrollPosition;
use crate::render::wgpu::{self, WGPURenderable, WGPURenderer};
use crate::{node, Node};
use lemna_macros::{state_component, state_component_impl};

const CURSOR_BLINK_PERIOD: u128 = 500; // millis

#[derive(Debug, Clone)]
pub struct TextBoxStyle {
    pub font_size: f32,
    pub text_color: Color,
    pub font: Option<String>,
    pub background_color: Color,
    pub selection_color: Color,
    pub cursor_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub padding: f32,
}

impl Default for TextBoxStyle {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            text_color: Color::BLACK,
            font: None,
            background_color: Color::WHITE,
            selection_color: 0.5.into(),
            cursor_color: Color::BLACK,
            border_color: Color::BLACK,
            border_width: 1.0,
            padding: 1.0,
        }
    }
}

#[derive(Debug)]
enum TextBoxMessage {
    Open,
    Close,
    Change(String),
    Commit(String),
}

#[derive(Debug, Copy, Clone)]
pub enum TextBoxAction {
    Cut,
    Copy,
    Paste,
}

#[derive(Debug, Default)]
struct TextBoxState {
    focused: bool,
}

#[state_component(TextBoxState)]
pub struct TextBox {
    style: TextBoxStyle,
    text: Option<String>,
    on_change: Option<Box<dyn Fn(&str) -> Message + Send + Sync>>,
    on_commit: Option<Box<dyn Fn(&str) -> Message + Send + Sync>>,
    on_focus: Option<Box<dyn Fn() -> Message + Send + Sync>>,
}

impl std::fmt::Debug for TextBox {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("TextBox")
            .field("text", &self.text)
            .field("style", &self.style)
            .finish()
    }
}

impl TextBox {
    pub fn new(default: Option<String>, style: TextBoxStyle) -> Self {
        Self {
            style,
            text: default,
            on_change: None,
            on_commit: None,
            on_focus: None,
            state: Some(TextBoxState::default()),
        }
    }

    pub fn on_change(mut self, change_fn: Box<dyn Fn(&str) -> Message + Send + Sync>) -> Self {
        self.on_change = Some(change_fn);
        self
    }

    pub fn on_commit(mut self, commit_fn: Box<dyn Fn(&str) -> Message + Send + Sync>) -> Self {
        self.on_commit = Some(commit_fn);
        self
    }

    pub fn on_focus(mut self, focus_fn: Box<dyn Fn() -> Message + Send + Sync>) -> Self {
        self.on_focus = Some(focus_fn);
        self
    }
}

#[state_component_impl(TextBoxState)]
impl Component<WGPURenderer> for TextBox {
    fn view(&self) -> Option<Node<WGPURenderer>> {
        Some(
            node!(
                TextBoxContainer::new(
                    self.style.background_color,
                    self.style.border_color,
                    self.style.border_width * if self.state_ref().focused { 2.0 } else { 1.0 },
                ),
                lay!(size: size_pct!(100.0),)
            )
            .push(node!(
                TextBoxText::new(self.text.clone(), self.style.clone(),),
                lay!(size: size_pct!(100.0),)
            )),
        )
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        let mut m: Vec<Message> = vec![];
        match message.downcast_ref::<TextBoxMessage>() {
            Some(TextBoxMessage::Open) => {
                self.state_mut().focused = true;
                if let Some(focus_fn) = &self.on_focus {
                    m.push(focus_fn())
                }
            }
            Some(TextBoxMessage::Close) => self.state_mut().focused = false,
            Some(TextBoxMessage::Change(s)) => {
                if let Some(change_fn) = &self.on_change {
                    m.push(change_fn(s))
                }
            }
            Some(TextBoxMessage::Commit(s)) => {
                if let Some(commit_fn) = &self.on_commit {
                    m.push(commit_fn(s))
                }
            }
            _ => m.push(message),
        }
        m
    }
}

#[derive(Debug, Default)]
#[allow(dead_code)]
struct TextBoxContainerState {
    scroll_position: f32,
    border_width_px: f32,
    width_px: f32,
}

#[state_component(TextBoxContainerState)]
#[derive(Debug)]
struct TextBoxContainer {
    background_color: Color,
    border_color: Color,
    border_width: f32,
}

impl TextBoxContainer {
    fn new(background_color: Color, border_color: Color, border_width: f32) -> Self {
        Self {
            background_color,
            border_color,
            border_width,
            state: Some(Default::default()),
        }
    }

    fn border_width_px(&self, scale_factor: f32) -> f32 {
        (self.border_width * scale_factor.floor()).round()
    }
}

#[state_component_impl(TextBoxContainerState)]
impl Component<WGPURenderer> for TextBoxContainer {
    fn full_control(&self) -> bool {
        true
    }

    fn set_aabb(
        &mut self,
        aabb: &mut AABB,
        _parent_aabb: AABB,
        mut children: Vec<(&mut AABB, Option<Scale>, Option<Point>)>,
        _frame: AABB,
        scale_factor: f32,
    ) {
        if let Some((child_aabb, _, Some(focus))) = children.first_mut() {
            let width = aabb.width();
            let border_width_px = self.border_width_px(scale_factor);
            // We need to expand our child's AABB width if it's not as big as this AABB
            if child_aabb.bottom_right.x < aabb.bottom_right.x {
                child_aabb.bottom_right.x = aabb.bottom_right.x - border_width_px;
            }

            // Scroll if our child's focus is outside of our bounds
            let inner_width = width - border_width_px * 2.0;
            let scroll_position = self.state_ref().scroll_position;
            if focus.x > inner_width + scroll_position {
                self.state_mut().scroll_position = focus.x - inner_width;
            } else if focus.x < scroll_position {
                self.state_mut().scroll_position = focus.x - border_width_px;
            }
        }
    }

    fn frame_bounds(&self, aabb: AABB, _inner_scale: Option<Scale>) -> AABB {
        let mut aabb = aabb;
        let w = self.state_ref().border_width_px;
        aabb.pos.x += w;
        aabb.pos.y += w;
        aabb.bottom_right.x -= w;
        aabb.bottom_right.y -= w;
        aabb
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.background_color.hash(hasher);
        self.border_color.hash(hasher);
        (self.border_width as u32).hash(hasher);
    }

    fn scroll_position(&self) -> Option<ScrollPosition> {
        Some(ScrollPosition {
            x: Some(self.state_ref().scroll_position),
            y: None,
        })
    }

    fn render<'a>(
        &mut self,
        context: RenderContext<'a, WGPURenderer>,
    ) -> Option<Vec<WGPURenderable>> {
        let border_width = self.border_width_px(context.scale_factor);
        self.state_mut().border_width_px = border_width;

        let background = WGPURenderable::Rect(wgpu::Rect::new(
            Pos {
                x: border_width,
                y: border_width,
                z: 0.5,
            },
            context.aabb.size() - Scale::new(border_width * 2.0, border_width * 2.0),
            self.background_color,
        ));

        let border = WGPURenderable::Rect(wgpu::Rect::new(
            Pos::default(),
            context.aabb.size(),
            self.border_color,
        ));

        Some(vec![background, border])
    }
}

#[cfg(feature = "backend_wx_rs")]
#[derive(Debug)]
struct TextBoxTextState {
    focused: bool,
    text: String,
    cursor_pos: usize,
    selection_from: Option<usize>,
    activated_at: Instant,
    cursor_visible: bool,
    glyphs: Vec<crate::font_cache::SectionGlyph>,
    glyph_widths: Vec<f32>,
    padding_offset_px: f32,
    dirty: bool,
    menu: Option<wx_rs::Menu<TextBoxAction>>,
}
#[derive(Debug)]
#[cfg(not(feature = "backend_wx_rs"))]
struct TextBoxTextState {
    focused: bool,
    text: String,
    cursor_pos: usize,
    selection_from: Option<usize>,
    activated_at: Instant,
    cursor_visible: bool,
    glyphs: Vec<crate::font_cache::SectionGlyph>,
    glyph_widths: Vec<f32>,
    padding_offset_px: f32,
    dirty: bool,
}

#[state_component(TextBoxTextState)]
#[derive(Debug)]
pub struct TextBoxText {
    pub style: TextBoxStyle,
    pub default_text: String,
}

impl TextBoxText {
    pub fn new(text: Option<String>, style: TextBoxStyle) -> Self {
        Self {
            style,
            default_text: text.unwrap_or(String::new()),
            state: None,
        }
    }

    fn reset_state(&mut self) {
        self.state = Some(TextBoxTextState {
            focused: false,
            text: self.default_text.clone(),
            cursor_pos: 0,
            selection_from: None,
            activated_at: Instant::now(),
            cursor_visible: false,
            glyphs: vec![],
            glyph_widths: vec![],
            padding_offset_px: 0.0,
            dirty: true,
            #[cfg(feature = "backend_wx_rs")]
            menu: None,
        });
    }

    fn selection(&self) -> Option<(usize, usize)> {
        let pos = self.state_ref().cursor_pos;
        self.state_ref().selection_from.and_then(|selection_from| {
            if pos == selection_from {
                None
            } else if pos > selection_from {
                Some((selection_from, pos))
            } else {
                Some((pos, selection_from))
            }
        })
    }

    fn position(&self, x: f32) -> usize {
        if let Some(i) = self
            .state_ref()
            .glyphs
            .iter()
            .position(|g| x < g.glyph.position.x + 4.0)
        // This should really be checking against the glyph center
        {
            i
        } else {
            self.state_ref().text.len()
        }
    }

    fn insert_text(&mut self, text: &str) {
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
    }

    fn activate(&mut self) {
        self.state_mut().activated_at = Instant::now();
        self.state_mut().cursor_visible = true;
        self.state_mut().selection_from = None;
    }

    fn cursor_position_px(&self, pos: usize) -> f32 {
        let len = self.state_ref().text.len();
        let glyphs = &self.state_ref().glyphs;
        (if pos < len {
            let g = &glyphs[pos].glyph;
            g.position.x
        } else if pos == 0 {
            0.0
        } else {
            // Last glyph, need to add the advance
            let g = &glyphs[pos - 1].glyph;
            g.position.x + self.state_ref().glyph_widths.last().map_or(0.0, |w| *w)
        }) + self.state_ref().padding_offset_px
    }

    fn cut(&mut self) -> bool {
        if let Some((a, b)) = self.selection() {
            if let Some(w) = crate::current_window() { w.put_on_clipboard(&self.state_ref().text[a..b].into()) }
            self.insert_text("");
            true
        } else {
            false
        }
    }

    fn copy(&mut self) -> bool {
        if let Some((a, b)) = self.selection() {
            if let Some(w) = crate::current_window() { w.put_on_clipboard(&self.state_ref().text[a..b].into()) }
            true
        } else {
            false
        }
    }

    fn paste(&mut self) -> bool {
        if let Some(crate::input::Data::String(text)) =
            crate::current_window().and_then(|w| w.get_from_clipboard())
        {
            self.insert_text(&text);
            true
        } else {
            false
        }
    }

    fn handle_action(&mut self, action: TextBoxAction) -> Vec<Message> {
        match action {
            TextBoxAction::Cut => {
                self.cut();
                vec![Box::new(TextBoxMessage::Change(
                    self.state_ref().text.clone(),
                ))]
            }
            TextBoxAction::Copy => {
                self.copy();
                vec![]
            }
            TextBoxAction::Paste => {
                self.paste();
                vec![Box::new(TextBoxMessage::Change(
                    self.state_ref().text.clone(),
                ))]
            }
        }
    }
}

#[state_component_impl(TextBoxTextState)]
impl Component<WGPURenderer> for TextBoxText {
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
        if let Some(action) = message.downcast_ref::<TextBoxAction>() {
            self.handle_action(*action)
        } else {
            vec![]
        }
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) -> Vec<Message> {
        event.stop_bubbling();
        if let Some(w) = crate::current_window() { w.set_cursor("Ibeam") }
        vec![]
    }

    fn on_mouse_leave(&mut self, _event: &mut event::Event<event::MouseLeave>) -> Vec<Message> {
        if let Some(w) = crate::current_window() { w.unset_cursor() }
        vec![]
    }

    fn on_tick(&mut self, event: &mut event::Event<event::Tick>) -> Vec<Message> {
        if self.state_ref().focused {
            let visible =
                (self.state_ref().activated_at.elapsed().as_millis() / CURSOR_BLINK_PERIOD) % 2
                    == 0;
            if visible != self.state_ref().cursor_visible {
                self.state_mut().cursor_visible = visible;
                event.dirty();
            }
        }
        vec![]
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) -> Vec<Message> {
        match event.input.0 {
            crate::input::MouseButton::Left => {
                self.activate();
                let new_pos = self.position(event.relative_physical_position().x);
                if new_pos != self.state_ref().cursor_pos {
                    self.state_mut().cursor_pos = new_pos;
                    event.dirty();
                }
            }
            #[cfg(feature = "backend_wx_rs")]
            crate::input::MouseButton::Right => {
                use wx_rs::{Menu, MenuEntry};
                crate::focus_immediately(event);

                if let Some(menu) = &self.state_ref().menu {
                    menu.popup();
                } else {
                    let menu = Menu::new(None)
                        .push_entry(MenuEntry::new(TextBoxAction::Cut, "&Cut".to_string()))
                        .push_entry(MenuEntry::new(TextBoxAction::Copy, "&Copy".to_string()))
                        .push_entry(MenuEntry::new(TextBoxAction::Paste, "&Paste".to_string()));
                    self.state_mut().menu = Some(menu);
                    self.state_ref().menu.as_ref().unwrap().popup();
                }
            }
            _ => (),
        }

        event.stop_bubbling();
        event.focus();

        vec![]
    }

    #[cfg(feature = "backend_wx_rs")]
    fn on_menu_select(&mut self, event: &mut event::Event<event::MenuSelect>) -> Vec<Message> {
        if let Some(action) = self
            .state_ref()
            .menu
            .as_ref()
            .and_then(|menu| menu.get_entry_from_event_id(event.input.0))
        {
            event.stop_bubbling();
            event.dirty();
            self.handle_action(action)
        } else {
            vec![]
        }
    }

    fn on_double_click(&mut self, _event: &mut event::Event<event::DoubleClick>) -> Vec<Message> {
        // TODO select words
        vec![]
    }

    fn on_focus(&mut self, event: &mut event::Event<event::Focus>) -> Vec<Message> {
        self.state_mut().focused = true;
        self.state_mut().cursor_visible = true;
        event.dirty();
        vec![Box::new(TextBoxMessage::Open)]
    }

    fn on_blur(&mut self, event: &mut event::Event<event::Blur>) -> Vec<Message> {
        self.state_mut().focused = false;
        self.state_mut().cursor_visible = false;
        self.state_mut().selection_from = None;
        self.state_mut().cursor_pos = 0;
        event.dirty();
        vec![
            Box::new(TextBoxMessage::Close),
            Box::new(TextBoxMessage::Commit(self.state_ref().text.clone())),
        ]
    }

    fn on_key_down(&mut self, event: &mut event::Event<event::KeyDown>) -> Vec<Message> {
        let pos = self.state_ref().cursor_pos;
        let len = self.state_ref().text.len();
        let mut changed = false;
        match event.input.0 {
            Key::Backspace => {
                if let Some((a, b)) = self.selection() {
                    self.state_mut().text.replace_range(a..b, "");
                    self.state_mut().cursor_pos = a;
                    self.state_mut().selection_from = None;
                    changed = true;
                } else if pos > 0 {
                    self.state_mut().text.remove(pos - 1);
                    self.state_mut().cursor_pos -= 1;
                    changed = true;
                }
            }
            Key::Left => {
                // TODO more modifiers
                if pos > 0 {
                    if event.modifiers_held.shift {
                        if let Some(s) = self.state_ref().selection_from {
                            if pos == s + 1 {
                                self.state_mut().selection_from = None;
                            }
                        } else {
                            self.state_mut().selection_from = Some(pos);
                        }
                        self.state_mut().cursor_pos -= 1;
                    } else if self.state_ref().selection_from.is_some() {
                        self.state_mut().selection_from = None;
                    } else {
                        self.state_mut().cursor_pos -= 1;
                    }
                } else if !event.modifiers_held.shift && self.state_ref().selection_from.is_some() {
                    self.state_mut().selection_from = None;
                }
            }
            Key::Right => {
                // TODO more modifiers
                if pos < len {
                    if event.modifiers_held.shift {
                        if let Some(s) = self.state_ref().selection_from {
                            if pos + 1 == s {
                                self.state_mut().selection_from = None;
                            }
                        } else {
                            self.state_mut().selection_from = Some(pos);
                        }
                        self.state_mut().cursor_pos += 1;
                    } else if self.state_ref().selection_from.is_some() {
                        self.state_mut().selection_from = None;
                    } else {
                        self.state_mut().cursor_pos += 1;
                    }
                } else if !event.modifiers_held.shift && self.state_ref().selection_from.is_some() {
                    self.state_mut().selection_from = None;
                }
            }
            Key::Up => {
                // TODO more modifiers
                if event.modifiers_held.shift {
                    if pos > 0 {
                        self.state_mut().selection_from = Some(pos);
                        self.state_mut().cursor_pos = 0;
                    }
                } else {
                    self.state_mut().cursor_pos = 0;
                    self.state_mut().selection_from = None;
                }
            }
            Key::Down => {
                // TODO more modifiers
                if event.modifiers_held.shift {
                    if pos > 0 {
                        self.state_mut().selection_from = Some(pos);
                        self.state_mut().cursor_pos = len;
                    }
                } else {
                    self.state_mut().cursor_pos = len;
                    self.state_mut().selection_from = None;
                }
            }
            Key::Return => {
                event.blur();
            }
            Key::X => {
                if event.modifiers_held.ctrl {
                    changed = self.cut();
                }
            }
            Key::C => {
                if event.modifiers_held.ctrl {
                    self.copy();
                }
            }
            Key::V => {
                if event.modifiers_held.ctrl {
                    changed = self.paste();
                }
            }
            _ => (),
        }

        event.dirty();
        if changed {
            self.state_mut().dirty = true;
            vec![Box::new(TextBoxMessage::Change(
                self.state_ref().text.clone(),
            ))]
        } else {
            vec![]
        }
    }

    fn on_text_entry(&mut self, event: &mut event::Event<event::TextEntry>) -> Vec<Message> {
        self.insert_text(&event.input.0);
        self.state_mut().dirty = true;
        event.dirty();
        event.stop_bubbling();
        vec![Box::new(TextBoxMessage::Change(
            self.state_ref().text.clone(),
        ))]
    }

    fn on_drag_start(&mut self, event: &mut event::Event<event::DragStart>) -> Vec<Message> {
        self.activate();
        self.state_mut().selection_from = Some(self.position(event.relative_physical_position().x));
        event.focus();
        event.stop_bubbling();
        event.dirty();
        vec![]
    }

    fn on_drag_end(&mut self, _event: &mut event::Event<event::DragEnd>) -> Vec<Message> {
        if self.selection().is_none() {
            self.state_mut().selection_from = None;
        }
        vec![]
    }

    fn on_drag(&mut self, event: &mut event::Event<event::Drag>) -> Vec<Message> {
        let new_pos = self.position(event.relative_physical_position().x);
        if new_pos != self.state_ref().cursor_pos {
            self.state_mut().cursor_pos = new_pos;
            event.dirty();
        }
        vec![]
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        (self.style.font_size as u32).hash(hasher);
        self.style.text_color.hash(hasher);
        (self.style.padding as u32).hash(hasher);
        self.style.font.hash(hasher);
        self.state_ref().focused.hash(hasher);
        self.state_ref().selection_from.hash(hasher);
        self.state_ref().text.hash(hasher);
        self.state_ref().cursor_pos.hash(hasher);
        self.state_ref().cursor_visible.hash(hasher);
    }

    fn focus(&self) -> Option<Point> {
        Some(Point {
            x: self.cursor_position_px(self.state_ref().cursor_pos),
            y: 0.0,
        })
    }

    fn fill_bounds(
        &mut self,
        _width: Option<f32>,
        _height: Option<f32>,
        _max_width: Option<f32>,
        _max_height: Option<f32>,
        font_cache: &FontCache,
        scale_factor: f32,
    ) -> (Option<f32>, Option<f32>) {
        if self.state_ref().dirty {
            let font_size_px = self.style.font_size * super::Text::SIZE_SCALE * scale_factor;
            let font_ref = font_cache.font_or_default(self.style.font.as_deref());
            let font = &font_cache.fonts[font_ref.0];

            self.state_mut().glyphs = font_cache.layout_text(
                &[SectionText {
                    text: &self.state_ref().text,
                    scale: font_size_px.into(),
                    font_id: font_ref,
                }],
                HorizontalAlign::Left,
                (0.0, 0.0),
                (f32::MAX, f32::MAX),
            );

            let glyph_widths = self
                .state_ref()
                .glyphs
                .iter()
                .map(|g| font.as_scaled(font_size_px).h_advance(g.glyph.id))
                .collect();
            self.state_mut().glyph_widths = glyph_widths;
            self.state_mut().padding_offset_px =
                ((self.style.padding + self.style.border_width) * scale_factor).round();

            self.state_mut().dirty = false;
        }

        let width = self
            .state_ref()
            .glyphs
            .last()
            .map_or(0.0, |g| g.glyph.position.x + g.glyph.scale.x)
            + self.state_ref().padding_offset_px * 2.0;
        (
            Some(width / scale_factor),
            Some(
                self.style.font_size * super::Text::SIZE_SCALE
                    + self.style.padding * 2.0
                    + self.style.border_width * 2.0,
            ),
        )
    }

    fn render<'a>(
        &mut self,
        context: RenderContext<'a, WGPURenderer>,
    ) -> Option<Vec<WGPURenderable>> {
        let cursor_z = 2.0;
        let text_z = 5.0;
        let font_size = self.style.font_size * super::Text::SIZE_SCALE;
        let pos = self.state_ref().cursor_pos;
        let offset = self.state_ref().padding_offset_px;
        let font_size_px = font_size * context.scale_factor;
        let cursor_x = self.cursor_position_px(pos);
        let selection_from_x = self
            .state_ref()
            .selection_from
            .map(|pos| self.cursor_position_px(pos));

        let mut renderables = vec![];

        if !self.state_ref().glyphs.is_empty() {
            let text = WGPURenderable::Text(wgpu::Text::new(
                self.state_ref().glyphs.clone(),
                Pos {
                    x: offset,
                    y: offset,
                    z: text_z,
                },
                self.style.text_color,
                &mut context.renderer.text_pipeline,
                context.prev_state.and_then(|v| match v.get(0) {
                    Some(WGPURenderable::Text(r)) => Some(r.buffer_id),
                    _ => None,
                }),
            ));

            renderables.push(text);
        }

        if self.state_ref().cursor_visible && self.selection().is_none() {
            let cursor_rect = WGPURenderable::Rect(wgpu::Rect::new(
                Pos::new(cursor_x, offset + 2.0, cursor_z),
                Scale::new(1.0, font_size_px - offset),
                self.style.cursor_color,
            ));
            renderables.push(cursor_rect);
        } else if self.selection().is_some() {
            let (x1, x2) = if cursor_x > selection_from_x.unwrap() {
                (selection_from_x.unwrap(), cursor_x)
            } else {
                (cursor_x, selection_from_x.unwrap())
            };

            let selection_rect = WGPURenderable::Rect(wgpu::Rect::new(
                Pos::new(x1, offset + 2.0, cursor_z),
                Scale::new(x2 - x1, font_size_px - offset),
                self.style.selection_color,
            ));
            renderables.push(selection_rect);
        }

        Some(renderables)
    }
}
