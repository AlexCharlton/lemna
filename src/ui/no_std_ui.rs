extern crate alloc;

use alloc::{boxed::Box, string::String, vec::Vec};
use core::marker::PhantomData;

use embedded_graphics::draw_target::DrawTarget;
use hashbrown::{HashMap, HashSet};

use crate::base_types::PixelSize;
use crate::component::Component;
use crate::event::EventCache;
use crate::focus::FocusState;
use crate::layout::Layout;
use crate::node::Node;
use crate::render::{ActiveRenderer, Renderer, RgbColor};
use crate::renderable::Caches;
use crate::window::Window;
use crate::window::{clear_current_window, set_current_window};
use crate::{Dirty, NodeId};

pub struct UI<
    A: Component + Default,
    D: DrawTarget<Color = C, Error = E>,
    C: RgbColor,
    E: core::fmt::Debug,
> {
    node: Node,
    phantom_app: PhantomData<A>,
    draw_target: D,
    renderer: ActiveRenderer,
    size: PixelSize,
    caches: Caches,
    references: HashMap<String, NodeId>,
    focus_state: FocusState,
    node_dirty: Dirty,
    frame_dirty: bool,
    event_cache: EventCache,
}

impl<
    A: Component + Default + Send + Sync + 'static,
    D: DrawTarget<Color = C, Error = E>,
    C: RgbColor,
    E: core::fmt::Debug,
> super::LemnaUI for UI<A, D, C, E>
{
    fn with_node<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Node) -> R,
    {
        f(&mut self.node)
    }

    fn set_node_dirty(&mut self, dirty: Dirty) {
        self.node_dirty += dirty;
    }

    fn focus_stack(&self) -> Vec<NodeId> {
        self.focus_state.stack().to_vec()
    }

    fn active_focus(&self) -> NodeId {
        self.focus_state.active()
    }

    fn set_focus(&mut self, node_id: Option<NodeId>, event_stack: &[NodeId]) {
        let root_id = self.node.id;
        self.focus_state.set_active(node_id, event_stack, root_id);
    }

    fn get_reference(&self, reference: &str) -> Option<NodeId> {
        self.references.get(reference).cloned()
    }

    fn with_focus_context<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut super::FocusContext) -> R,
    {
        let scale_factor = self.event_cache.scale_factor;
        let mut ctx = super::FocusContext::new(
            &mut self.node,
            &mut self.focus_state,
            &self.references,
            scale_factor,
        );

        let result = f(&mut ctx);

        // Apply dirty state
        self.node_dirty += ctx.dirty;

        result
    }

    fn draw(&mut self) {
        if self.node_dirty == Dirty::Full {
            self.node_dirty = Dirty::No;
            let size = self.size;
            let mut new = Node::new(
                Box::<A>::default(),
                0,
                lay!(size: size!(size.width as f32, size.height as f32)),
            );
            let mut new_references = HashMap::new();
            let mut new_focus_state = FocusState::default();
            let mut all_nodes = HashSet::new();
            let root_id = self.node.id;

            new.view(
                Some(&mut self.node),
                &mut new_references,
                &mut new_focus_state,
                &mut all_nodes,
                root_id,
            );

            // Layout the new node
            new.layout(&self.caches, 1.0);

            // Handle focus changes if needed
            let prev_focus = self.focus_state.active();
            new_focus_state.inherit_active(&self.focus_state, &all_nodes, root_id);
            if new_focus_state.active() != prev_focus {
                // Focus changed during view - handle it with FocusContext
                let mut ctx = super::FocusContext::new(
                    &mut new,
                    &mut new_focus_state,
                    &new_references,
                    self.event_cache.scale_factor,
                );

                ctx.handle_focus_change(&self.focus_state);

                self.node_dirty += ctx.dirty;
            }

            self.references = new_references;
            self.focus_state = new_focus_state;

            let do_render = new.render(&mut self.caches, 1.0);
            self.node = new;
            self.frame_dirty = do_render;
        } else if self.node_dirty == Dirty::RenderOnly {
            self.node_dirty = Dirty::No;
            self.node.reposition(1.0);
            let do_render = self.node.render(&mut self.caches, 1.0);
            self.frame_dirty = do_render;
        }
    }

    fn render(&mut self) {
        if self.frame_dirty {
            self.renderer.render(
                &mut self.draw_target,
                &self.node,
                &mut self.caches,
                self.size,
            );
            self.frame_dirty = false;
        }
    }

    fn add_font(&mut self, name: String, bytes: &'static [u8]) -> Result<(), &'static str> {
        self.caches.font.add_font(name, bytes)
    }

    fn event_cache(&mut self) -> &mut EventCache {
        &mut self.event_cache
    }

    fn exit(&mut self) {
        clear_current_window();
    }
}

impl<
    A: Component + Default + Send + Sync + 'static,
    D: DrawTarget<Color = C, Error = E>,
    C: RgbColor,
    E: core::fmt::Debug,
> UI<A, D, C, E>
{
    pub fn new<W: Window + 'static>(window: W, draw_target: D) -> Self {
        let size = window.physical_size();
        let mut component = A::default();
        component.init();
        let renderer = ActiveRenderer::new(&window);

        set_current_window(Box::new(window));
        Self {
            node: Node::new(Box::new(component), 0, Layout::default()),
            phantom_app: PhantomData,
            draw_target,
            size,
            renderer,
            caches: Caches::default(),
            references: HashMap::new(),
            focus_state: FocusState::default(),
            node_dirty: Dirty::Full,
            frame_dirty: false,
            event_cache: EventCache::new(1.0),
        }
    }
}
