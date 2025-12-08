use std::marker::PhantomData;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, unbounded};
use hashbrown::{HashMap, HashSet};
use log::info;

use crate::component::Component;
use crate::event::EventCache;
use crate::focus::FocusState;
use crate::instrumenting::*;
use crate::layout::*;
use crate::node::Node;
use crate::render::{ActiveRenderer, Renderer};
use crate::renderable::Caches;
use crate::window::Window;
use crate::window::{clear_current_window, current_window, set_current_window};
use crate::{Dirty, NodeId, base_types::*};

/// `UI` is the main struct that holds the [`Window`], `Renderer` and [`Node`]s of an app.
/// It handles events and drawing+rendering.
/// You probably don't need to reference it directly, unless you're implementing a windowing backend.
///
/// Drawing (laying out [`Node`]s and assembling their [`Renderable`][crate::renderable::Renderable]s) and rendering
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
    references: Arc<RwLock<HashMap<String, u64>>>,
    focus_state: Arc<RwLock<FocusState>>,
    scale_factor: Arc<RwLock<f32>>,
    physical_size: Arc<RwLock<PixelSize>>,
    logical_size: Arc<RwLock<PixelSize>>,
    event_cache: EventCache,
    node_dirty: Arc<RwLock<Dirty>>,
}

impl<A: 'static + Component + Default + Send + Sync> super::LemnaUI for UI<A> {
    /// Signal to the draw thread that it may be time to draw a redraw the app.
    /// This performs three actions:
    /// - View, which calls [`view`][Component#method.view] on the root Component and then recursively across the children of the returned Node, thus recreating the Node graph. This does a number of sub tasks:
    ///   - State is transferred from the old graph to the new one, where possible. Some new Nodes will not have existed in the old graph.
    ///   - For net new Nodes (not present in the old graph), [`init`][Component#method.init] is called, and then a hash of input values is computed with [`props_hash`][Component#method.props_hash].
    ///   - For Nodes that existed in the old graph, [`props_hash`][Component#method.props_hash] is called on the new Component. If the new hash is not equal to the old one, then [`new_props`][Component#method.new_props] is called.
    /// - Layout, which calculates the positions and sizes all of the Nodes in the graph. See [`layout`][crate::layout] for how it interacts with the [`Component`] interface.
    /// - Render Nodes, which generates new [`Renderable`][crate::renderable::Renderable]s for each Node, or else recycles the previously generated ones. [`render_hash`][Component#method.render_hash] is called and compared to the old value -- if any -- to decide whether or not [`render`][Component#method.render] needs to be called.
    ///
    /// A draw will only occur if an event was handled that resulted in [`state_mut`][crate::state_component_impl] being called.
    fn draw(&mut self) {
        self.draw_channel.send(()).unwrap();
    }

    /// Signal to the render thread that it may be time to render a frame.
    /// A render will only occur if the draw thread has marked `frame_dirty` as true,
    /// which it will do after drawing. This thread does not interact with the user-facing API,
    /// just the [`Renderable`][crate::renderable::Renderable]s generated during [`draw`][UI#method.draw].
    fn render(&mut self) {
        self.render_channel.send(()).unwrap();
    }

    /// Add a font to the [`font_cache::FontCache`][crate::font_cache::FontCache]. The name provided is the name used to reference the font in a [`TextSegment`][crate::font_cache::TextSegment]. `bytes` are the bytes of a OpenType font, which must be held in static memory.
    fn add_font(&mut self, name: String, bytes: &'static [u8]) -> Result<(), &'static str> {
        self.caches.write().unwrap().font.add_font(name, bytes)
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
            *self.node_dirty.write().unwrap() = Dirty::Full;
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

    fn set_node_dirty(&mut self, dirty: Dirty) {
        if dirty != Dirty::No {
            *self.node_dirty.write().unwrap() += dirty;
        }
    }

    fn with_node<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Node) -> R,
    {
        f(&mut self.node.write().unwrap())
    }

    fn focus_stack(&self) -> Vec<NodeId> {
        self.focus_state.read().unwrap().stack().to_vec()
    }

    fn active_focus(&self) -> NodeId {
        self.focus_state.read().unwrap().active()
    }

    fn set_focus(&mut self, node_id: Option<NodeId>, event_stack: &[NodeId]) {
        let root_id = self.with_node(|node| node.id);
        self.focus_state
            .write()
            .unwrap()
            .set_active(node_id, event_stack, root_id);
    }

    fn get_reference(&self, reference: &str) -> Option<NodeId> {
        self.references.read().unwrap().get(reference).cloned()
    }

    fn with_focus_context<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut super::FocusContext) -> R,
    {
        let scale_factor = *self.scale_factor.read().unwrap();
        let mut node = self.node.write().unwrap();
        let mut focus_state = self.focus_state.write().unwrap();
        let references = self.references.read().unwrap();

        let mut ctx =
            super::FocusContext::new(&mut node, &mut focus_state, &references, scale_factor);

        let result = f(&mut ctx);

        // Apply dirty state
        *self.node_dirty.write().unwrap() += ctx.dirty;

        result
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
        let node_dirty = Arc::new(RwLock::new(Dirty::Full));
        let references = Arc::new(RwLock::new(HashMap::new()));
        let focus_state = Arc::new(RwLock::new(FocusState::default()));
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
            references.clone(),
            focus_state.clone(),
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
            references,
            focus_state,
            scale_factor,
            physical_size,
            logical_size,
            event_cache,
            node_dirty,
        };
        inst_end();
        n
    }

    #[allow(unused_variables)]
    fn render_thread(
        receiver: Receiver<()>,
        renderer: Arc<RwLock<Option<ActiveRenderer>>>,
        caches: Arc<RwLock<Caches>>,
        node: Arc<RwLock<Node>>,
        physical_size: Arc<RwLock<PixelSize>>,
        frame_dirty: Arc<RwLock<bool>>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            #[cfg(feature = "std_cpu")]
            let mut draw_target = SoftBufferDrawTarget::new(
                &**current_window().as_ref().unwrap(),
                *physical_size.read().unwrap(),
            );

            for _ in receiver.iter() {
                if *frame_dirty.read().unwrap() {
                    inst("UI::render");
                    // Pull out size so it gets pulled into the renderer lock
                    let size = *physical_size.read().unwrap();
                    #[allow(unused_mut)]
                    let mut caches = caches.write().unwrap();

                    #[cfg(feature = "wgpu_renderer")]
                    renderer.write().unwrap().as_mut().unwrap().render(
                        &node.read().unwrap(),
                        &mut caches,
                        size,
                    );

                    #[cfg(feature = "std_cpu")]
                    draw_target.resize(size);
                    #[cfg(feature = "std_cpu")]
                    renderer.write().unwrap().as_mut().unwrap().render(
                        &mut draw_target,
                        &node.read().unwrap(),
                        &mut caches,
                        size,
                    );
                    #[cfg(feature = "std_cpu")]
                    draw_target.present();

                    *frame_dirty.write().unwrap() = false;
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
        node_dirty: Arc<RwLock<Dirty>>,
        references: Arc<RwLock<HashMap<String, u64>>>,
        focus_state: Arc<RwLock<FocusState>>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            for _ in receiver.iter() {
                if *node_dirty.read().unwrap() == Dirty::Full {
                    // Set the node to clean right away so that concurrent events can reset it to dirty
                    *node_dirty.write().unwrap() = Dirty::No;
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
                        let mut new_references = HashMap::new();
                        let mut all_nodes = HashSet::new();
                        let mut new_focus_state = FocusState::default();
                        let root_id = old.id;
                        new.view(
                            Some(&mut old),
                            &mut new_references,
                            &mut new_focus_state,
                            &mut all_nodes,
                            root_id, // Root node is the default focus
                        );
                        inst_end();

                        inst("Node::layout");
                        new.layout(&caches, scale_factor);
                        inst_end();

                        inst("Node::update_focus");
                        // Handle focus changes
                        // We layout first, since this may trigger ScrollTo Signals, which require the layout to be up-to-date
                        let prev_focus = focus_state.read().unwrap().active();

                        if new_focus_state.active() == root_id {
                            new_focus_state
                                .inherit_active(&focus_state.read().unwrap(), &all_nodes);
                        }
                        if new_focus_state.active() != prev_focus {
                            // Use FocusContext to handle blur/focus events with signal support
                            let prev_focus = focus_state.read().unwrap();
                            let mut focus_ctx = super::FocusContext::new(
                                &mut new,
                                &mut new_focus_state,
                                &new_references,
                                scale_factor,
                            );
                            focus_ctx.handle_focus_change(&prev_focus);
                            *node_dirty.write().unwrap() += focus_ctx.dirty;
                        }
                        *references.write().unwrap() = new_references;
                        *focus_state.write().unwrap() = new_focus_state;
                        inst_end();

                        inst("Node::render");
                        let do_render = new.render(&mut caches, scale_factor);
                        inst_end();

                        *old = new;

                        if do_render {
                            current_window().as_ref().unwrap().redraw();
                        }
                        *frame_dirty.write().unwrap() = true;
                    }

                    inst_end();
                } else if *node_dirty.read().unwrap() == Dirty::RenderOnly {
                    // If the node is only render dirty, we don't need to re-compute it
                    *node_dirty.write().unwrap() = Dirty::No;

                    let mut caches = caches.write().unwrap();
                    let mut node = node.write().unwrap();
                    inst("UI::draw");
                    let scale_factor = *scale_factor.read().unwrap();
                    inst("Node::reposition");
                    node.reposition(scale_factor);
                    inst_end();

                    inst("Node::render");
                    let do_render = node.render(&mut caches, scale_factor);
                    inst_end();

                    if do_render {
                        current_window().as_ref().unwrap().redraw();
                    }
                    *frame_dirty.write().unwrap() = true;
                    inst_end();
                }
            }
        })
    }
}

