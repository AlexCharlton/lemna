use std::collections::HashSet;
use std::time::Instant;

use super::base_types::*;
use super::input::{Key, MouseButton};
use crate::Message;

/// How much time (ms) can elapse between clicks before it's no longer considered a double click
pub const DOUBLE_CLICK_INTERVAL_MS: u128 = 500; // ms
/// How much mouse travel (px) is allowed before it's no longer considered a double click
pub const DOUBLE_CLICK_MAX_DIST: f32 = 10.0; // px
/// How much distance (px) is required before we start a drag event
pub const DRAG_THRESHOLD: f32 = 15.0; // px
/// How much mouse travel (px) is allowed until we'll no longer send a click event
/// Note this is longer than DRAG_THRESHOLD
pub const DRAG_CLICK_MAX_DIST: f32 = 30.0; // px

pub struct Event<T: EventInput> {
    pub input: T,
    pub(crate) bubbles: bool,
    pub(crate) dirty: bool,
    pub(crate) mouse_position: Point,
    pub modifiers_held: ModifiersHeld,
    pub(crate) current_node_id: Option<u64>,
    pub(crate) current_aabb: Option<AABB>,
    pub(crate) current_inner_scale: Option<Scale>,
    pub(crate) over_child_n: Option<usize>,
    pub(crate) over_subchild_n: Option<usize>,
    pub(crate) target: Option<u64>,
    pub(crate) focus: Option<u64>,
    pub(crate) scale_factor: f32,
    pub(crate) messages: Vec<Message>,
    pub(crate) registrations: Vec<crate::node::Registration>,
}

impl<T: EventInput> std::fmt::Debug for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Event")
            .field("input", &self.input)
            .field("bubbles", &self.bubbles)
            .field("dirty", &self.dirty)
            .field("mouse_position", &self.mouse_position)
            .field("modifiers_held", &self.modifiers_held)
            .field("current_node_id", &self.current_node_id)
            .field("current_aabb", &self.current_aabb)
            .field("current_inner_scale", &self.current_inner_scale)
            .field("over_child_n", &self.over_child_n)
            .field("over_subchild_n", &self.over_subchild_n)
            .field("target", &self.target)
            .field("focus", &self.focus)
            .field("scale_factor", &self.scale_factor)
            .finish()
    }
}

/// Types that can be Event inputs
pub trait EventInput: std::fmt::Debug {
    #[doc(hidden)]
    // For internal use only
    fn matching_registrations(&self, _: &[crate::node::Registration]) -> Vec<u64> {
        vec![]
    }
}

