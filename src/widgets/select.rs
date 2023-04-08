use std::hash::Hash;

use crate::base_types::*;
use crate::component::{Component, ComponentHasher, Message, RenderContext};
use crate::event;
use crate::font_cache::HorizontalAlign;
use crate::layout::*;
use crate::render::wgpu::{Shape, WGPURenderable, WGPURenderer};
use crate::{node, txt, Node};
use lemna_macros::{state_component, state_component_impl};

#[derive(Debug, Clone)]
pub struct SelectStyle {
    pub text_color: Color,
    pub font_size: f32,
    pub font: Option<String>,
    pub background_color: Color,
    pub highlight_color: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radius: f32,
    pub padding: f32,
    pub max_height: f32,
}

impl Default for SelectStyle {
    fn default() -> Self {
        Self {
            text_color: 0.0.into(),
            font_size: 12.0,
            font: None,
            background_color: 1.0.into(),
            highlight_color: 0.9.into(),
            border_color: 0.0.into(),
            border_width: 2.0,
            radius: 4.0,
            padding: 2.0,
            max_height: 250.0,
        }
    }
}

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

#[state_component(SelectState)]
pub struct Select<M: Send + Sync>
where
    M: Send + Sync,
{
    pub selection: Vec<M>,
    pub style: SelectStyle,
    pub selected: usize,
    on_change: Option<Box<dyn Fn(usize, &M) -> Message + Send + Sync>>,
}

impl<M: std::fmt::Debug + Send + Sync> std::fmt::Debug for Select<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("selection", &self.selection)
            .field("style", &self.style)
            .finish()
    }
}

impl<M: ToString + Send + Sync> Select<M> {
    pub fn new(selection: Vec<M>, selected: usize, style: SelectStyle) -> Self {
        Self {
            selection,
            style,
            selected,
            on_change: None,
            state: Some(SelectState::default()),
        }
    }

    pub fn on_change(mut self, change_fn: Box<dyn Fn(usize, &M) -> Message + Send + Sync>) -> Self {
        self.on_change = Some(change_fn);
        self
    }
}

