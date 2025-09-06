use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, unbounded};
use log::info;

use super::window::{clear_current_window, current_window, set_current_window};
use crate::base_types::*;
use crate::component::Component;
use crate::event::EventCache;
use crate::instrumenting::*;
use crate::layout::*;
use crate::node::{Node, Registration};
use crate::render::{ActiveRenderer, Caches, Renderer};
use crate::window::Window;

/// `UI` is the main struct that holds the [`Window`], `Renderer` and [`Node`]s of an app.
/// It handles events and drawing+rendering.
/// You probably don't need to reference it directly, unless you're implementing a windowing backend.
///
/// Drawing (laying out [`Node`]s and assembling their [`Renderable`][crate::renderables::Renderable]s) and rendering
/// (painting the `Renderables` onto the `Window`'s frame) are performed in separate threads
/// from the handling of events/render requests. This prevents hanging when handling events
/// which could otherwise happen if rendering takes a while. Even though the wgpu rendering pipeline
/// itself is quite efficient, delays have been observed when fetching
/// the next frame in the swapchain after resizing on certain platforms.
/// Event handling happens on the same thread that the [`current_window`] is accessible from.
pub struct UI<A: Component + Default + Send + Sync> {
    renderer: Arc<RwLock<Option<ActiveRenderer>>>,
    caches: Arc<RwLock<Caches>>,
    _render_thread: JoinHandle<()>,
    _draw_thread: JoinHandle<()>,
    render_channel: Sender<()>,
    draw_channel: Sender<()>,
    node: Arc<RwLock<Node>>,
    phantom_app: PhantomData<A>,
    registrations: Arc<RwLock<Vec<Registration>>>,
    scale_factor: Arc<RwLock<f32>>,
    physical_size: Arc<RwLock<PixelSize>>,
    logical_size: Arc<RwLock<PixelSize>>,
    event_cache: EventCache,
    node_dirty: Arc<RwLock<bool>>,
}

impl<A: 'static + Component + Default + Send + Sync> super::LemnaUI for UI<A> {
    /// Signal to the draw thread that it may be time to draw a redraw the app.
    /// This performs three actions:
    /// - View, which calls [`view`][Component#method.view] on the root Component and then recursively across the children of the returned Node, thus recreating the Node graph. This does a number of sub tasks:
    ///   - State is transferred from the old graph to the new one, where possible. Some new Nodes will not have existed in the old graph.
    ///   - For net new Nodes (not present in the old graph), [`init`][Component#method.init] is called, and then a hash of input values is computed with [`props_hash`][Component#method.props_hash].
    ///   - For Nodes that existed in the old graph, [`props_hash`][Component#method.props_hash] is called on the new Component. If the new hash is not equal to the old one, then [`new_props`][Component#method.new_props] is called.
    ///   - [`register`][Component#method.register] is also called on all Nodes.
    /// - Layout, which calculates the positions and sizes all of the Nodes in the graph. See [`layout`][crate::layout] for how it interacts with the [`Component`] interface.
    /// - Render Nodes, which generates new [`Renderable`][crate::renderables::Renderable]s for each Node, or else recycles the previously generated ones. [`render_hash`][Component#method.render_hash] is called and compared to the old value -- if any -- to decide whether or not [`render`][Component#method.render] needs to be called.
    ///
    /// A draw will only occur if an event was handled that resulted in [`state_mut`][crate::state_component_impl] being called.
    fn draw(&mut self) {
        self.draw_channel.send(()).unwrap();
    }

    /// Signal to the render thread that it may be time to render a frame.
    /// A render will only occur if the draw thread has marked `frame_dirty` as true,
    /// which it will do after drawing. This thread does not interact with the user-facing API,
    /// just the [`Renderable`][crate::renderables::Renderable]s generated during [`draw`][UI#method.draw].
    fn render(&mut self) {
        self.render_channel.send(()).unwrap();
    }

    /// Add a font to the [`font_cache::FontCache`][crate::font_cache::FontCache]. The name provided is the name used to reference the font in a [`TextSegment`][crate::font_cache::TextSegment]. `bytes` are the bytes of a OpenType font, which must be held in static memory.
    fn add_font(&mut self, name: String, bytes: &'static [u8]) {
        self.caches.write().unwrap().font.add_font(name, bytes);
    }

    fn resize(&mut self) {
        if current_window().is_none() {
            return;
        }
        let new_size = current_window().as_ref().unwrap().physical_size();
        if new_size.width != 0 && new_size.height != 0 {
            let scale_factor = current_window().as_ref().unwrap().scale_factor();
            *self.physical_size.write().unwrap() = new_size;
            *self.logical_size.write().unwrap() = current_window().as_ref().unwrap().logical_size();
            *self.scale_factor.write().unwrap() = scale_factor;
            self.event_cache.scale_factor = scale_factor;
            *self.node_dirty.write().unwrap() = true;
            current_window().as_ref().unwrap().redraw(); // Always redraw after resizing
        }
    }

    fn exit(&mut self) {
        clear_current_window();

        let renderer = self.renderer.write().unwrap().take().unwrap();
        drop(renderer);
    }

    fn event_cache(&mut self) -> &mut EventCache {
        &mut self.event_cache
    }

    fn set_node_dirty(&mut self, dirty: bool) {
        *self.node_dirty.write().unwrap() = dirty;
    }

    fn registrations(&self) -> Vec<Registration> {
        self.registrations.read().unwrap().clone()
    }

    fn with_node<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Node) -> R,
    {
        f(&mut self.node.write().unwrap())
    }
}