#[derive(Debug)]
pub struct Focus;
impl EventInput for Focus {}
#[derive(Debug)]
pub struct Blur;
impl EventInput for Blur {}
#[derive(Debug)]
pub struct Tick;
impl EventInput for Tick {}
#[derive(Debug)]
pub struct MouseMotion;
impl EventInput for MouseMotion {}
#[derive(Debug)]
pub struct MouseDown(pub MouseButton);
impl EventInput for MouseDown {}
#[derive(Debug)]
pub struct MouseUp(pub MouseButton);
impl EventInput for MouseUp {}
#[derive(Debug)]
pub struct MouseEnter;
impl EventInput for MouseEnter {}
#[derive(Debug)]
pub struct MouseLeave;
impl EventInput for MouseLeave {}
#[derive(Debug)]
pub struct Click(pub MouseButton);
impl EventInput for Click {}
#[derive(Debug)]
pub struct DoubleClick(pub MouseButton);
impl EventInput for DoubleClick {}
#[derive(Debug)]
pub struct KeyDown(pub Key);
impl EventInput for KeyDown {
    fn matching_registrations(&self, registrations: &[crate::node::Registration]) -> Vec<u64> {
        registrations
            .iter()
            .filter_map(|(r, node_id)| match r {
                Register::KeyDown => Some(*node_id),
                _ => None,
            })
            .collect()
    }
}
#[derive(Debug)]
pub struct KeyUp(pub Key);
impl EventInput for KeyUp {
    fn matching_registrations(&self, registrations: &[crate::node::Registration]) -> Vec<u64> {
        registrations
            .iter()
            .filter_map(|(r, node_id)| match r {
                Register::KeyUp => Some(*node_id),
                _ => None,
            })
            .collect()
    }
}
#[derive(Debug)]
pub struct KeyPress(pub Key);
impl EventInput for KeyPress {
    fn matching_registrations(&self, registrations: &[crate::node::Registration]) -> Vec<u64> {
        registrations
            .iter()
            .filter_map(|(r, node_id)| match r {
                Register::KeyPress => Some(*node_id),
                _ => None,
            })
            .collect()
    }
}
#[derive(Debug)]
pub struct TextEntry(pub String);
impl EventInput for TextEntry {}
#[derive(Debug, Copy, Clone)]
pub struct Scroll {
    pub x: f32,
    pub y: f32,
}
impl EventInput for Scroll {}
#[derive(Debug, Copy, Clone)]
pub struct Drag {
    pub button: MouseButton,
    pub start_pos: Point,
}
impl EventInput for Drag {}
#[derive(Debug)]
pub struct DragStart(pub MouseButton);
impl EventInput for DragStart {}
#[derive(Debug, Copy, Clone)]
pub struct DragEnd {
    pub button: MouseButton,
    pub start_pos: Point,
}
impl EventInput for DragEnd {}
#[derive(Debug)]
pub struct DragTarget;
impl EventInput for DragTarget {}
#[derive(Debug)]
pub struct DragEnter(pub Vec<Data>);
impl EventInput for DragEnter {}
#[derive(Debug)]
pub struct DragLeave;
impl EventInput for DragLeave {}
#[derive(Debug)]
pub struct DragDrop(pub Data);
impl EventInput for DragDrop {}
#[derive(Debug)]
pub struct MenuSelect(pub i32);
impl EventInput for MenuSelect {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Register {
    KeyDown,
    KeyUp,
    KeyPress,
    // Maybe TODO: Include Tick?
}

impl Scalable for Scroll {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            x: self.x * scale_factor,
            y: self.y * scale_factor,
        }
    }
}

impl Scalable for Drag {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            button: self.button,
            start_pos: self.start_pos.scale(scale_factor),
        }
    }
}

impl Scalable for DragEnd {
    fn scale(self, scale_factor: f32) -> Self {
        Self {
            button: self.button,
            start_pos: self.start_pos.scale(scale_factor),
        }
    }
}

impl<T: EventInput> Event<T> {
    pub(crate) fn new(input: T, event_cache: &EventCache) -> Self {
        Self {
            input,
            bubbles: true,
            dirty: false,
            modifiers_held: event_cache.modifiers_held,
            mouse_position: event_cache.mouse_position,
            focus: Some(event_cache.focus),
            target: None,
            current_node_id: None,
            current_aabb: None,
            current_inner_scale: None,
            over_child_n: None,
            over_subchild_n: None,
            scale_factor: event_cache.scale_factor,
            messages: vec![],
            registrations: vec![],
        }
    }

    pub fn focus(&mut self) {
        self.focus = self.current_node_id;
    }

    pub fn blur(&mut self) {
        self.focus = None;
    }

    pub fn stop_bubbling(&mut self) {
        self.bubbles = false;
    }

    pub(crate) fn dirty(&mut self) {
        self.dirty = true;
    }

