use std::any::Any;
use std::fmt;

use ahash::AHasher;

use crate::base_types::*;
use crate::event::{self, Event};
use crate::font_cache::FontCache;
use crate::layout::*;
use crate::node::Node;
use crate::render::{Caches, Renderable};

/// A `Box<dyn Any>` type, used to convey information from a [`Component`] to one of its parent nodes.
pub type Message = Box<dyn Any>;
#[doc(hidden)]
// Only used by `replace_state` and `take_state`, which are not meant to be implemented by the user.
pub type State = Box<dyn Any>;
/// A concrete implementor of [`std::hash::Hasher`], used by [`Component#props_hash`][Component#props_hash] and [`#render_hash`][Component#render_hash].
///
/// [`AHasher`] is used, since it makes it easier to create reproducible hashes.
pub type ComponentHasher = AHasher;

#[macro_export]
macro_rules! msg {
    ($e:expr) => {
        Box::new($e)
    };
}

/// Passed to [`Component#render`][Component#render], with context required for rendering.
pub struct RenderContext {
    /// The `AABB` that contains the given [`Component`] instance.
    pub aabb: AABB,
    pub inner_scale: Option<Scale>,
    /// The caches used by the renderer.
    pub caches: Caches,
    /// The value previously returned by [`Component#render`][Component#render] of the given instance.
    pub prev_state: Option<Vec<Renderable>>,
    /// The scale factor of the current monitor. Renderables should be scaled by this value.
    pub scale_factor: f32,
}

/// The primary interface of Lemna. Components are the -- optionally stateful -- elements that are drawn on a window that a user interacts with.
pub trait Component: fmt::Debug {
    fn init(&mut self) {}

    fn new_props(&mut self) {}

    fn update(&mut self, msg: Message) -> Vec<Message> {
        vec![msg]
    }

    fn view(&self) -> Option<Node> {
        None
    }

    fn render(&mut self, _context: RenderContext) -> Option<Vec<Renderable>> {
        None
    }

    #[doc(hidden)]
    fn replace_state(&mut self, _other: State) {}

    #[doc(hidden)]
    fn take_state(&mut self) -> Option<State> {
        None
    }

    #[doc(hidden)]
    fn is_dirty(&mut self) -> bool {
        false
    }

    fn register(&mut self) -> Vec<event::Register> {
        vec![]
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.props_hash(hasher);
    }

    fn props_hash(&self, _hasher: &mut ComponentHasher) {}

    fn is_mouse_over(&self, mouse_position: Point, aabb: AABB) -> bool {
        aabb.is_under(mouse_position)
    }

    fn is_mouse_maybe_over(&self, mouse_position: Point, aabb: AABB) -> bool {
        aabb.is_under(mouse_position)
    }

    fn fill_bounds(
        &mut self,
        _width: Option<f32>,
        _height: Option<f32>,
        _max_width: Option<f32>,
        _max_height: Option<f32>,
        _font_cache: &FontCache,
        _scale_factor: f32,
    ) -> (Option<f32>, Option<f32>) {
        (None, None)
    }

    /// Give component full control over its own AABB
    fn full_control(&self) -> bool {
        false
    }

    /// Called when the child of a full control Node
    fn focus(&self) -> Option<Point> {
        None
    }

    fn set_aabb(
        &mut self,
        _aabb: &mut AABB,
        _parent_aabb: AABB,
        _children: Vec<(&mut AABB, Option<Scale>, Option<Point>)>,
        _frame: AABB,
        _scale_factor: f32,
    ) {
    }

    // Scrollable containers
    fn scroll_position(&self) -> Option<ScrollPosition> {
        None
    }

    fn frame_bounds(&self, aabb: AABB, _inner_scale: Option<Scale>) -> AABB {
        aabb
    }

    // Event handlers
    fn on_click(&mut self, _event: &mut Event<event::Click>) {}
    fn on_double_click(&mut self, _event: &mut Event<event::DoubleClick>) {}
    fn on_mouse_down(&mut self, _event: &mut Event<event::MouseDown>) {}
    fn on_mouse_up(&mut self, _event: &mut Event<event::MouseUp>) {}
    fn on_mouse_enter(&mut self, _event: &mut Event<event::MouseEnter>) {}
    fn on_mouse_leave(&mut self, _event: &mut Event<event::MouseLeave>) {}
    fn on_mouse_motion(&mut self, _event: &mut Event<event::MouseMotion>) {}
    fn on_focus(&mut self, _event: &mut Event<event::Focus>) {}
    fn on_blur(&mut self, _event: &mut Event<event::Blur>) {}
    fn on_tick(&mut self, _event: &mut Event<event::Tick>) {}
    fn on_key_down(&mut self, _event: &mut Event<event::KeyDown>) {}
    fn on_key_up(&mut self, _event: &mut Event<event::KeyUp>) {}
    fn on_key_press(&mut self, _event: &mut Event<event::KeyPress>) {}
    fn on_text_entry(&mut self, _event: &mut Event<event::TextEntry>) {}
    fn on_scroll(&mut self, _event: &mut Event<event::Scroll>) {}
    fn on_drag(&mut self, _event: &mut Event<event::Drag>) {}
    fn on_drag_start(&mut self, _event: &mut Event<event::DragStart>) {}
    fn on_drag_end(&mut self, _event: &mut Event<event::DragEnd>) {}
    fn on_drag_target(&mut self, _event: &mut Event<event::DragTarget>) {}
    fn on_drag_enter(&mut self, _event: &mut Event<event::DragEnter>) {}
    fn on_drag_leave(&mut self, _event: &mut Event<event::DragLeave>) {}
    fn on_drag_drop(&mut self, _event: &mut Event<event::DragDrop>) {}
    fn on_menu_select(&mut self, _event: &mut Event<event::MenuSelect>) {}
}
