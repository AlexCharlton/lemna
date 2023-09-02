use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crossbeam_channel::{unbounded, Receiver, Sender};
use log::info;

use crate::base_types::*;
use crate::component::Component;
use crate::event::{self, Event, EventCache};
use crate::font_cache::FontCache;
use crate::input::*;
use crate::instrumenting::*;
use crate::layout::*;
use crate::node::Node;
use crate::render::Renderer;
use crate::window::Window;

/// `UI` is the main struct that holds the `Window`, `Renderer` and `Node`s of an `App`.
/// It handles events and drawing/rendering.
/// Drawing (laying out `Nodes` and assembling their `Renderable`s) and rendering
/// (painting the `Renderables` onto the `Window`'s frame) are performed in separate threads
/// from the handling of events/render requests. This prevents hanging when handling events
/// which could otherwise happen if rendering takes a while. Even though the wgpu rendering pipeline
/// itself is quite efficient, delays have been observed when fetching
/// the next frame in the swapchain after resizing on certain platforms.
/// Event handling happens on the same thread that the `current_window` is accessible from.
pub struct UI<W: Window, R: Renderer, A: Component + Default + Send + Sync> {
    pub renderer: Arc<RwLock<Option<R>>>,
    pub window: Arc<RwLock<W>>,
    _render_thread: JoinHandle<()>,
    _draw_thread: JoinHandle<()>,
    render_channel: Sender<()>,
    draw_channel: Sender<()>,
    node: Arc<RwLock<Node>>,
    phantom_app: PhantomData<A>,
    scale_factor: Arc<RwLock<f32>>,
    physical_size: Arc<RwLock<PixelSize>>,
    logical_size: Arc<RwLock<PixelSize>>,
    event_cache: EventCache,
    font_cache: Arc<RwLock<FontCache>>,
    node_dirty: Arc<RwLock<bool>>,
}

thread_local!(
    static IMMEDIATE_FOCUS: UnsafeCell<Option<u64>> = {
        UnsafeCell::new(None)
    }
);

fn immediate_focus() -> Option<u64> {
    *IMMEDIATE_FOCUS.with(|r| unsafe { r.get().as_ref().unwrap() })
}

fn clear_immediate_focus() {
    IMMEDIATE_FOCUS.with(|r| unsafe { *r.get().as_mut().unwrap() = None })
}

pub fn focus_immediately<T>(event: &Event<T>) {
    IMMEDIATE_FOCUS.with(|r| unsafe { *r.get().as_mut().unwrap() = event.current_node_id })
}

thread_local!(
    static CURRENT_WINDOW: UnsafeCell<Option<Arc<RwLock<dyn Window>>>> = {
        UnsafeCell::new(None)
    }
);

pub fn current_window<'a>() -> Option<RwLockReadGuard<'a, dyn Window>> {
    CURRENT_WINDOW.with(|r| unsafe {
        r.get()
            .as_ref()
            .unwrap()
            .as_ref()
            .map(|w| w.read().unwrap())
    })
}

fn clear_current_window() {
    CURRENT_WINDOW.with(|r| unsafe { *r.get().as_mut().unwrap() = None })
}

pub fn set_current_window(window: Arc<RwLock<dyn Window>>) {
    CURRENT_WINDOW.with(|r| unsafe { *r.get().as_mut().unwrap() = Some(window) })
}