    pub fn emit(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    pub fn current_physical_aabb(&self) -> AABB {
        self.current_aabb.unwrap()
    }

    pub fn current_logical_aabb(&self) -> AABB {
        self.current_aabb.unwrap().unscale(self.scale_factor)
    }

    pub fn current_inner_scale(&self) -> Option<Scale> {
        self.current_inner_scale
    }

    pub fn physical_mouse_position(&self) -> Point {
        self.mouse_position
    }

    pub fn logical_mouse_position(&self) -> Point {
        self.mouse_position.unscale(self.scale_factor)
    }

    pub fn relative_physical_position(&self) -> Point {
        let pos = self.current_aabb.unwrap().pos;
        self.mouse_position - Point { x: pos.x, y: pos.y }
    }

    pub fn relative_logical_position(&self) -> Point {
        let pos = self.current_aabb.unwrap().pos;
        (self.mouse_position - Point { x: pos.x, y: pos.y }).unscale(self.scale_factor)
    }

    pub fn over_subchild_n(&self) -> Option<usize> {
        self.over_subchild_n
    }

    pub(crate) fn matching_registrations(&self) -> Vec<u64> {
        self.input.matching_registrations(&self.registrations)
    }

    // Unclear if this needs to be exposed
    #[allow(dead_code)]
    pub(crate) fn focus_immediately(&self) {
        crate::focus_immediately(self)
    }
}

impl<T: Scalable + Copy + EventInput> Event<T> {
    pub fn input_unscaled(&self) -> T {
        self.input.unscale(self.scale_factor)
    }
}

impl Event<Drag> {
    pub fn physical_delta(&self) -> Point {
        self.mouse_position - self.input.start_pos
    }

    pub fn logical_delta(&self) -> Point {
        self.physical_delta().unscale(self.scale_factor)
    }

    pub fn bounded_physical_delta(&self) -> Point {
        self.mouse_position.clamp(self.current_physical_aabb()) - self.input.start_pos
    }

    pub fn bounded_logical_delta(&self) -> Point {
        self.bounded_physical_delta().unscale(self.scale_factor)
    }
}

impl Event<DragEnd> {
    pub fn physical_delta(&self) -> Point {
        self.mouse_position - self.input.start_pos
    }

    pub fn logical_delta(&self) -> Point {
        self.physical_delta().unscale(self.scale_factor)
    }

    pub fn bounded_physical_delta(&self) -> Point {
        self.mouse_position.clamp(self.current_physical_aabb()) - self.input.start_pos
    }

