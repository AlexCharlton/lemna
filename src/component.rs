use std::any::Any;
use std::fmt;
use std::sync::{Arc, RwLock};

use ahash::AHasher;

use crate::base_types::*;
use crate::event::{self, Event};
use crate::font_cache::FontCache;
use crate::layout::*;
use crate::node::Node;
use crate::render::{BufferCaches, Renderable};

pub type Message = Box<dyn Any>;
pub type State = Box<dyn Any>;
// AHasher makes it easier to make reproducible hashes
pub type ComponentHasher = AHasher;

#[macro_export]
macro_rules! msg {
    ($e:expr) => {
        Box::new($e)
    };
}

pub struct RenderContext {
    pub aabb: AABB,
    pub inner_scale: Option<Scale>,
    pub buffer_caches: BufferCaches,
    pub prev_state: Option<Vec<Renderable>>,
    pub font_cache: Arc<RwLock<FontCache>>,
    pub scale_factor: f32,
}

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

    fn replace_state(&mut self, _other: State) {}

    fn take_state(&mut self) -> Option<State> {
        None
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
