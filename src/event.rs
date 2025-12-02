//! Types that relate to event handling.
extern crate alloc;

use crate::time::Instant;
use alloc::{string::String, vec, vec::Vec};
use core::fmt;

use hashbrown::HashSet;

use super::base_types::*;
use super::input::{Key, MouseButton};
use crate::{Message, Node, NodeId};

/// How much time (ms) can elapse between clicks before it's no longer considered a double click.
pub const DOUBLE_CLICK_INTERVAL_MS: i64 = 500; // ms
/// How much mouse travel (px) is allowed before it's no longer considered a double click.
pub const DOUBLE_CLICK_MAX_DIST: f32 = 10.0; // px
/// How much distance (px) is required before we start a drag event.
pub const DRAG_THRESHOLD: f32 = 15.0; // px
/// How much mouse travel (px) is allowed until we'll no longer send a click event.
///
/// Note that this is longer than [`DRAG_THRESHOLD`].
pub const DRAG_CLICK_MAX_DIST: f32 = 30.0; // px

/// The contextual data that is sent to a [`Component`][crate::Component]'s `on_EVENT` methods.
pub struct Event<T: EventInput> {
    /// The event-specific [`EventInput`]
    pub input: T,
    pub(crate) bubbles: bool,
    // Does the node need to be re-computed?
    // Implies render_dirty
    pub(crate) dirty: bool,
    // Does the node need to be re-rendered?
    pub(crate) render_dirty: bool,
    pub(crate) mouse_position: Point,
    /// What keyboard modifiers (Shift, Alt, Ctr, Meta) were held when this event was fired.
    pub modifiers_held: ModifiersHeld,
    pub(crate) current_node_id: Option<NodeId>,
    // In physical coordinates
    pub(crate) current_aabb: Option<Rect>,
    // In logical coordinates
    pub(crate) current_inner_scale: Option<Scale>,
    pub(crate) over_child_n: Option<usize>,
    pub(crate) over_subchild_n: Option<usize>,
    pub(crate) target: Option<NodeId>,
    pub(crate) focus: Option<NodeId>,
    pub(crate) focus_stack: Vec<NodeId>,
    pub(crate) scale_factor: f32,
    pub(crate) messages: Vec<Message>,
    pub(crate) signals: Signaller,
    /// Stack of nodes that the event passed through. If the event is targeted, then this is the nodes on the way to the target. If the event is a mouse event, the this is the nodes through which the mouse passed (and in particular, the nodes on the way to whatever stopped the event from bubbling).
    pub(crate) stack: Vec<NodeId>,
}

impl<T: EventInput> fmt::Debug for Event<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
            .field("focus_stack", &self.focus_stack)
            .field("scale_factor", &self.scale_factor)
            .field("stack", &self.stack)
            .field("signals", &self.signals)
            .finish()
    }
}

/// Types that can be an [`Event::input`].
pub trait EventInput: fmt::Debug {}

/// [`EventInput`] type for focus events.
#[derive(Debug)]
pub struct Focus;
impl EventInput for Focus {}

/// [`EventInput`] type for blur events.
#[derive(Debug)]
pub struct Blur;
impl EventInput for Blur {}

/// [`EventInput`] type for tick events.
#[derive(Debug)]
pub struct Tick;
impl EventInput for Tick {}

/// [`EventInput`] type for mouse motion events.
#[derive(Debug)]
pub struct MouseMotion;
impl EventInput for MouseMotion {}

/// [`EventInput`] type for mouse down events.
#[derive(Debug)]
pub struct MouseDown(
    /// The [`MouseButton`] pressed.
    pub MouseButton,
);
impl EventInput for MouseDown {}

/// [`EventInput`] type for mouse up events.
#[derive(Debug)]
pub struct MouseUp(
    /// The [`MouseButton`] released.
    pub MouseButton,
);
impl EventInput for MouseUp {}

