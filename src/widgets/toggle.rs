use std::fmt;
use std::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, Message, RenderContext};
use crate::event;
use crate::render::{
    renderables::shape::{self, Shape},
    Renderable,
};
use crate::style::Styled;
use lemna_macros::{component, state_component_impl};

// TODO Make a tooltip
// TODO Font icons

#[derive(Debug, Default)]
struct ToggleState {
    pressed: bool,
}

#[component(State = "ToggleState", Styled, Internal)]
pub struct Toggle {
    active: bool,
    on_change: Option<Box<dyn Fn(bool) -> Message + Send + Sync>>,
}

impl fmt::Debug for Toggle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Toggle")
            .field("active", &self.active)
            .finish()
    }
}

impl Toggle {
    pub fn new(active: bool) -> Self {
        Self {
            active,
            on_change: None,
            state: Some(ToggleState::default()),
            dirty: true,
            class: Default::default(),
            style_overrides: Default::default(),
        }
    }

    pub fn on_change(mut self, change_fn: Box<dyn Fn(bool) -> Message + Send + Sync>) -> Self {
        self.on_change = Some(change_fn);
        self
    }
}

#[state_component_impl(ToggleState)]
impl Component for Toggle {
    fn on_mouse_leave(&mut self, _event: &mut event::Event<event::MouseLeave>) {
        self.state_mut().pressed = false;
    }

    fn on_mouse_down(&mut self, _event: &mut event::Event<event::MouseDown>) {
        self.state_mut().pressed = true;
    }

    fn on_mouse_up(&mut self, _event: &mut event::Event<event::MouseUp>) {
        self.state_mut().pressed = false;
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

        let background_color: Color = self.style_val("background_color").into();
        let active_color: Color = self.style_val("active_color").into();
        let border_color: Color = self.style_val("border_color").into();
        let highlight_color: Color = self.style_val("highlight_color").into();
        let border_width: f32 = self.style_val("border_width").unwrap().f32();

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

        if border_width > 0.0 {
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
                highlight_color
            } else if self.active {
                active_color
            } else {
                background_color
            },
            border_color,
            border_width * 0.5,
            0.0,
            &mut context.buffer_caches.shape_cache.write().unwrap(),
            context.prev_state.as_ref().and_then(|v| match v.get(0) {
                Some(Renderable::Shape(r)) => Some(r.buffer_id),
                _ => None,
            }),
        ))])
    }
}
