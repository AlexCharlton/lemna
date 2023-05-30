use std::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, Message, RenderContext};
use crate::event;
use crate::font_cache::HorizontalAlign;
use crate::layout::*;
use crate::render::{renderables::shape::Shape, Renderable};
use crate::style::{current_style, Styled};
use crate::{node, txt, Node};
use lemna_macros::{component, state_component_impl};

#[derive(Debug)]
enum SelectMessage {
    OpenClose,
    Close,
    Hover(usize),
    Select(usize),
}

//
// Select
// The top-level, public component
#[derive(Debug, Default)]
struct SelectState {
    open: bool,
    selected: usize,
    hovering: usize,
}

#[component(State = "SelectState", Styled, Internal)]
pub struct Select<M: Send + Sync>
where
    M: Send + Sync,
{
    pub selection: Vec<M>,
    pub selected: usize,
    on_change: Option<Box<dyn Fn(usize, &M) -> Message + Send + Sync>>,
}

impl<M: std::fmt::Debug + Send + Sync> std::fmt::Debug for Select<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("selection", &self.selection)
            .finish()
    }
}

impl<M: ToString + Send + Sync> Select<M> {
    pub fn new(selection: Vec<M>, selected: usize) -> Self {
        Self {
            selection,
            selected,
            on_change: None,
            class: Default::default(),
            style_overrides: Default::default(),
            state: Some(SelectState::default()),
        }
    }

    pub fn on_change(mut self, change_fn: Box<dyn Fn(usize, &M) -> Message + Send + Sync>) -> Self {
        self.on_change = Some(change_fn);
        self
    }
}

#[state_component_impl(SelectState)]
impl<M: 'static + std::fmt::Debug + Clone + ToString + std::fmt::Display + Send + Sync> Component
    for Select<M>
{
    fn view(&self) -> Option<Node> {
        let mut base =
            node!(super::Div::new(), lay!(direction: Direction::Column)).push(node!(SelectBox {
                selection: self.selection.get(self.state_ref().selected).cloned(),
                style_overrides: self.style_overrides.clone(),
                class: self.class.clone(),
            }));
        if self.state_ref().open {
            base = base.push(node!(
                SelectList {
                    selections: self.selection.clone(),
                    hovering: self.state_ref().hovering,
                    style_overrides: self.style_overrides.clone(),
                    class: self.class.clone(),
                },
                lay!(position_type: PositionType::Absolute, z_index_increment: 1000.0),
                1
            ));
        }
        Some(base)
    }

    fn props_hash(&self, hasher: &mut ComponentHasher) {
        self.selected.hash(hasher);
    }

    fn init(&mut self) {
        self.state_mut().selected = self.selected;
    }

    fn new_props(&mut self) {
        self.state_mut().selected = self.selected;
    }

    fn render_hash(&self, hasher: &mut ComponentHasher) {
        self.state_ref().open.hash(hasher)
    }

    fn update(&mut self, message: Message) -> Vec<Message> {
        let mut m: Vec<Message> = vec![];

        match message.downcast_ref::<SelectMessage>() {
            Some(SelectMessage::OpenClose) => {
                self.state_mut().hovering = self.state_ref().selected;
                self.state_mut().open = !self.state_ref().open;
            }
            Some(SelectMessage::Close) => self.state_mut().open = false,
            Some(SelectMessage::Select(i)) => {
                self.state_mut().selected = *i;
                if let Some(change_fn) = &self.on_change {
                    m.push(change_fn(*i, &self.selection[*i]))
                }
            }
            Some(SelectMessage::Hover(i)) => self.state_mut().hovering = *i,
            _ => panic!(),
        }
        m
    }
}

//
// SelectBox
// The base component you interact with. A button that shows selection state
#[component(Styled = "Select", Internal)]
#[derive(Debug)]
struct SelectBox<M> {
    selection: Option<M>,
}

impl<M: 'static + std::fmt::Debug + Clone + ToString> Component for SelectBox<M> {
    fn view(&self) -> Option<Node> {
        let padding: f64 = self.style_param("padding").unwrap().into();
        let radius: f32 = self.style_param("radius").unwrap().f32();
        let font_size: f32 = self.style_param("font_size").unwrap().f32();
        let background_color: Color = self.style_param("background_color").into();
        let border_color: Color = self.style_param("border_color").into();
        let caret_color: Color = self.style_param("caret_color").into();
        let border_width: f32 = self.style_param("border_width").unwrap().f32();

        let mut base = node!(
            super::RoundedRect {
                background_color: background_color,
                border_color: border_color,
                border_width: border_width,
                radius: (radius, radius, radius, radius),
            },
            lay!(
                size: size_pct!(100.0),
                padding: rect!(padding),
                cross_alignment: Alignment::Center,
                axis_alignment: Alignment::Center,
                direction: Direction::Row,
            )
        );
        if let Some(selection) = self.selection.as_ref() {
            base = base
                .push(node!(super::Text::new(txt!(selection.to_string()))
                    .style("size", self.style_param("font_size").unwrap())
                    .style("color", self.style_param("text_color").unwrap())
                    .style("h_alignment", HorizontalAlign::Center.into())
                    .maybe_style("font", self.style_param("font"))))
                .push(node!(
                    Caret { color: caret_color },
                    lay!(
                        size: size!(font_size / 2.0),
                        // TODO: Margin here is awkward
                        margin: rect!(Auto, padding)
                    )
                ))
        }
        Some(base)
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) {
        event.stop_bubbling();
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) {
        event.dirty();
        event.focus();
        event.stop_bubbling();
        event.emit(Box::new(SelectMessage::OpenClose));
    }

    fn on_blur(&mut self, event: &mut event::Event<event::Blur>) {
        event.dirty();
        event.emit(Box::new(SelectMessage::Close));
    }
}