impl<A: 'static + Component + Default + Send + Sync> UI<A> {
    /// Create a new `UI`, given a [`Window`].
    pub fn new<W: Window + 'static>(window: W) -> Self {
        let scale_factor = Arc::new(RwLock::new(window.scale_factor()));
        // dbg!(scale_factor);
        let physical_size = Arc::new(RwLock::new(window.physical_size()));
        let logical_size = Arc::new(RwLock::new(window.logical_size()));
        info!(
            "New window with physical size {:?} client size {:?} and scale factor {:?}",
            physical_size, logical_size, scale_factor
        );
        inst("UI::new");
        let mut component = A::default();
        component.init();

        let renderer = Arc::new(RwLock::new(Some(ActiveRenderer::new(&window))));
        let event_cache = EventCache::new(window.scale_factor());
        set_current_window(Box::new(window));

        // Root node
        let node = Arc::new(RwLock::new(Node::new(
            Box::new(component),
            0,
            Layout::default(),
        )));
        let frame_dirty = Arc::new(RwLock::new(false));
        let node_dirty = Arc::new(RwLock::new(true));
        let registrations: Arc<RwLock<Vec<Registration>>> = Default::default();
        let caches = Arc::new(RwLock::new(Caches::default()));

        // Create a channel to speak to the renderer. Every time we send to this channel we want to trigger a render;
        let (render_channel, receiver) = unbounded::<()>();
        let render_thread = Self::render_thread(
            receiver,
            renderer.clone(),
            caches.clone(),
            node.clone(),
            physical_size.clone(),
            frame_dirty.clone(),
        );

        // Create a channel to speak to the drawer. Every time we send to this channel we want to trigger a draw;
        let (draw_channel, receiver) = unbounded::<()>();
        let draw_thread = Self::draw_thread(
            receiver,
            caches.clone(),
            node.clone(),
            logical_size.clone(),
            scale_factor.clone(),
            frame_dirty,
            node_dirty.clone(),
            registrations.clone(),
        );

        let n = Self {
            renderer,
            caches,
            render_channel,
            _render_thread: render_thread,
            draw_channel,
            _draw_thread: draw_thread,
            node,
            phantom_app: PhantomData,
            registrations,
            scale_factor,
            physical_size,
            logical_size,
            event_cache,
            node_dirty,
        };
        inst_end();
        n
    }

    fn render_thread(
        receiver: Receiver<()>,
        renderer: Arc<RwLock<Option<ActiveRenderer>>>,
        caches: Arc<RwLock<Caches>>,
        node: Arc<RwLock<Node>>,
        physical_size: Arc<RwLock<PixelSize>>,
        frame_dirty: Arc<RwLock<bool>>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            for _ in receiver.iter() {
                if *frame_dirty.read().unwrap() {
                    inst("UI::render");
                    // Pull out size so it gets pulled into the renderer lock
                    let size = *physical_size.read().unwrap();
                    let mut caches = caches.write().unwrap();
                    renderer.write().unwrap().as_mut().unwrap().render(
                        &node.read().unwrap(),
                        &mut caches,
                        size,
                    );
                    *frame_dirty.write().unwrap() = false;
                    // println!("rendered");
                    inst_end();
                }
            }
        })
    }

    fn draw_thread(
        receiver: Receiver<()>,
        caches: Arc<RwLock<Caches>>,
        node: Arc<RwLock<Node>>,
        logical_size: Arc<RwLock<PixelSize>>,
        scale_factor: Arc<RwLock<f32>>,
        frame_dirty: Arc<RwLock<bool>>,
        node_dirty: Arc<RwLock<bool>>,
        registrations: Arc<RwLock<Vec<Registration>>>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            for _ in receiver.iter() {
                if *node_dirty.read().unwrap() {
                    // Set the node to clean right away so that concurrent events can reset it to dirty
                    *node_dirty.write().unwrap() = false;
                    inst("UI::draw");
                    let logical_size = *logical_size.read().unwrap();
                    let scale_factor = *scale_factor.read().unwrap();
                    let mut new = Node::new(
                        Box::<A>::default(),
                        0,
                        lay!(size: size!(logical_size.width as f32, logical_size.height as f32)),
                    );

                    {
                        let mut caches = caches.write().unwrap();

                        // We need to acquire a lock on the node once we `view` it, because we remove its state at this point
                        let mut old = node.write().unwrap();
                        inst("Node::view");
                        let mut new_registrations: Vec<Registration> = vec![];
                        new.view(Some(&mut old), &mut new_registrations);
                        *registrations.write().unwrap() = new_registrations;
                        inst_end();

                        inst("Node::layout");
                        new.layout(&old, &caches.font, scale_factor);
                        inst_end();

                        inst("Node::render");
                        let do_render = new.render(&mut caches, Some(&mut old), scale_factor);
                        inst_end();

                        *old = new;

                        if do_render {
                            current_window().as_ref().unwrap().redraw();
                        }
                        *frame_dirty.write().unwrap() = true;
                    }

                    inst_end();
                }
            }
        })
    }
}
