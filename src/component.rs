use std::any::Any;
use std::fmt;

use ahash::AHasher;

use crate::base_types::*;
use crate::event::{self, Event};
use crate::font_cache::FontCache;
use crate::layout::*;
use crate::node::Node;
use crate::render::Renderer;

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

pub trait App<R>: Component<R> + Send + Sync
where
    R: Renderer + fmt::Debug,
{
    fn new() -> Self;
}

pub struct RenderContext<'a, R>
where
    R: Renderer,
{
    pub aabb: &'a AABB,
    pub inner_scale: Option<Scale>,
    pub renderer: &'a mut R,
    pub prev_state: Option<Vec<R::Renderable>>,
    pub font_cache: &'a FontCache,
    pub scale_factor: f32,
}

pub trait Component<R>: fmt::Debug
where
    R: Renderer + fmt::Debug,
{
    fn init(&mut self) {}

    fn new_props(&mut self) {}

    fn update(&mut self, msg: Message) -> Vec<Message> {
        vec![msg]
    }

    fn view(&self) -> Option<Node<R>> {
        None
    }

    fn render<'a>(&mut self, _context: RenderContext<'a, R>) -> Option<Vec<R::Renderable>> {
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
    fn on_click(&mut self, _event: &mut Event<event::Click>) -> Vec<Message> {
        vec![]
    }
    fn on_double_click(&mut self, _event: &mut Event<event::DoubleClick>) -> Vec<Message> {
        vec![]
    }
    fn on_mouse_down(&mut self, _event: &mut Event<event::MouseDown>) -> Vec<Message> {
        vec![]
    }
    fn on_mouse_up(&mut self, _event: &mut Event<event::MouseUp>) -> Vec<Message> {
        vec![]
    }
    fn on_mouse_enter(&mut self, _event: &mut Event<event::MouseEnter>) -> Vec<Message> {
        vec![]
    }
    fn on_mouse_leave(&mut self, _event: &mut Event<event::MouseLeave>) -> Vec<Message> {
        vec![]
    }
    fn on_mouse_motion(&mut self, _event: &mut Event<event::MouseMotion>) -> Vec<Message> {
        vec![]
    }
    fn on_focus(&mut self, _event: &mut Event<event::Focus>) -> Vec<Message> {
        vec![]
    }
    fn on_blur(&mut self, _event: &mut Event<event::Blur>) -> Vec<Message> {
        vec![]
    }
    fn on_tick(&mut self, _event: &mut Event<event::Tick>) -> Vec<Message> {
        vec![]
    }
    fn on_key_down(&mut self, _event: &mut Event<event::KeyDown>) -> Vec<Message> {
        vec![]
    }
    fn on_key_up(&mut self, _event: &mut Event<event::KeyUp>) -> Vec<Message> {
        vec![]
    }
    fn on_key_press(&mut self, _event: &mut Event<event::KeyPress>) -> Vec<Message> {
        vec![]
    }
    fn on_text_entry(&mut self, _event: &mut Event<event::TextEntry>) -> Vec<Message> {
        vec![]
    }
    fn on_scroll(&mut self, _event: &mut Event<event::Scroll>) -> Vec<Message> {
        vec![]
    }
    fn on_drag(&mut self, _event: &mut Event<event::Drag>) -> Vec<Message> {
        vec![]
    }
    fn on_drag_start(&mut self, _event: &mut Event<event::DragStart>) -> Vec<Message> {
        vec![]
    }
    fn on_drag_end(&mut self, _event: &mut Event<event::DragEnd>) -> Vec<Message> {
        vec![]
    }
    fn on_menu_select(&mut self, _event: &mut Event<event::MenuSelect>) -> Vec<Message> {
        vec![]
    }
}