/// [`EventInput`] type for mouse enter events.
#[derive(Debug)]
pub struct MouseEnter;
impl EventInput for MouseEnter {}

/// [`EventInput`] type for mouse leave events.
#[derive(Debug)]
pub struct MouseLeave;
impl EventInput for MouseLeave {}

/// [`EventInput`] type for mouse click events.
#[derive(Debug)]
pub struct Click(
    /// The [`MouseButton`] clicked.
    pub MouseButton,
);
impl EventInput for Click {}

/// [`EventInput`] type for mouse double click events.
#[derive(Debug)]
pub struct DoubleClick(
    ///  The [`MouseButton`] clicked.
    pub MouseButton,
);
impl EventInput for DoubleClick {}

/// [`EventInput`] type for key down events.
#[derive(Debug)]
pub struct KeyDown(
    /// The [`Key`] pressed.
    pub Key,
);
impl EventInput for KeyDown {}

/// [`EventInput`] type for key up events.
#[derive(Debug)]
pub struct KeyUp(
    /// The [`Key`] released.
    pub Key,
);
impl EventInput for KeyUp {}

/// [`EventInput`] type for key press (up and down) events.
#[derive(Debug)]
pub struct KeyPress(
    /// The [`Key`] pressed.
    pub Key,
);
impl EventInput for KeyPress {}

/// [`EventInput`] type for text entry events.
#[derive(Debug)]
pub struct TextEntry(
    /// The string entered.
    pub String,
);
impl EventInput for TextEntry {}

/// [`EventInput`] type for scroll events.
#[derive(Debug, Copy, Clone)]
pub struct Scroll {
    /// Amount scrolled along the x axis.
    pub x: f32,
    /// Amount scrolled along the y axis.
    pub y: f32,
}
impl EventInput for Scroll {}

/// [`EventInput`] type for drag events.
#[derive(Debug, Copy, Clone)]
pub struct Drag {
    /// The mouse button that initiated the drag.
    pub button: MouseButton,
    /// The logical start position of the drag.
    pub start_pos: Point,
}
impl EventInput for Drag {}

/// [`EventInput`] type for drag start events.
#[derive(Debug)]
pub struct DragStart(
    /// The [`MouseButton`] that initiated the drag.
    pub MouseButton,
);
impl EventInput for DragStart {}

/// [`EventInput`] type for drag end events.
#[derive(Debug, Copy, Clone)]
pub struct DragEnd {
    /// The mouse button that initiated the drag.
    pub button: MouseButton,
    /// The logical start position of the drag.
    pub start_pos: Point,
}
impl EventInput for DragEnd {}

/// [`EventInput`] type for drag target events.
#[derive(Debug)]
pub struct DragTarget;
impl EventInput for DragTarget {}

/// [`EventInput`] type for drag enter events.
#[derive(Debug)]
pub struct DragEnter(
    /// The [`Data`] being dragged.
    pub Vec<Data>,
);
impl EventInput for DragEnter {}

/// [`EventInput`] type for drag leave events.
#[derive(Debug)]
pub struct DragLeave;
impl EventInput for DragLeave {}

