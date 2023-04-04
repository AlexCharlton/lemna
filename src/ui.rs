use std::cell::UnsafeCell;
use std::cell::{RefCell, RefMut};
use std::marker::PhantomData;
use std::rc::Rc;

use log::info;

use crate::base_types::*;
use crate::component::App;
use crate::event::{self, Event, EventCache};
use crate::font_cache::FontCache;
use crate::input::*;
use crate::instrumenting::*;
use crate::layout::*;
use crate::node::Node;
use crate::render::Renderer;
use crate::window::Window;

const DRAG_THRESHOLD: f32 = 5.0; // px

pub struct UI<W: Window, R: Renderer, A> {
    pub renderer: Option<R>,
    pub(crate) window: Rc<RefCell<W>>,
    node: Option<Node<R>>,
    phantom_app: PhantomData<A>,
    scale_factor: f32,
    physical_size: PixelSize,
    client_size: PixelSize,
    event_cache: EventCache,
    font_cache: FontCache,
    dirty: bool,
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
    static CURRENT_WINDOW: UnsafeCell<Option<Rc<RefCell<dyn Window>>>> = {
        UnsafeCell::new(None)
    }
);

pub fn current_window<'a>() -> Option<RefMut<'a, dyn Window>> {
    CURRENT_WINDOW.with(|r| unsafe {
        if let Some(w) = r.get().as_ref().unwrap() {
            Some(w.borrow_mut())
        } else {
            None
        }
    })
}

// TODO: Probably need this
// fn clear_current_window() {
//     CURRENT_WINDOW.with(|r| unsafe { *r.get().as_mut().unwrap() = None })
// }

pub fn set_current_window(window: Rc<RefCell<dyn Window>>) {
    CURRENT_WINDOW.with(|r| unsafe { *r.get().as_mut().unwrap() = Some(window) })
}

impl<W: 'static + Window, R: Renderer, A: 'static + App<R>> UI<W, R, A> {
    fn node_ref(&self) -> &Node<R> {
        self.node.as_ref().unwrap()
    }

    fn node_mut(&mut self) -> &mut Node<R> {
        self.node.as_mut().unwrap()
    }

    pub fn new(window: W) -> Self {
        let scale_factor = window.scale_factor();
        // dbg!(scale_factor);
        let physical_size = window.physical_size();
        let client_size = window.client_size();
        info!(
            "New window with physical size {:?} client size {:?} and scale factor {:?}",
            physical_size, client_size, scale_factor
        );
        inst("UI::new");
        let mut component = A::new();
        component.init();

        let renderer = Some(R::new(&window));
        let window = Rc::new(RefCell::new(window));
        set_current_window(window.clone());
        let event_cache = EventCache::new(scale_factor);

        let n = Self {
            renderer,
            window,
            node: Some(Node::new(Box::new(component), 0, Layout::default())),
            phantom_app: PhantomData,
            scale_factor,
            physical_size,
            client_size,
            event_cache,
            font_cache: FontCache {
                scale_factor,
                ..Default::default()
            },
            dirty: true,
        };
        inst_end();
        n
    }

    pub fn draw(&mut self) -> bool
    where
        <R as Renderer>::Renderable: std::fmt::Debug,
    {
        if !self.dirty {
            return false;
        }

        inst("UI::draw");
        let mut new = Node::new(
            Box::new(A::new()),
            0,
            lay!(
                size: size!(
                    self.client_size.width as f32,
                    self.client_size.height as f32
                )
            ),
        );

        inst("Node::view");
        new.view(self.node.as_mut());
        inst_end();

        inst("Node::layout");
        new.layout(self.node_ref(), &self.font_cache, self.scale_factor);
        inst_end();

        inst("Node::render");
        let do_render = new.render(
            self.renderer.as_mut().unwrap(),
            self.node.as_mut(),
            &self.font_cache,
            self.scale_factor,
        );
        inst_end();

        if do_render {
            self.node = Some(new);
            self.window.borrow().redraw();
        } else {
            self.node = Some(new);
        }

        self.dirty = false;
        inst_end();

        do_render
    }

    pub fn render(&mut self) {
        inst("UI::render");
        self.renderer.as_mut().unwrap().render(
            self.node.as_ref().unwrap(),
            self.physical_size,
            &self.font_cache,
        );
        // println!("rendered");
        inst_end();
    }