#[cfg(feature = "std_cpu")]
struct SoftBufferDrawTarget {
    size: PixelSize,
    _context: softbuffer::Context,
    surface: softbuffer::Surface,
}

#[cfg(feature = "std_cpu")]
impl SoftBufferDrawTarget {
    fn new<W: raw_window_handle::HasRawDisplayHandle + raw_window_handle::HasRawWindowHandle>(
        window: W,
        size: PixelSize,
    ) -> Self {
        let context = unsafe { softbuffer::Context::new(&window).unwrap() };
        let surface = unsafe { softbuffer::Surface::new(&context, &window).unwrap() };
        let mut target = Self {
            // Start with a zero size so that we can resize it
            size: PixelSize {
                width: 0,
                height: 0,
            },
            _context: context,
            surface,
        };
        target.resize(size);
        target
    }

    fn resize(&mut self, size: PixelSize) {
        if self.size != size && size.width > 0 && size.height > 0 {
            self.size = size;
            if let Err(e) = self.surface.resize(
                core::num::NonZero::new(size.width).unwrap(),
                core::num::NonZero::new(size.height).unwrap(),
            ) {
                log::error!("Failed to resize softbuffer surface: {}", e);
            }
            log::debug!("Resized softbuffer surface to {:?}", self.size);
        }
    }