#[derive(Debug)]
struct Caret {
    color: Color,
}

use lyon::path::Path;
use lyon::tessellation::math as lyon_math;
impl Component for Caret {
    fn render(&mut self, context: RenderContext) -> Option<Vec<Renderable>> {
        let scale = 1.0;

        let mut path_builder = Path::builder();
        let w = context.aabb.width();
        let h = context.aabb.height();
        let start = h / 2.0;
        path_builder.move_to(lyon_math::point(0.0, start));
        path_builder.line_to(lyon_math::point(w / 2.0, h));
        path_builder.line_to(lyon_math::point(w, start));

        let (geometry, _) = Shape::path_to_shape_geometry(path_builder.build(), false, true);

        Some(vec![Renderable::Shape(Shape::stroke(
            geometry,
            self.color,
            scale,
            0.0,
            &mut context.buffer_caches.shape_cache.write().unwrap(),
            context.prev_state.as_ref().and_then(|v| match v.get(0) {
                Some(Renderable::Shape(r)) => Some(r.buffer_id),
                _ => None,
            }),
        ))])
    }
}

//
// SelectList
// Visible after opening: The full selection list
#[derive(Debug)]
#[component(Styled = "Select", Internal)]
struct SelectList<M>
where
    M: Send + Sync,
{
    selections: Vec<M>,
    hovering: usize,
}

impl<M: 'static + std::fmt::Debug + Clone + ToString + Send + Sync> Component for SelectList<M> {
    fn view(&self) -> Option<Node> {
        let background_color: Color = self.style_param("background_color").into();

        let mut l = node!(
            super::Div::new().bg(background_color).scroll_y(),
            [direction: Column, cross_alignment: Stretch,]
        );
        for (i, s) in self.selections.iter().enumerate() {
            l = l.push(
                node!(SelectEntry {
                    selection: s.clone(),
                    id: i,
                    selected: i == self.hovering,
                    style_overrides: self.style_overrides.clone(),
                    class: self.class.clone(),
                })
                .key(i as u64),
            );
        }
        Some(l)
    }

    fn full_control(&self) -> bool {
        true
    }

    fn set_aabb(
        &mut self,
        aabb: &mut AABB,
        parent_aabb: AABB,
        mut children: Vec<(&mut AABB, Option<Scale>, Option<Point>)>,
        frame: AABB,
        scale_factor: f32,
    ) {
        if let Some((child_aabb, Some(inner_scale), _)) = children.first_mut() {
            let max_height: f32 = self.style_param("max_height").unwrap().f32();
            let bar_width: f32 = current_style().style("Scroll", "bar_width").unwrap().f32();
            // Set size based on list elements and max_height
            let mut h = inner_scale.height;
            let mut w = inner_scale.width;
            if h > max_height * scale_factor {
                h = max_height * scale_factor;
                w = inner_scale.width + bar_width * scale_factor;
            }

            // Shrink if there isn't enough room
            let room_above = parent_aabb.pos.y - frame.pos.y;
            let room_bellow = frame.bottom_right.y - parent_aabb.bottom_right.y;
            if h > room_bellow && h > room_above {
                h = room_bellow.max(room_above);
                w = inner_scale.width + bar_width * scale_factor;
            }

            aabb.set_scale_mut(w, h);
            child_aabb.set_scale_mut(w, h);
        }

        if aabb.bottom_right.y > frame.bottom_right.y {
            // Flip up if there isn't enough room underneath
            aabb.translate_mut(0.0, -parent_aabb.height() - aabb.height());
        }
    }
}

//
// SelectEntry
// An individual entry within a SelectList
#[component(Styled = "Select", Internal)]
#[derive(Debug)]
struct SelectEntry<M>
where
    M: Send + Sync,
{
    selection: M,
    id: usize,
    selected: bool,
}

impl<M: 'static + std::fmt::Debug + Clone + ToString + Send + Sync> Component for SelectEntry<M> {
    fn view(&self) -> Option<Node> {
        let padding: f64 = self.style_param("padding").unwrap().into();
        let highlight_color: Color = self.style_param("highlight_color").into();

        let mut div = super::Div::new();
        if self.selected {
            div = div.bg(highlight_color)
        }

        Some(
            node!(div, lay!(size: size_pct!(100.0), padding: rect!(padding))).push(node!(
                super::Text::new(txt!(self.selection.to_string()))
                    .style("size", self.style_param("font_size").unwrap())
                    .style("color", self.style_param("text_color").unwrap())
                    .style("h_alignment", HorizontalAlign::Center.into())
                    .maybe_style("font", self.style_param("font"))
            )),
        )
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) {
        event.stop_bubbling();
    }

    fn on_mouse_enter(&mut self, event: &mut event::Event<event::MouseEnter>) {
        event.dirty();
        event.emit(Box::new(SelectMessage::Hover(self.id)));
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) {
        event.dirty();
        event.stop_bubbling();
        event.emit(Box::new(SelectMessage::Select(self.id)));
        event.emit(Box::new(SelectMessage::Close));
    }
}