impl<
        W: 'static + Window,
        R: 'static + Renderer,
        A: 'static + Component + Default + Send + Sync,
    > UI<W, R, A>
{
    fn node_ref(&self) -> RwLockReadGuard<'_, Node> {
        self.node.read().unwrap()
    }

    fn node_mut(&mut self) -> RwLockWriteGuard<'_, Node> {
        self.node.write().unwrap()
    }

    fn render_thread(
        receiver: Receiver<()>,
        renderer: Arc<RwLock<Option<R>>>,
        node: Arc<RwLock<Node>>,
        font_cache: Arc<RwLock<FontCache>>,
        physical_size: Arc<RwLock<PixelSize>>,
        frame_dirty: Arc<RwLock<bool>>,
    ) -> JoinHandle<()>
    where
        R: Renderer,
    {
        thread::spawn(move || {
            for _ in receiver.iter() {
                if *frame_dirty.read().unwrap() {
                    inst("UI::render");
                    // Pull out size so it gets pulled into the renderer lock
                    let size = *physical_size.read().unwrap();
                    renderer.write().unwrap().as_mut().unwrap().render(
                        &node.read().unwrap(),
                        size,
                        &font_cache.read().unwrap(),
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
        renderer: Arc<RwLock<Option<R>>>,
        node: Arc<RwLock<Node>>,
        font_cache: Arc<RwLock<FontCache>>,
        logical_size: Arc<RwLock<PixelSize>>,
        scale_factor: Arc<RwLock<f32>>,
        frame_dirty: Arc<RwLock<bool>>,
        node_dirty: Arc<RwLock<bool>>,
        window: Arc<RwLock<W>>,
    ) -> JoinHandle<()>
    where
        R: Renderer,
    {
        thread::spawn(move || {
            for _ in receiver.iter() {
                if *node_dirty.read().unwrap() {
                    // Set the node to clean right away so that concurrent events can reset it to dirty
                    *node_dirty.write().unwrap() = false;
                    inst("UI::draw");
                    let logical_size = *logical_size.read().unwrap();
                    let scale_factor = *scale_factor.read().unwrap();
                    let mut new = Node::new(
                        Box::new(A::default()),
                        0,
                        lay!(size: size!(logical_size.width as f32, logical_size.height as f32)),
                    );

                    {
                        // We need to lock the renderer while we modify the node, so that we don't try to render it while doing so
                        // Since this will cause a deadlock
                        let mut renderer = renderer.write().unwrap();

                        // We need to acquire a lock on the node once we `view` it, because we remove its state at this point
                        let mut old = node.write().unwrap();
                        inst("Node::view");
                        new.view(Some(&mut old));
                        inst_end();

                        inst("Node::layout");
                        new.layout(&old, &font_cache.read().unwrap(), scale_factor);
                        inst_end();

                        inst("Node::render");
                        let do_render = new.render(
                            renderer.as_mut().unwrap().caches(),
                            Some(&mut old),
                            font_cache.clone(),
                            scale_factor,
                        );
                        inst_end();

                        *old = new;

                        if do_render {
                            window.write().unwrap().redraw();
                        }
                        *frame_dirty.write().unwrap() = true;
                    }

                    inst_end();
                }
            }
        })
    }

    pub fn new(window: W) -> Self {
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

        let renderer = Arc::new(RwLock::new(Some(R::new(&window))));
        let event_cache = EventCache::new(window.scale_factor());
        let window = Arc::new(RwLock::new(window));
        set_current_window(window.clone());

        let node = Arc::new(RwLock::new(Node::new(
            Box::new(component),
            0,
            Layout::default(),
        )));
        let font_cache = Arc::new(RwLock::new(FontCache::default()));
        let frame_dirty = Arc::new(RwLock::new(false));
        let node_dirty = Arc::new(RwLock::new(true));

        // Create a channel to speak to the renderer. Every time we send to this channel we want to trigger a render;
        let (render_channel, receiver) = unbounded::<()>();
        let render_thread = Self::render_thread(
            receiver,
            renderer.clone(),
            node.clone(),
            font_cache.clone(),
            physical_size.clone(),
            frame_dirty.clone(),
        );

        // Create a channel to speak to the drawer. Every time we send to this channel we want to trigger a draw;
        let (draw_channel, receiver) = unbounded::<()>();
        let draw_thread = Self::draw_thread(
            receiver,
            renderer.clone(),
            node.clone(),
            font_cache.clone(),
            logical_size.clone(),
            scale_factor.clone(),
            frame_dirty,
            node_dirty.clone(),
            window.clone(),
        );

        let n = Self {
            renderer,
            render_channel,
            _render_thread: render_thread,
            draw_channel,
            _draw_thread: draw_thread,
            window,
            node,
            phantom_app: PhantomData,
            scale_factor,
            physical_size,
            logical_size,
            event_cache,
            font_cache,
            node_dirty,
        };
        inst_end();
        n
    }

    pub fn draw(&mut self) {
        self.draw_channel.send(()).unwrap();
    }

    pub fn render(&mut self) {
        self.render_channel.send(()).unwrap();
    }

    fn blur(&mut self) {
        let mut blur_event = Event::new(event::Blur, &self.event_cache);
        blur_event.target = Some(self.event_cache.focus);
        self.node_mut().blur(&mut blur_event);
        self.handle_dirty_event(&blur_event);

        self.event_cache.focus = self.node.read().unwrap().id; // The root note gets focus
    }

    fn handle_focus_or_blur<T>(&mut self, event: &Event<T>) {
        if event.focus.is_none() {
            self.blur();
        } else if event.focus != Some(self.event_cache.focus) {
            self.blur();
            self.event_cache.focus = event.focus.unwrap();
            let mut focus_event = Event::new(event::Focus, &self.event_cache);
            focus_event.target = Some(self.event_cache.focus);
            self.node_mut().focus(&mut focus_event);
            self.handle_dirty_event(&focus_event);
        }
    }

    fn handle_dirty_event<T>(&mut self, event: &Event<T>) {
        if event.dirty {
            *self.node_dirty.write().unwrap() = true
        }
    }

    fn handle_event<T, F>(&mut self, handler: F, event: &mut Event<T>, target: Option<u64>)
    where
        F: Fn(&mut Node, &mut Event<T>),
    {
        event.target = target;
        handler(&mut *self.node_mut(), event);
        self.handle_focus_or_blur(event);
        self.handle_dirty_event(event);
    }

    fn handle_event_without_focus<T, F>(
        &mut self,
        handler: F,
        event: &mut Event<T>,
        target: Option<u64>,
    ) where
        F: Fn(&mut Node, &mut Event<T>),
    {
        event.target = target;
        handler(&mut *self.node_mut(), event);
        self.handle_dirty_event(event);
    }

    pub fn handle_input(&mut self, input: &Input) {
        inst("UI::handle_input");
        // if self.node.is_none() || self.renderer.is_none() {
        //     // If there is no node, the event has happened after exiting
        //     // For some reason checking for both works better, even though they're unset at the same time?
        //     return;
        // }
        match input {
            Input::Resize => {
                let new_size = self.window.read().unwrap().physical_size();
                if new_size.width != 0 && new_size.height != 0 {
                    let scale_factor = self.window.read().unwrap().scale_factor();
                    *self.physical_size.write().unwrap() = new_size;
                    *self.logical_size.write().unwrap() =
                        self.window.read().unwrap().logical_size();
                    *self.scale_factor.write().unwrap() = scale_factor;
                    self.event_cache.scale_factor = scale_factor;
                    *self.node_dirty.write().unwrap() = true;
                    self.window.write().unwrap().redraw(); // Always redraw after resizing
                }
            }
            Input::Motion(Motion::Mouse { x, y }) => {
                let pos = Point::new(*x, *y) * self.event_cache.scale_factor;

                if let Some(button) = self.event_cache.mouse_button_held() {
                    if self.event_cache.drag_started.is_none() {
                        self.event_cache.drag_started = Some(self.event_cache.mouse_position);
                    }

                    let drag_start = self.event_cache.drag_started.unwrap();

                    if self.event_cache.drag_button.is_none()
                        && ((drag_start.x - pos.x).abs() > event::DRAG_THRESHOLD
                            || (drag_start.y - pos.y).abs() > event::DRAG_THRESHOLD)
                    {
                        self.event_cache.drag_button = Some(button);
                        let mut drag_start_event =
                            Event::new(event::DragStart(button), &self.event_cache);
                        drag_start_event.mouse_position = self.event_cache.drag_started.unwrap();
                        self.handle_event(Node::drag_start, &mut drag_start_event, None);
                        self.event_cache.drag_target = drag_start_event.target;
                    }
                }

                self.event_cache.mouse_position = pos;
                let mut motion_event = Event::new(event::MouseMotion, &self.event_cache);
                self.handle_event_without_focus(Node::mouse_motion, &mut motion_event, None);

                let held_button = self.event_cache.mouse_button_held();
                if held_button.is_some() && self.event_cache.drag_button.is_some() {
                    let mut drag_event = Event::new(
                        event::Drag {
                            button: held_button.unwrap(),
                            start_pos: self.event_cache.drag_started.unwrap(),
                        },
                        &self.event_cache,
                    );
                    self.handle_event_without_focus(
                        Node::drag,
                        &mut drag_event,
                        self.event_cache.drag_target,
                    );
                } else if motion_event.target != self.event_cache.mouse_over {
                    if self.event_cache.mouse_over.is_some() {
                        let mut leave_event = Event::new(event::MouseLeave, &self.event_cache);
                        self.handle_event(
                            Node::mouse_leave,
                            &mut leave_event,
                            self.event_cache.mouse_over,
                        );
                    }
                    if motion_event.target.is_some() {
                        let mut enter_event = Event::new(event::MouseEnter, &self.event_cache);
                        self.handle_event(Node::mouse_enter, &mut enter_event, motion_event.target);
                    }
                    self.event_cache.mouse_over = motion_event.target;
                }
            }
            Input::Motion(Motion::Scroll { x, y }) => {
                let mut event = Event::new(
                    event::Scroll {
                        x: *x * self.event_cache.scale_factor,
                        y: *y * self.event_cache.scale_factor,
                    },
                    &self.event_cache,
                );
                self.handle_event_without_focus(Node::scroll, &mut event, None);
            }
            Input::Press(Button::Mouse(b)) => {
                self.event_cache.mouse_down(*b);
                let mut event = Event::new(event::MouseDown(*b), &self.event_cache);
                self.handle_event(Node::mouse_down, &mut event, None);
            }
            Input::Release(Button::Mouse(b)) => {
                let mut event = Event::new(event::MouseUp(*b), &self.event_cache);
                self.handle_event(Node::mouse_up, &mut event, None);

                let mut is_double_click = false;
                // Double clicking
                if b == &MouseButton::Left {
                    if self.event_cache.last_mouse_click.elapsed().as_millis()
                        < event::DOUBLE_CLICK_INTERVAL_MS
                        && self
                            .event_cache
                            .last_mouse_click_position
                            .dist(self.event_cache.mouse_position)
                            < event::DOUBLE_CLICK_MAX_DIST
                    {
                        is_double_click = true;
                    }
                    self.event_cache.last_mouse_click = Instant::now();
                    self.event_cache.last_mouse_click_position = self.event_cache.mouse_position;
                }

                // End drag
                if Some(*b) == self.event_cache.drag_button {
                    let mut drag_end_event = Event::new(
                        event::DragEnd {
                            button: *b,
                            start_pos: self.event_cache.drag_started.unwrap(),
                        },
                        &self.event_cache,
                    );
                    self.handle_event(
                        Node::drag_end,
                        &mut drag_end_event,
                        self.event_cache.drag_target,
                    );

                    let drag_distance = self
                        .event_cache
                        .drag_started
                        .unwrap()
                        .dist(self.event_cache.mouse_position);
                    if drag_distance < event::DRAG_CLICK_MAX_DIST {
                        // Send a Click event if the drag was quite short
                        let mut click_event = Event::new(event::Click(*b), &self.event_cache);
                        self.handle_event(Node::click, &mut click_event, None);
                    }

                    // Unfocus when clicking a thing not focused
                    if drag_end_event.current_node_id != Some(self.event_cache.focus)
                    // Ignore the root node, which is the default focus
                        && self.event_cache.focus != self.node_ref().id
                    {
                        self.blur();
                    }

                    // Clean up event cache
                    self.event_cache.drag_started = None;
                    self.event_cache.drag_button = None;
                    self.event_cache.mouse_up(*b);
                } else
                // Resolve click
                if self.event_cache.is_mouse_button_held(*b) {
                    self.event_cache.mouse_up(*b);
                    let event_current_node_id = if is_double_click {
                        let mut event = Event::new(event::DoubleClick(*b), &self.event_cache);
                        self.handle_event(Node::double_click, &mut event, None);
                        event.current_node_id
                    } else {
                        let mut event = Event::new(event::Click(*b), &self.event_cache);
                        self.handle_event(Node::click, &mut event, None);
                        event.current_node_id
                    };

                    // Unfocus when clicking a thing not focused
                    if event_current_node_id != Some(self.event_cache.focus)
                        // Ignore the root node, which is the default focus
                            && self.event_cache.focus != self.node_ref().id
                    {
                        self.blur();
                    }
                }
            }
            Input::Press(Button::Keyboard(k)) => {
                self.event_cache.key_down(*k);
                let mut event = Event::new(event::KeyDown(*k), &self.event_cache);
                let focus = event.focus;
                self.handle_event(Node::key_down, &mut event, focus);
            }
            Input::Release(Button::Keyboard(k)) => {
                if self.event_cache.key_held(*k) {
                    self.event_cache.key_up(*k);
                    let mut event = Event::new(event::KeyPress(*k), &self.event_cache);
                    let focus = event.focus;
                    self.handle_event(Node::key_press, &mut event, focus);
                }

                let mut event = Event::new(event::KeyUp(*k), &self.event_cache);
                let focus = event.focus;
                self.handle_event(Node::key_up, &mut event, focus);
            }
            Input::Text(s) => {
                let mods = self.event_cache.modifiers_held;
                if !mods.alt && !mods.ctrl && !mods.meta {
                    let mut event = Event::new(event::TextEntry(s.clone()), &self.event_cache);
                    let focus = event.focus;
                    self.handle_event(Node::text_entry, &mut event, focus);
                }
            }
            Input::Focus(false) => {
                self.event_cache.clear();
                let mut event = Event::new(event::Blur, &self.event_cache);
                self.node_mut().component.on_blur(&mut event);
                self.handle_dirty_event(&event);
            }
            Input::Focus(true) => {
                let mut event = Event::new(event::Focus, &self.event_cache);
                self.node_mut().component.on_focus(&mut event);
                self.handle_dirty_event(&event);
            }
            Input::Timer => {
                let mut event = Event::new(event::Tick, &self.event_cache);
                self.node_mut().tick(&mut event);
                self.handle_dirty_event(&event);
            }
            Input::MouseLeaveWindow => {
                if self.event_cache.mouse_over.is_some() {
                    let mut leave_event = Event::new(event::MouseLeave, &self.event_cache);
                    self.handle_event(
                        Node::mouse_leave,
                        &mut leave_event,
                        self.event_cache.mouse_over,
                    );
                }
                if self.event_cache.drag_button.is_some() {
                    let mut drag_end_event = Event::new(
                        event::DragEnd {
                            button: self.event_cache.drag_button.unwrap(),
                            start_pos: self.event_cache.drag_started.unwrap(),
                        },
                        &self.event_cache,
                    );
                    drag_end_event.target = self.event_cache.drag_target;

                    self.event_cache.drag_started = None;
                    self.event_cache.drag_button = None;

                    self.handle_event_without_focus(Node::drag_end, &mut drag_end_event, None);
                }
                self.event_cache.clear();
            }
            Input::MouseEnterWindow => (),
            Input::Drag(drag) => match drag {
                Drag::Start(data) => {
                    self.event_cache.drag_data.push(data.clone());
                }
                Drag::Dragging => {
                    let mut drag_event = Event::new(event::DragTarget, &self.event_cache);
                    self.handle_event_without_focus(Node::drag_target, &mut drag_event, None);

                    if drag_event.target != self.event_cache.drag_target {
                        if self.event_cache.drag_target.is_some() {
                            let mut leave_event = Event::new(event::DragLeave, &self.event_cache);
                            self.handle_event_without_focus(
                                Node::drag_leave,
                                &mut leave_event,
                                self.event_cache.drag_target,
                            );
                        }
                        if drag_event.target.is_some() {
                            let mut enter_event = Event::new(
                                event::DragEnter(self.event_cache.drag_data.clone()),
                                &self.event_cache,
                            );
                            self.handle_event_without_focus(
                                Node::drag_enter,
                                &mut enter_event,
                                drag_event.target,
                            );
                        }
                        self.event_cache.drag_target = drag_event.target;
                    }
                }
                Drag::End => {
                    if self.event_cache.drag_target.is_some() {
                        let mut leave_event = Event::new(event::DragLeave, &self.event_cache);
                        self.handle_event_without_focus(
                            Node::drag_leave,
                            &mut leave_event,
                            self.event_cache.drag_target,
                        );
                    }
                    self.event_cache.clear();
                }
                Drag::Drop(data) => {
                    let mut event = Event::new(event::DragDrop(data.clone()), &self.event_cache);
                    self.handle_event_without_focus(
                        Node::drag_drop,
                        &mut event,
                        self.event_cache.drag_target.or(Some(0)),
                    );
                    self.event_cache.clear();
                }
            },
            Input::Exit => {
                clear_current_window();
                let renderer = self.renderer.write().unwrap().take().unwrap();
                drop(renderer);
            }
            Input::Menu(id) => {
                let current_focus = self.event_cache.focus;
                let mut menu_event = Event::new(event::MenuSelect(*id), &self.event_cache);
                menu_event.target = immediate_focus().or(menu_event.focus);

                // If the event is focused on a non-root node
                if current_focus != self.node_ref().id {
                    // First see if the focused node will respond
                    self.handle_event_without_focus(Node::menu_select, &mut menu_event, None);

                    if menu_event.bubbles {
                        // See if the root node reacts to the menu event
                        self.node_mut().component.on_menu_select(&mut menu_event);
                        self.handle_dirty_event(&menu_event);
                        if !menu_event.messages.is_empty() {
                            // If so, first send the messages to the non-root node
                            if let Some(stack) =
                                self.node.read().unwrap().get_target_stack(current_focus)
                            {
                                self.node
                                    .write()
                                    .unwrap()
                                    .send_messages(stack, &mut menu_event.messages);
                            }
                        }
                    }
                } else {
                    // If it's the root node
                    self.node_mut().component.on_menu_select(&mut menu_event);
                    self.handle_dirty_event(&menu_event);
                    // Send the messages to the root update function,
                    // because that's where it should do its work
                    for message in menu_event.messages.drain(..) {
                        self.update(message);
                    }
                }
            }
        }
        clear_immediate_focus();
        inst_end();
    }

    pub fn add_font(&mut self, name: String, bytes: &'static [u8]) {
        self.font_cache.write().unwrap().add_font(name, bytes);
    }

    pub fn set_dirty(&mut self) {
        *self.node_dirty.write().unwrap() = true
    }

    pub fn update(&mut self, msg: crate::Message) {
        self.node_mut().component.update(msg);
    }

    pub fn with_app_state<S, F>(&mut self, f: F)
    where
        F: Fn(&mut S) -> (),
        S: 'static,
    {
        let mut node = self.node_mut();
        if let Some(mut state) = node.component.take_state() {
            if let Some(s) = state.as_mut().downcast_mut::<S>() {
                f(s);
            }
            node.component.replace_state(state);
        }
    }
}
