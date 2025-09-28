extern crate alloc;

use alloc::{boxed::Box, string::String, vec, vec::Vec};
use core::marker::PhantomData;

use embedded_graphics::draw_target::DrawTarget;

use crate::base_types::PixelSize;
use crate::component::Component;
use crate::event::EventCache;
use crate::layout::Layout;
use crate::node::{Node, Registration};
use crate::render::{ActiveRenderer, Renderer, RgbColor};
use crate::renderable::Caches;
use crate::window::Window;
use crate::window::{clear_current_window, set_current_window};

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
    registrations: Vec<Registration>,
    node_dirty: bool,
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

    fn set_node_dirty(&mut self, dirty: bool) {
        self.node_dirty = dirty;
    }

    fn registrations(&self) -> Vec<Registration> {
        self.registrations.clone()
    }

    fn draw(&mut self) {
        if self.node_dirty {
            self.node_dirty = false;
            let size = self.size;
            let mut new = Node::new(
                Box::<A>::default(),
                0,
                lay!(size: size!(size.width as f32, size.height as f32)),
            );
            let mut new_registrations: Vec<Registration> = vec![];
            new.view(Some(&mut self.node), &mut new_registrations);
            self.registrations = new_registrations;

            new.layout(&self.node, &self.caches, 1.0);
            let do_render = new.render(&mut self.caches, Some(&mut self.node), 1.0);
            self.node = new;
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
            registrations: vec![],
            node_dirty: true,
            frame_dirty: false,
            event_cache: EventCache::new(1.0),
        }
    }
}