/// [`EventInput`] type for drag drop events.
#[derive(Debug)]
pub struct DragDrop(
    /// The [`Data`] being dragged.
    pub Data,
);
impl EventInput for DragDrop {}

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
    pub(crate) fn new(input: T, event_cache: &EventCache, focus: NodeId) -> Self {
        Self {
            input,
            bubbles: true,
            dirty: false,
            render_dirty: false,
            modifiers_held: event_cache.modifiers_held,
            mouse_position: event_cache.mouse_position,
            focus: Some(focus),
            focus_stack: vec![],
            target: None,
            current_node_id: None,
            current_aabb: None,
            current_inner_scale: None,
            over_child_n: None,
            over_subchild_n: None,
            scale_factor: event_cache.scale_factor,
            messages: vec![],
            signals: Signaller::default(),
            stack: vec![],
        }
    }

    /// Set the current Node to be "focused".
    /// This will cause it to receive [`Blur`], [`KeyDown`], [`KeyUp`], [`KeyPress`], [`TextEntry`], [`Drag`], and [`DragEnd`] events.
    ///
    /// Implies `stop_bubbling`. Note that any other Nodes may also request focus.
    pub fn focus(&mut self) {
        self.focus = self.current_node_id;
        self.bubbles = false;
    }

    /// Remove focus from this Node, if applicable.
    pub fn blur(&mut self) {
        self.focus = None;
    }

    /// Prevent this Event from being sent to one of the ancestor Nodes of the current one.
    pub fn stop_bubbling(&mut self) {
        self.bubbles = false;
    }

    pub(crate) fn dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn set_focus_stack(&mut self, focus_stack: Vec<NodeId>) {
        self.focus_stack = focus_stack;
    }

    /// Mark the event as requiring a re-render (but not necessarily a full recompute)
    pub fn render_dirty(&mut self) {
        self.render_dirty = true;
    }

    /// Send the [`Message`] to the ancestor Nodes of the current one. They will receive it through the [`Component#update`][crate::Component#method.update] method.
    pub fn emit(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    /// Return the AABB of the current Node, in physical coordinates.
    pub fn current_physical_aabb(&self) -> Rect {
        self.current_aabb.unwrap()
    }

    /// Return the AABB of the current Node, in logical coordinates.
    pub fn current_logical_aabb(&self) -> Rect {
        self.current_aabb.unwrap().unscale(self.scale_factor)
    }

    /// For scrollable [`Component`s][crate::Component], returns the size of the children of the current Node, in logical coordinates.
    pub fn current_logical_inner_scale(&self) -> Option<Scale> {
        self.current_inner_scale
    }

    /// For scrollable [`Component`s][crate::Component], returns the size of the children of the current Node, in physical coordinates.
    pub fn current_physical_inner_scale(&self) -> Option<Scale> {
        self.current_inner_scale.map(|s| s.scale(self.scale_factor))
    }

    /// The current absolutely mouse position, in physical coordinates.
    pub fn physical_mouse_position(&self) -> Point {
        self.mouse_position
    }

    /// The current absolutely mouse position, in logical coordinates.
    pub fn logical_mouse_position(&self) -> Point {
        self.mouse_position.unscale(self.scale_factor)
    }

    /// The current mouse position relative to this Node's AABB, in physical coordinates.
    pub fn relative_physical_position(&self) -> Point {
        let pos = self.current_aabb.unwrap().pos;
        self.mouse_position - Point { x: pos.x, y: pos.y }
    }

    /// The current mouse position relative to this Node's AABB, in logical coordinates.
    pub fn relative_logical_position(&self) -> Point {
        let pos = self.current_aabb.unwrap().pos;
        (self.mouse_position - Point { x: pos.x, y: pos.y }).unscale(self.scale_factor)
    }

    /// Returns which child of this Node the mouse is over, if any.
    pub fn over_child_n(&self) -> Option<usize> {
        self.over_child_n
    }

    /// Returns which child of the child of this Node the mouse is over, if any.
    pub fn over_subchild_n(&self) -> Option<usize> {
        self.over_subchild_n
    }

    /// Signal that the given child (or self if the child vector is empty) should be focused.
    pub fn focus_child(&mut self, child: Vec<usize>) {
        self.signals.focus_child(child);
    }

    /// Signal that the given reference should be focused.
    pub fn focus_ref<S: Into<String>>(&mut self, target: S) {
        self.signals.focus_ref(target)
    }

    /// Signal that the given child (or self if the child vector is empty) should be scrolled to.
    pub fn scroll_to_child(&mut self, child: Vec<usize>) {
        self.signals.scroll_to_child(child)
    }

    /// Signal that the given reference should be scrolled to.
    pub fn scroll_to_ref<S: Into<String>>(&mut self, target: S) {
        self.signals.scroll_to_ref(target)
    }

    pub(crate) fn resolve_signal_children(&mut self, current_node: &Node) {
        self.signals.resolve_children(current_node);
    }
}