    pub fn bounded_logical_delta(&self) -> Point {
        self.bounded_physical_delta().unscale(self.scale_factor)
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub(crate) struct MouseButtonsHeld {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
    pub aux1: bool,
    pub aux2: bool,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct ModifiersHeld {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
}

pub(crate) struct EventCache {
    pub focus: u64,
    pub keys_held: HashSet<Key>,
    pub modifiers_held: ModifiersHeld,
    pub mouse_buttons_held: MouseButtonsHeld,
    pub mouse_over: Option<u64>,
    pub mouse_position: Point,
    // Used to detect double clicks
    pub last_mouse_click: Instant,
    pub last_mouse_click_position: Point,
    // This is used as the start of the drag position, even if we haven't decided to start dragging
    pub drag_started: Option<Point>,
    // This is used as the indicator of whether a drag is actually ongoing
    pub drag_button: Option<MouseButton>,
    pub drag_target: Option<u64>,
    pub scale_factor: f32,
    pub drag_data: Vec<Data>,
}

impl std::fmt::Debug for EventCache {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("EventCache")
            .field("focus", &self.focus)
            .field("keys_held", &self.keys_held)
            .field("modifiers_held", &self.modifiers_held)
            .field("mouse_buttons_held", &self.mouse_buttons_held)
            .field("mouse_over", &self.mouse_over)
            .field("mouse_position", &self.mouse_position)
            .field("drag_started", &self.drag_started)
            .field("drag_button", &self.drag_button)
            .field("drag_target", &self.drag_target)
            .field("scale_factor", &self.scale_factor)
            .field("drag_data", &self.drag_data)
            .finish()
    }
}

impl EventCache {
    pub fn new(scale_factor: f32) -> Self {
        Self {
            focus: 0,
            keys_held: Default::default(),
            modifiers_held: Default::default(),
            mouse_buttons_held: Default::default(),
            mouse_over: None,
            mouse_position: Default::default(),
            last_mouse_click: Instant::now(),
            last_mouse_click_position: Default::default(),
            drag_button: None,
            drag_started: None,
            drag_target: None,
            drag_data: vec![],
            scale_factor,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.modifiers_held = Default::default();
        self.mouse_buttons_held = Default::default();
        self.mouse_over = None;
        self.drag_button = None;
        self.drag_started = None;
        self.drag_target = None;
        self.drag_data = vec![];
    }

    pub(crate) fn key_down(&mut self, key: Key) {
        match key {
            Key::LCtrl => self.modifiers_held.ctrl = true,
            Key::LShift => self.modifiers_held.shift = true,
            Key::LAlt => self.modifiers_held.alt = true,
            Key::LMeta => self.modifiers_held.meta = true,
            Key::RCtrl => self.modifiers_held.ctrl = true,
            Key::RShift => self.modifiers_held.shift = true,
            Key::RAlt => self.modifiers_held.alt = true,
            Key::RMeta => self.modifiers_held.meta = true,
            _ => {
                self.keys_held.insert(key);
            }
        }
    }

    pub(crate) fn key_up(&mut self, key: Key) {
        match key {
            Key::LCtrl => self.modifiers_held.ctrl = false,
            Key::LShift => self.modifiers_held.shift = false,
            Key::LAlt => self.modifiers_held.alt = false,
            Key::LMeta => self.modifiers_held.meta = false,
            Key::RCtrl => self.modifiers_held.ctrl = false,
            Key::RShift => self.modifiers_held.shift = false,
            Key::RAlt => self.modifiers_held.alt = false,
            Key::RMeta => self.modifiers_held.meta = false,
            _ => {
                self.keys_held.remove(&key);
            }
        }
    }

    pub(crate) fn key_held(&self, key: Key) -> bool {
        match key {
            Key::LCtrl => self.modifiers_held.ctrl,
            Key::LShift => self.modifiers_held.shift,
            Key::LAlt => self.modifiers_held.alt,
            Key::LMeta => self.modifiers_held.meta,
            Key::RCtrl => self.modifiers_held.ctrl,
            Key::RShift => self.modifiers_held.shift,
            Key::RAlt => self.modifiers_held.alt,
            Key::RMeta => self.modifiers_held.meta,
            _ => self.keys_held.contains(&key),
        }
    }

    pub(crate) fn mouse_down(&mut self, b: MouseButton) {
        match b {
            MouseButton::Left => self.mouse_buttons_held.left = true,
            MouseButton::Right => self.mouse_buttons_held.right = true,
            MouseButton::Middle => self.mouse_buttons_held.middle = true,
            MouseButton::Aux1 => self.mouse_buttons_held.aux1 = true,
            MouseButton::Aux2 => self.mouse_buttons_held.aux2 = true,
        }
    }

    pub(crate) fn mouse_up(&mut self, b: MouseButton) {
        match b {
            MouseButton::Left => self.mouse_buttons_held.left = false,
            MouseButton::Right => self.mouse_buttons_held.right = false,
            MouseButton::Middle => self.mouse_buttons_held.middle = false,
            MouseButton::Aux1 => self.mouse_buttons_held.aux1 = false,
            MouseButton::Aux2 => self.mouse_buttons_held.aux2 = false,
        }
    }

    pub(crate) fn is_mouse_button_held(&self, b: MouseButton) -> bool {
        match b {
            MouseButton::Left => self.mouse_buttons_held.left,
            MouseButton::Right => self.mouse_buttons_held.right,
            MouseButton::Middle => self.mouse_buttons_held.middle,
            MouseButton::Aux1 => self.mouse_buttons_held.aux1,
            MouseButton::Aux2 => self.mouse_buttons_held.aux2,
        }
    }

    pub(crate) fn mouse_button_held(&self) -> Option<MouseButton> {
        if self.mouse_buttons_held.left {
            Some(MouseButton::Left)
        } else if self.mouse_buttons_held.right {
            Some(MouseButton::Right)
        } else if self.mouse_buttons_held.middle {
            Some(MouseButton::Middle)
        } else if self.mouse_buttons_held.aux1 {
            Some(MouseButton::Aux1)
        } else if self.mouse_buttons_held.aux2 {
            Some(MouseButton::Aux2)
        } else {
            None
        }
    }
}