    fn blur(&mut self) {
        let mut blur_event = Event::new(event::Blur, &self.event_cache);
        blur_event.target = Some(self.event_cache.focus);
        self.node_mut().blur(&mut blur_event);
        self.handle_dirty_event(&blur_event);

        self.event_cache.focus = self.node.as_ref().unwrap().id; // The root note gets focus
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
            self.dirty = true
        }
    }

    pub fn handle_input(&mut self, input: &Input)
    where
        <R as Renderer>::Renderable: std::fmt::Debug,
    {
        inst("UI::handle_input");
        if self.node.is_none() || self.renderer.is_none() {
            // If there is no node, the event has happened after exiting
            // For some reason checking for both works better, even though they're unset at the same time?
            return;
        }
        match input {
            Input::Resize => {
                self.renderer
                    .as_mut()
                    .unwrap()
                    .resize(self.window.borrow().client_size());
                self.physical_size = self.window.borrow().physical_size();
                self.client_size = self.window.borrow().client_size();
                self.dirty = true;
                self.window.borrow().redraw(); // Always redraw after resizing
            }
            Input::Motion(Motion::Mouse { x, y }) => {
                let pos = Point::new(*x, *y);

                if let Some(button) = self.event_cache.mouse_button_held() {
                    if self.event_cache.drag_started.is_none() {
                        self.event_cache.drag_started = Some(self.event_cache.mouse_position);
                    }

                    let drag_start = self.event_cache.drag_started.unwrap();

                    if self.event_cache.drag_button.is_none()
                        && ((drag_start.x - pos.x).abs() > DRAG_THRESHOLD
                            || (drag_start.y - pos.y).abs() > DRAG_THRESHOLD)
                    {
                        self.event_cache.drag_button = Some(button);
                        let mut drag_start_event =
                            Event::new(event::DragStart(button), &self.event_cache);
                        drag_start_event.mouse_position = self.event_cache.drag_started.unwrap();
                        self.node_mut().drag_start(&mut drag_start_event);
                        self.event_cache.drag_target = drag_start_event.target;
                        self.handle_focus_or_blur(&drag_start_event);
                        self.handle_dirty_event(&drag_start_event);
                    }
                }

                self.event_cache.mouse_position = pos;
                let mut event = Event::new(event::MouseMotion, &self.event_cache);
                self.node_mut().mouse_motion(&mut event);
                self.handle_dirty_event(&event);

                let held_button = self.event_cache.mouse_button_held();
                if held_button.is_some() && self.event_cache.drag_button.is_some() {
                    let mut drag_event = Event::new(
                        event::Drag {
                            button: held_button.unwrap(),
                            start_pos: self.event_cache.drag_started.unwrap(),
                        },
                        &self.event_cache,
                    );
                    drag_event.target = self.event_cache.drag_target;
                    self.node_mut().drag(&mut drag_event);
                    self.handle_dirty_event(&drag_event);
                } else if event.target != self.event_cache.mouse_over {
                    if self.event_cache.mouse_over.is_some() {
                        let mut leave_event = Event::new(event::MouseLeave, &self.event_cache);
                        leave_event.target = self.event_cache.mouse_over;
                        self.node_mut().mouse_leave(&mut leave_event);
                        self.handle_focus_or_blur(&leave_event);
                        self.handle_dirty_event(&leave_event);
                    }
                    if event.target.is_some() {
                        let mut enter_event = Event::new(event::MouseEnter, &self.event_cache);
                        enter_event.target = event.target;
                        self.node_mut().mouse_enter(&mut enter_event);
                        self.handle_focus_or_blur(&enter_event);
                        self.handle_dirty_event(&enter_event);
                    }
                    self.event_cache.mouse_over = event.target;
                }
            }
            Input::Motion(Motion::Scroll { x, y }) => {
                let mut event = Event::new(event::Scroll { x: *x, y: *y }, &self.event_cache);
                self.node_mut().scroll(&mut event);
                self.handle_dirty_event(&event);
                // TODO change target?
            }
            Input::Press(Button::Mouse(b)) => {
                self.event_cache.mouse_down(*b);
                let mut event = Event::new(event::MouseDown(*b), &self.event_cache);
                self.node_mut().mouse_down(&mut event);
                self.handle_focus_or_blur(&event);
                self.handle_dirty_event(&event);
            }
            Input::Release(Button::Mouse(b)) => {
                let mut event = Event::new(event::MouseUp(*b), &self.event_cache);
                self.node_mut().mouse_up(&mut event);
                self.handle_focus_or_blur(&event);
                self.handle_dirty_event(&event);

                // End drag
                if Some(*b) == self.event_cache.drag_button {
                    let mut drag_end_event = Event::new(
                        event::DragEnd {
                            button: *b,
                            start_pos: self.event_cache.drag_started.unwrap(),
                        },
                        &self.event_cache,
                    );
                    drag_end_event.target = self.event_cache.drag_target;

                    self.event_cache.drag_started = None;
                    self.event_cache.drag_button = None;
                    self.event_cache.mouse_up(*b);

                    self.node_mut().drag_end(&mut drag_end_event);
                    self.handle_focus_or_blur(&drag_end_event);
                    self.handle_dirty_event(&drag_end_event);

                    // Unfocus when clicking a thing not focused
                    if drag_end_event.current_node_id != Some(self.event_cache.focus)
                    // Ignore the root node, which is the default focus
                        && self.event_cache.focus != self.node_ref().id
                    {
                        self.blur();
                    }
                } else
                // Resolve click
                if self.event_cache.is_mouse_button_held(*b) {
                    // TODO: Double clicks
                    self.event_cache.mouse_up(*b);
                    let mut event = Event::new(event::Click(*b), &self.event_cache);
                    self.node_mut().click(&mut event);
                    self.handle_focus_or_blur(&event);
                    self.handle_dirty_event(&event);

                    // Unfocus when clicking a thing not focused
                    if event.current_node_id != Some(self.event_cache.focus)
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
                event.target = event.focus;
                self.node_mut().key_down(&mut event);
                self.handle_focus_or_blur(&event);
                self.handle_dirty_event(&event);
            }
            Input::Release(Button::Keyboard(k)) => {
                if self.event_cache.key_held(*k) {
                    self.event_cache.key_up(*k);
                    let mut event = Event::new(event::KeyPress(*k), &self.event_cache);
                    event.target = event.focus;
                    self.node_mut().key_press(&mut event);
                    self.handle_focus_or_blur(&event);
                    self.handle_dirty_event(&event);
                }

                let mut event = Event::new(event::KeyUp(*k), &self.event_cache);
                event.target = event.focus;
                self.node_mut().key_up(&mut event);
                self.handle_focus_or_blur(&event);
                self.handle_dirty_event(&event);
            }
            Input::Text(s) => {
                let mut event = Event::new(event::TextEntry(s.clone()), &self.event_cache);
                event.target = event.focus;
                self.node_mut().text_entry(&mut event);
                self.handle_focus_or_blur(&event);
                self.handle_dirty_event(&event);
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
                    leave_event.target = self.event_cache.mouse_over;
                    self.node_mut().mouse_leave(&mut leave_event);
                    self.handle_focus_or_blur(&leave_event);
                    self.handle_dirty_event(&leave_event);
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

                    self.node_mut().drag_end(&mut drag_end_event);
                    self.handle_dirty_event(&drag_end_event);
                }
                self.event_cache.clear();
            }
            Input::MouseEnterWindow => (),
            Input::Redraw => (),
            Input::Exit => {
                // This prevents a hang when exiting on some backends
                self.renderer = None;
                self.node = None;
            }
            Input::Menu(id) => {
                let current_focus = self.event_cache.focus;
                let mut menu_event = Event::new(event::MenuSelect(*id), &self.event_cache);
                menu_event.target = immediate_focus().or(menu_event.focus);

                // If the event is focused on a non-root node
                if current_focus != self.node_ref().id {
                    // First see if the focused node will respond
                    self.node_mut().menu_select(&mut menu_event);
                    self.handle_dirty_event(&menu_event);
                    if menu_event.bubbles {
                        // See if the root node reacts to the menu event
                        let messages = self.node_mut().component.on_menu_select(&mut menu_event);
                        self.handle_dirty_event(&menu_event);
                        if !messages.is_empty() {
                            // If so, first send the messages to the non-root node
                            if let Some(stack) = self.node_ref().get_target_stack(current_focus) {
                                self.node_mut().send_messages(stack, messages);
                            }
                        }
                    }
                } else {
                    // If it's the root node
                    let mut messages = self.node_mut().component.on_menu_select(&mut menu_event);
                    self.handle_dirty_event(&menu_event);
                    // Send the messages to the root update function,
                    // because that's where it should do its work
                    for message in messages.drain(..) {
                        self.node_mut().component.update(message);
                    }
                }
            }
        }
        clear_immediate_focus();
        inst_end();
    }

    pub fn add_font(&mut self, name: String, bytes: &'static [u8]) {
        self.font_cache.add_font(name, bytes);
    }
}