#[state_component_impl(SelectState)]
impl<M: 'static + std::fmt::Debug + Clone + ToString + std::fmt::Display + Send + Sync>
    Component<WGPURenderer> for Select<M>
{
    fn view(&self) -> Option<Node<WGPURenderer>> {
        let mut base =
            node!(super::Div::new(), lay!(direction: Direction::Column)).push(node!(SelectBox {
                selection: self
                    .selection
                    .get(self.state_ref().selected)
                    .map(|x| x.clone()),
                style: self.style.clone(),
            }));
        if self.state_ref().open {
            base = base.push(node!(
                SelectList {
                    selections: self.selection.clone(),
                    style: self.style.clone(),
                    hovering: self.state_ref().hovering,
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
#[derive(Debug)]
struct SelectBox<M> {
    selection: Option<M>,
    style: SelectStyle,
}

impl<M: 'static + std::fmt::Debug + Clone + ToString> Component<WGPURenderer> for SelectBox<M> {
    fn view(&self) -> Option<Node<WGPURenderer>> {
        let mut base = node!(
            super::RoundedRect {
                background_color: self.style.background_color,
                border_color: self.style.border_color,
                border_width: self.style.border_width,
                radius: (
                    self.style.radius,
                    self.style.radius,
                    self.style.radius,
                    self.style.radius
                ),
            },
            lay!(
                size: size_pct!(100.0),
                padding: rect!(self.style.padding),
                cross_alignment: Alignment::Center,
                axis_alignment: Alignment::Center,
                direction: Direction::Row,
            )
        );
        if let Some(selection) = self.selection.as_ref() {
            base = base
                .push(node!(super::Text::new(
                    txt!(selection.to_string()),
                    super::TextStyle {
                        size: self.style.font_size,
                        color: self.style.text_color,
                        font: self.style.font.clone(),
                        h_alignment: HorizontalAlign::Center,
                    }
                )))
                .push(node!(
                    Caret {
                        style: self.style.clone()
                    },
                    lay!(
                        size: size!(self.style.font_size / 2.0),
                        // TODO: Margin here is awkward
                        margin: rect!(Auto, self.style.padding)
                    ),
                    1
                ))
        }
        Some(base)
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) -> Vec<Message> {
        event.stop_bubbling();
        vec![]
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) -> Vec<Message> {
        event.dirty();
        event.focus();
        event.stop_bubbling();
        vec![Box::new(SelectMessage::OpenClose)]
    }

    fn on_blur(&mut self, event: &mut event::Event<event::Blur>) -> Vec<Message> {
        event.dirty();
        vec![Box::new(SelectMessage::Close)]
    }
}

#[derive(Debug)]
struct Caret {
    style: SelectStyle,
}

use lyon::path::Path;
use lyon::tessellation::math as lyon_math;
impl Component<WGPURenderer> for Caret {
    fn render<'a>(
        &mut self,
        context: RenderContext<'a, WGPURenderer>,
    ) -> Option<Vec<WGPURenderable>> {
        let scale = 1.0; // TODO: Adjust

        let mut path_builder = Path::builder();
        let w = context.aabb.width();
        let h = context.aabb.height();
        let start = h / 2.0;
        path_builder.move_to(lyon_math::point(0.0, start));
        path_builder.line_to(lyon_math::point(w / 2.0, h));
        path_builder.line_to(lyon_math::point(w, start));

        let (geometry, _) = Shape::path_to_shape_geometry(path_builder.build(), false, true);

        Some(vec![WGPURenderable::Shape(Shape::stroke(
            geometry,
            self.style.border_color,
            scale,
            0.0,
            &mut context.renderer.shape_pipeline,
            context.prev_state.as_ref().and_then(|v| match v.get(0) {
                Some(WGPURenderable::Shape(r)) => Some(r.buffer_id),
                _ => None,
            }),
        ))])
    }
}

//
// SelectList
// Visible after opening: The full selection list
#[derive(Debug)]
struct SelectList<M>
where
    M: Send + Sync,
{
    selections: Vec<M>,
    style: SelectStyle,
    hovering: usize,
}

impl<M: 'static + std::fmt::Debug + Clone + ToString + Send + Sync> Component<WGPURenderer>
    for SelectList<M>
{
    fn view(&self) -> Option<Node<WGPURenderer>> {
        let mut l = node!(
            super::Div::new()
                .bg(self.style.background_color)
                .scroll(super::ScrollDescriptor {
                    scroll_y: true,
                    ..Default::default()
                }),
            lay!(
                direction: Direction::Column,
                cross_alignment: Alignment::Stretch,
            )
        );
        for (i, s) in self.selections.iter().enumerate() {
            l = l.push(node!(
                SelectEntry {
                    selection: s.clone(),
                    id: i,
                    style: self.style.clone(),
                    selected: i == self.hovering,
                },
                lay!(),
                i as u64
            ));
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
            // Set size based on list elements and style.max_height
            let mut h = inner_scale.height;
            let mut w = inner_scale.width;
            if h > self.style.max_height * scale_factor {
                h = self.style.max_height * scale_factor;
                w = inner_scale.width + super::ScrollDescriptor::default().bar_width * scale_factor;
            }

            // Shrink if there isn't enough room
            let room_above = parent_aabb.pos.y - frame.pos.y;
            let room_bellow = frame.bottom_right.y - parent_aabb.bottom_right.y;
            if h > room_bellow && h > room_above {
                h = room_bellow.max(room_above);
                w = inner_scale.width + super::ScrollDescriptor::default().bar_width * scale_factor;
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
#[derive(Debug)]
struct SelectEntry<M>
where
    M: Send + Sync,
{
    selection: M,
    id: usize,
    selected: bool,
    style: SelectStyle,
}

impl<M: 'static + std::fmt::Debug + Clone + ToString + Send + Sync> Component<WGPURenderer>
    for SelectEntry<M>
{
    fn view(&self) -> Option<Node<WGPURenderer>> {
        let mut div = super::Div::new();
        if self.selected {
            div = div.bg(self.style.highlight_color)
        }

        let mut base = node!(
            div,
            lay!(size: size_pct!(100.0), padding: rect!(self.style.padding),)
        );

        base = base.push(node!(super::Text::new(
            txt!(self.selection.to_string()),
            super::TextStyle {
                size: self.style.font_size,
                color: self.style.text_color,
                font: self.style.font.clone(),
                h_alignment: HorizontalAlign::Center,
            }
        )));
        Some(base)
    }

    fn on_mouse_motion(&mut self, event: &mut event::Event<event::MouseMotion>) -> Vec<Message> {
        event.stop_bubbling();
        vec![]
    }

    fn on_mouse_enter(&mut self, event: &mut event::Event<event::MouseEnter>) -> Vec<Message> {
        event.dirty();
        vec![Box::new(SelectMessage::Hover(self.id))]
    }

    fn on_click(&mut self, event: &mut event::Event<event::Click>) -> Vec<Message> {
        event.dirty();
        event.stop_bubbling();
        vec![
            Box::new(SelectMessage::Select(self.id)),
            Box::new(SelectMessage::Close),
        ]
    }
}
