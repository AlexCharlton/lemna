use std::fmt;
use std::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, Message, RenderContext};
use crate::event;
use crate::render::{
    renderables::shape::{self, Shape},
    Renderable,
};
use lemna_macros::{state_component, state_component_impl};

// TODO Make a tooltip
// TODO Font icons

#[derive(Debug, Default)]
struct ToggleState {
    pressed: bool,
}

#[derive(Debug, Clone)]
pub struct ToggleStyle {
    pub background_color: Color,
    pub highlight_color: Color,
    pub active_color: Color,
    pub border_color: Color,
    pub border_width: f32,
}

impl Default for ToggleStyle {
    fn default() -> Self {
        Self {
            background_color: Color::LIGHT_GREY,
            highlight_color: Color::DARK_GREY,
            active_color: Color::MID_GREY,
            border_color: Color::BLACK,
            border_width: 2.0,
        }
    }
}

#[state_component(ToggleState)]
pub struct Toggle {
    active: bool,
    style: ToggleStyle,
    on_change: Option<Box<dyn Fn(bool) -> Message + Send + Sync>>,
}

impl fmt::Debug for Toggle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Toggle")
            .field("active", &self.active)
            .field("style", &self.style)
            .finish()
    }
}

impl Toggle {
    pub fn new(active: bool, style: ToggleStyle) -> Self {
        Self {
            active,
            style,
            on_change: None,
            state: Some(ToggleState::default()),
        }
    }

    pub fn on_change(mut self, change_fn: Box<dyn Fn(bool) -> Message + Send + Sync>) -> Self {
        self.on_change = Some(change_fn);
        self
    }
}

#[state_component_impl(ToggleState)]
impl Component for Toggle {
    fn on_mouse_leave(&mut self, event: &mut event::Event<event::MouseLeave>) {
        self.state_mut().pressed = false;
        event.dirty();
    }

    fn on_mouse_down(&mut self, event: &mut event::Event<event::MouseDown>) {
        self.state_mut().pressed = true;
        event.dirty();
    }

    fn on_mouse_up(&mut self, event: &mut event::Event<event::MouseUp>) {
        self.state_mut().pressed = false;
        event.dirty();
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) {
        if let Some(f) = &self.on_change {
            event.emit(f(!self.active));
        }
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.active.hash(hasher);
        self.state_ref().pressed.hash(hasher);
    }

    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        use lyon::tessellation::math as lyon_math;
        use lyon::tessellation::{self, basic_shapes};

        let mut geometry = shape::ShapeGeometry::new();
        let center = lyon_math::point(context.aabb.width() / 2.0, context.aabb.height() / 2.0);

        let fill_count = basic_shapes::fill_circle(
            center,
            context.aabb.width() / 2.0,
            &tessellation::FillOptions::tolerance(shape::TOLERANCE),
            &mut tessellation::BuffersBuilder::new(
                &mut geometry,
                shape::Vertex::basic_vertex_constructor,
            ),
        )
        .unwrap()
        .indices;

        if self.style.border_width > 0.0 {
            basic_shapes::stroke_circle(
                center,
                context.aabb.width() / 2.0,
                &tessellation::StrokeOptions::tolerance(shape::TOLERANCE).dont_apply_line_width(),
                &mut tessellation::BuffersBuilder::new(
                    &mut geometry,
                    shape::Vertex::stroke_vertex_constructor,
                ),
            )
            .unwrap();
        }

        Some(vec![Renderable::Shape(Shape::new(
            geometry,
            fill_count,
            if self.state_ref().pressed {
                self.style.highlight_color
            } else if self.active {
                self.style.active_color
            } else {
                self.style.background_color
            },
            self.style.border_color,
            self.style.border_width * 0.5,
            0.0,
            &mut context.buffer_caches.shape_cache.write().unwrap(),
            context.prev_state.as_ref().and_then(|v| match v.get(0) {
                Some(Renderable::Shape(r)) => Some(r.buffer_id),
                _ => None,
            }),
        ))])
    }
}