    // TODO: Use present_with_damage
    fn present(&mut self) {
        let buffer = self.surface.buffer_mut().unwrap();
        if let Err(e) = buffer.present() {
            log::error!("Failed to present softbuffer surface: {}", e);
        }
    }
}

#[cfg(feature = "std_cpu")]
impl embedded_graphics::draw_target::DrawTarget for SoftBufferDrawTarget {
    type Color = embedded_graphics::pixelcolor::Rgb888;
    type Error = softbuffer::SoftBufferError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::prelude::Pixel<Self::Color>>,
    {
        use embedded_graphics::prelude::{IntoStorage, Pixel};

        let mut buffer = self.surface.buffer_mut()?;
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.y >= 0
                && (coord.x as u32) < self.size.width
                && (coord.y as u32) < self.size.height
            {
                let index = coord.y as usize * self.size.width as usize + coord.x as usize;
                buffer[index] = color.into_storage();
            }
        }
        Ok(())
    }

    fn fill_contiguous<I>(
        &mut self,
        area: &embedded_graphics::primitives::Rectangle,
        colors: I,
    ) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        use embedded_graphics::prelude::{IntoStorage, Point, Size};
        use embedded_graphics::primitives::Rectangle;

        let mut buffer = self.surface.buffer_mut()?;

        let self_area = Rectangle::new(
            Point::zero(),
            Size::new(self.size.width as u32, self.size.height as u32),
        );
        let target_area = self_area.intersection(area);
        if let Some(bottom_right) = target_area.bottom_right() {
            // bottom right is inclusive
            let mut index = 0;
            let mut x = target_area.top_left.x;
            let mut y = target_area.top_left.y;
            for color in colors {
                if x > bottom_right.x {
                    x = target_area.top_left.x;
                    y += 1;
                } else if y > bottom_right.y {
                    break;
                }
                buffer[index] = color.into_storage();
                index += 1;
                x += 1;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "std_cpu")]
impl embedded_graphics::geometry::Dimensions for SoftBufferDrawTarget {
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        embedded_graphics::primitives::Rectangle::new(
            embedded_graphics::geometry::Point::new(0, 0),
            embedded_graphics::geometry::Size::new(self.size.width, self.size.height),
        )
    }
}