impl Event<Drag> {
    /// The distance dragged, in physical coordinates.
    pub fn physical_delta(&self) -> Point {
        self.mouse_position - self.input.start_pos
    }

    /// The distance dragged, in logical coordinates.
    pub fn logical_delta(&self) -> Point {
        self.physical_delta().unscale(self.scale_factor)
    }

    /// The distance dragged, but clamped to the current Node's AABB, in physical coordinates.
    pub fn bounded_physical_delta(&self) -> Point {
        self.mouse_position.clamp(self.current_physical_aabb()) - self.input.start_pos
    }

    /// The distance dragged, but clamped to the current Node's AABB, in logical coordinates.
    pub fn bounded_logical_delta(&self) -> Point {
        self.bounded_physical_delta().unscale(self.scale_factor)
    }
}

impl Event<DragEnd> {
    /// The distance dragged, in physical coordinates.
    pub fn physical_delta(&self) -> Point {
        self.mouse_position - self.input.start_pos
    }

    /// The distance dragged, in logical coordinates.
    pub fn logical_delta(&self) -> Point {
        self.physical_delta().unscale(self.scale_factor)
    }

    /// The distance dragged, but clamped to the current Node's AABB, in physical coordinates.
    pub fn bounded_physical_delta(&self) -> Point {
        self.mouse_position.clamp(self.current_physical_aabb()) - self.input.start_pos
    }

    /// The distance dragged, but clamped to the current Node's AABB, in logical coordinates.
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

/// The keyboard modifiers that are held down while an [`Event`] is fired.
#[derive(Debug, Default, Copy, Clone)]
pub struct ModifiersHeld {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
}

//-------------------------------------------------------
// MARK: EventCache
//-------------------------------------------------------

/// Points are all logical positions.
pub(crate) struct EventCache {
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

impl fmt::Debug for EventCache {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EventCache")
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

//---------------------------------------------
// MARK: Signaller
//---------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    Ref(String),
    // The child node ID gets resolved when event handling
    Child(Vec<usize>, Option<NodeId>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Signal {
    Focus(Target),
    ScrollTo(Target),
}

impl Signal {
    fn target_mut(&mut self) -> &mut Target {
        match self {
            Signal::Focus(target) | Signal::ScrollTo(target) => target,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Signaller {
    pub(crate) signals: Vec<Signal>,
}

impl Signaller {
    fn focus_ref<S: Into<String>>(&mut self, ref_id: S) {
        self.signals.push(Signal::Focus(Target::Ref(ref_id.into())));
    }

    fn focus_child(&mut self, child_index: Vec<usize>) {
        self.signals
            .push(Signal::Focus(Target::Child(child_index, None)));
    }

    fn scroll_to_ref<S: Into<String>>(&mut self, ref_id: S) {
        self.signals
            .push(Signal::ScrollTo(Target::Ref(ref_id.into())));
    }

    fn scroll_to_child(&mut self, child_index: Vec<usize>) {
        self.signals
            .push(Signal::ScrollTo(Target::Child(child_index, None)));
    }

    fn resolve_children(&mut self, current_node: &Node) {
        for signal in self.signals.iter_mut() {
            let mut node = current_node;
            let mut found_child = false;
            if let Signal::ScrollTo(Target::Child(child_index, None))
            | Signal::Focus(Target::Child(child_index, None)) = signal
            {
                found_child = true;
                for i in child_index.iter() {
                    if let Some(child) = node.children.get(*i) {
                        node = child;
                    } else {
                        found_child = false;
                        break;
                    }
                }
            }
            if found_child {
                match signal.target_mut() {
                    Target::Child(_, id) => {
                        *id = Some(node.id);
                    }
                    _ => {}
                }
            }
        }
    }
}
