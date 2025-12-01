extern crate alloc;
use alloc::{string::String, vec::Vec};

use crate::component::Component;
use crate::event::{self, Event, EventCache, EventInput};
use crate::input::*;
use crate::instrumenting::{inst, inst_end};
use crate::node::Node;
use crate::time::Instant;
use crate::{NodeId, base_types::*};

#[cfg(feature = "std")]
mod std_ui;
#[cfg(feature = "std")]
pub use std_ui::*;

#[cfg(not(feature = "std"))]
mod no_std_ui;
#[cfg(not(feature = "std"))]
pub use no_std_ui::*;

pub(crate) trait LemnaUI {
    fn draw(&mut self);

    fn render(&mut self);

    fn add_font(&mut self, name: String, bytes: &'static [u8]) -> Result<(), &'static str>;

    fn with_node<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Node) -> R;

    fn set_node_dirty(&mut self, dirty: bool);
    fn set_node_render_dirty(&mut self);

    fn event_cache(&mut self) -> &mut EventCache;
    fn resize(&mut self) {}
    fn exit(&mut self);

    fn focus_stack(&self) -> Vec<NodeId>;
    fn active_focus(&self) -> Option<NodeId>;
    fn set_focus(&mut self, focus: Option<NodeId>, event_stack: &[NodeId]);

    fn root_id(&mut self) -> NodeId {
        self.with_node(|node| node.id)
    }

    /// Calls [`Component#update`][Component#method.update] with `msg` on the root Node of the application. This will always trigger a redraw.
    fn update(&mut self, msg: crate::Message) {
        self.with_node(|node| node.component.update(msg));
        self.set_node_dirty(true);
    }

    /// Calls the equivalent of [`state_mut`][crate::state_component_impl] on the root Node of the application, and passes it as an arg to given closure `f`.
    fn state_mut<S, F>(&mut self, f: F)
    where
        F: Fn(&mut S),
        S: 'static,
    {
        let mut dirty = false;
        {
            self.with_node(|node| {
                if let Some(mut state) = node.component.take_state() {
                    if let Some(s) = state.as_mut().downcast_mut::<S>() {
                        f(s);
                    }
                    node.component.replace_state(state);
                    dirty = true;
                }
            });
        }
        self.set_node_dirty(dirty);
    }

    fn blur(&mut self, event_stack: &[NodeId]) {
        let focus = self.active_focus();
        let mut blur_event = Event::new(event::Blur, self.event_cache(), focus);
        blur_event.set_focus_stack(self.focus_stack());
        blur_event.target = focus;
        self.with_node(|node| node.blur(&mut blur_event));
        self.handle_dirty_event(&blur_event);

        // Blur means we're removing focus, pass None
        self.set_focus(None, event_stack)
    }

    fn handle_focus_or_blur<T: EventInput>(&mut self, event: &Event<T>) {
        if event.focus.is_none() {
            self.blur(&event.stack);
        } else if event.focus != self.active_focus() {
            self.set_focus(event.focus, &event.stack);
            let focus = self.active_focus();
            let mut focus_event = Event::new(event::Focus, self.event_cache(), focus);
            focus_event.set_focus_stack(self.focus_stack());
            focus_event.target = focus;
            self.with_node(|node| node.set_focus(&mut focus_event));
            self.handle_dirty_event(&focus_event);
        }
    }

    fn handle_dirty_event<T: EventInput>(&mut self, event: &Event<T>) {
        if event.dirty {
            self.set_node_dirty(true);
        } else if event.render_dirty {
            self.set_node_render_dirty();
        }
    }

    fn handle_event<T: EventInput, F>(
        &mut self,
        handler: F,
        event: &mut Event<T>,
        target: Option<u64>,
    ) where
        F: Fn(&mut Node, &mut Event<T>),
    {
        event.target = target;
        self.with_node(|node| handler(node, event));
        self.handle_focus_or_blur(event);
        self.handle_dirty_event(event);
    }

    fn handle_event_without_focus<T: EventInput, F>(
        &mut self,
        handler: F,
        event: &mut Event<T>,
        target: Option<u64>,
    ) where
        F: Fn(&mut Node, &mut Event<T>),
    {
        event.target = target;
        self.with_node(|node| handler(node, event));
        self.handle_dirty_event(event);
    }

    /// Handle [`Input`]s coming from the [`Window`] backend.
    fn handle_input(&mut self, input: &Input) {
        inst("UI::handle_input");
        // if self.node.is_none() || self.renderer.is_none() {
        //     // If there is no node, the event has happened after exiting
        //     // For some reason checking for both works better, even though they're unset at the same time?
        //     return;
        // }
        match input {
            Input::Resize => {
                self.resize();
            }
            Input::Motion(Motion::Mouse { x, y }) => {
                let pos = Point::new(*x, *y) * self.event_cache().scale_factor;

                if let Some(button) = self.event_cache().mouse_button_held() {
                    if self.event_cache().drag_started.is_none() {
                        self.event_cache().drag_started = Some(self.event_cache().mouse_position);
                    }

                    let drag_start = self.event_cache().drag_started.unwrap();

                    if self.event_cache().drag_button.is_none()
                        && ((drag_start.x - pos.x).abs() > event::DRAG_THRESHOLD
                            || (drag_start.y - pos.y).abs() > event::DRAG_THRESHOLD)
                    {
                        self.event_cache().drag_button = Some(button);
                        let focus = self.active_focus();
                        let mut drag_start_event =
                            Event::new(event::DragStart(button), self.event_cache(), focus);
                        drag_start_event.mouse_position = self.event_cache().drag_started.unwrap();
                        self.handle_event(Node::drag_start, &mut drag_start_event, None);
                        self.event_cache().drag_target = drag_start_event.target;
                    }
                }

                self.event_cache().mouse_position = pos;
                let focus = self.active_focus();
                let mut motion_event = Event::new(event::MouseMotion, self.event_cache(), focus);
                self.handle_event_without_focus(Node::mouse_motion, &mut motion_event, None);

                let held_button = self.event_cache().mouse_button_held();
                if held_button.is_some() && self.event_cache().drag_button.is_some() {
                    let mut drag_event = Event::new(
                        event::Drag {
                            button: held_button.unwrap(),
                            start_pos: self.event_cache().drag_started.unwrap(),
                        },
                        self.event_cache(),
                        focus,
                    );
                    let target = self.event_cache().drag_target;
                    self.handle_event_without_focus(Node::drag, &mut drag_event, target);
                } else if motion_event.target != self.event_cache().mouse_over {
                    if self.event_cache().mouse_over.is_some() {
                        let mut leave_event =
                            Event::new(event::MouseLeave, self.event_cache(), focus);
                        let target = self.event_cache().mouse_over;
                        self.handle_event(Node::mouse_leave, &mut leave_event, target);
                    }
                    if motion_event.target.is_some() {
                        let mut enter_event =
                            Event::new(event::MouseEnter, self.event_cache(), focus);
                        self.handle_event(Node::mouse_enter, &mut enter_event, motion_event.target);
                    }
                    self.event_cache().mouse_over = motion_event.target;
                }
            }
            Input::Motion(Motion::Scroll { x, y }) => {
                let focus = self.active_focus();
                let mut event = Event::new(
                    event::Scroll {
                        x: *x * self.event_cache().scale_factor,
                        y: *y * self.event_cache().scale_factor,
                    },
                    self.event_cache(),
                    focus,
                );
                self.handle_event_without_focus(Node::scroll, &mut event, None);
            }
            Input::Press(Button::Mouse(b)) => {
                self.event_cache().mouse_down(*b);
                let focus = self.active_focus();
                let mut event = Event::new(event::MouseDown(*b), self.event_cache(), focus);
                self.handle_event(Node::mouse_down, &mut event, None);
            }
            Input::Release(Button::Mouse(b)) => {
                let focus = self.active_focus();
                let mut event = Event::new(event::MouseUp(*b), self.event_cache(), focus);
                self.handle_event(Node::mouse_up, &mut event, None);

                let mut is_double_click = false;
                // Double clicking
                if b == &MouseButton::Left {
                    if self.event_cache().last_mouse_click.elapsed().as_millis()
                        < event::DOUBLE_CLICK_INTERVAL_MS
                        && self
                            .event_cache()
                            .last_mouse_click_position
                            .dist(self.event_cache().mouse_position)
                            < event::DOUBLE_CLICK_MAX_DIST
                    {
                        is_double_click = true;
                    }
                    self.event_cache().last_mouse_click = Instant::now();
                    self.event_cache().last_mouse_click_position =
                        self.event_cache().mouse_position;
                }

                // End drag
                if Some(*b) == self.event_cache().drag_button {
                    let drag_distance = self
                        .event_cache()
                        .drag_started
                        .unwrap()
                        .dist(self.event_cache().mouse_position);
                    if drag_distance < event::DRAG_CLICK_MAX_DIST {
                        // Send a Click event if the drag was quite short
                        // We send it before the drag end event, so that widgets can choose to ignore it
                        let focus = self.active_focus();
                        let mut click_event =
                            Event::new(event::Click(*b), self.event_cache(), focus);
                        self.handle_event(Node::click, &mut click_event, None);
                    }

                    let focus = self.active_focus();
                    let mut drag_end_event = Event::new(
                        event::DragEnd {
                            button: *b,
                            start_pos: self.event_cache().drag_started.unwrap(),
                        },
                        self.event_cache(),
                        focus,
                    );
                    let target = self.event_cache().drag_target;
                    self.handle_event(Node::drag_end, &mut drag_end_event, target);

                    // Unfocus when clicking a thing not focused
                    if drag_end_event.current_node_id != self.active_focus()
                    // Ignore the root node, which is the default focus
                        && self.active_focus() != Some(self.root_id())
                    {
                        self.blur(&drag_end_event.stack);
                    }

                // Clean up event cache
                } else if self.event_cache().is_mouse_button_held(*b) {
                    // Resolve click
                    let focus = self.active_focus();
                    let (event_current_node_id, event_stack) = if is_double_click {
                        let mut event =
                            Event::new(event::DoubleClick(*b), self.event_cache(), focus);
                        self.handle_event(Node::double_click, &mut event, None);
                        (event.current_node_id, event.stack)
                    } else {
                        let mut event = Event::new(event::Click(*b), self.event_cache(), focus);
                        self.handle_event(Node::click, &mut event, None);
                        (event.current_node_id, event.stack)
                    };

                    // Unfocus when clicking a thing not focused
                    if event_current_node_id != self.active_focus()
                        // Ignore the root node, which is the default focus
                            && self.active_focus() != Some(self.root_id())
                    {
                        self.blur(&event_stack);
                    }
                }
                // Clean up cache state
                self.event_cache().drag_started = None;
                self.event_cache().drag_button = None;
                self.event_cache().mouse_up(*b);
            }
            Input::Press(Button::Keyboard(k)) => {
                self.event_cache().key_down(*k);
                let focus = self.active_focus();
                let mut event = Event::new(event::KeyDown(*k), self.event_cache(), focus);
                event.set_focus_stack(self.focus_stack());
                self.handle_event(Node::key_down, &mut event, focus);
            }
            Input::Release(Button::Keyboard(k)) => {
                if self.event_cache().key_held(*k) {
                    self.event_cache().key_up(*k);
                    let focus = self.active_focus();
                    let mut event = Event::new(event::KeyPress(*k), self.event_cache(), focus);
                    event.set_focus_stack(self.focus_stack());
                    self.handle_event(Node::key_press, &mut event, focus);
                }

                let focus = self.active_focus();
                let mut event = Event::new(event::KeyUp(*k), self.event_cache(), focus);
                event.set_focus_stack(self.focus_stack());
                self.handle_event(Node::key_up, &mut event, focus);
            }
            Input::Text(s) => {
                let mods = self.event_cache().modifiers_held;
                if !mods.alt && !mods.ctrl && !mods.meta {
                    let focus = self.active_focus();
                    let mut event =
                        Event::new(event::TextEntry(s.clone()), self.event_cache(), focus);
                    event.set_focus_stack(self.focus_stack());
                    self.handle_event(Node::text_entry, &mut event, focus);
                }
            }
            Input::Focus(false) => {
                self.event_cache().clear();
                let focus = self.active_focus();
                let mut event = Event::new(event::Blur, self.event_cache(), focus);
                event.set_focus_stack(self.focus_stack());
                self.with_node(|node| node.component.on_blur(&mut event));
                self.handle_dirty_event(&event);
            }
            Input::Focus(true) => {
                let focus = self.active_focus();
                let mut event = Event::new(event::Focus, self.event_cache(), focus);
                event.set_focus_stack(self.focus_stack());
                self.with_node(|node| node.component.on_focus(&mut event));
                self.handle_dirty_event(&event);
            }
            Input::Timer => {
                let focus = self.active_focus();
                let mut event = Event::new(event::Tick, self.event_cache(), focus);
                self.with_node(|node| node.tick(&mut event));
                self.handle_dirty_event(&event);
            }
            Input::MouseLeaveWindow => {
                if self.event_cache().mouse_over.is_some() {
                    let focus = self.active_focus();
                    let mut leave_event = Event::new(event::MouseLeave, self.event_cache(), focus);
                    let target = self.event_cache().mouse_over;
                    self.handle_event(Node::mouse_leave, &mut leave_event, target);
                }
                if self.event_cache().drag_button.is_some() {
                    let focus = self.active_focus();
                    let mut drag_end_event = Event::new(
                        event::DragEnd {
                            button: self.event_cache().drag_button.unwrap(),
                            start_pos: self.event_cache().drag_started.unwrap(),
                        },
                        self.event_cache(),
                        focus,
                    );
                    drag_end_event.target = self.event_cache().drag_target;

                    self.event_cache().drag_started = None;
                    self.event_cache().drag_button = None;

                    self.handle_event_without_focus(Node::drag_end, &mut drag_end_event, None);
                }
                self.event_cache().clear();
            }
            Input::MouseEnterWindow => (),
            Input::Drag(drag) => match drag {
                Drag::Start(data) => {
                    self.event_cache().drag_data.push(data.clone());
                }
                Drag::Dragging => {
                    let focus = self.active_focus();
                    let mut drag_event = Event::new(event::DragTarget, self.event_cache(), focus);
                    self.handle_event_without_focus(Node::drag_target, &mut drag_event, None);

                    if drag_event.target != self.event_cache().drag_target {
                        if self.event_cache().drag_target.is_some() {
                            let mut leave_event =
                                Event::new(event::DragLeave, self.event_cache(), focus);
                            let target = self.event_cache().drag_target;
                            self.handle_event_without_focus(
                                Node::drag_leave,
                                &mut leave_event,
                                target,
                            );
                        }
                        if drag_event.target.is_some() {
                            let mut enter_event = Event::new(
                                event::DragEnter(self.event_cache().drag_data.clone()),
                                self.event_cache(),
                                focus,
                            );
                            self.handle_event_without_focus(
                                Node::drag_enter,
                                &mut enter_event,
                                drag_event.target,
                            );
                        }
                        self.event_cache().drag_target = drag_event.target;
                    }
                }
                Drag::End => {
                    if self.event_cache().drag_target.is_some() {
                        let focus = self.active_focus();
                        let mut leave_event =
                            Event::new(event::DragLeave, self.event_cache(), focus);
                        let target = self.event_cache().drag_target;
                        self.handle_event_without_focus(Node::drag_leave, &mut leave_event, target);
                    }
                    self.event_cache().clear();
                }
                Drag::Drop(data) => {
                    let focus = self.active_focus();
                    let mut event =
                        Event::new(event::DragDrop(data.clone()), self.event_cache(), focus);
                    let target = self.event_cache().drag_target.or(Some(self.root_id()));
                    self.handle_event_without_focus(Node::drag_drop, &mut event, target);
                    self.event_cache().clear();
                }
            },
            Input::Exit => {
                self.exit();
            }
        }
        inst_end();
    }
}

#[cfg(feature = "std")]
impl<A: Component + Default + Send + Sync + 'static> UI<A> {
    /// Signal to the draw thread that it may be time to draw a redraw the app.
    pub fn draw(&mut self) {
        LemnaUI::draw(self)
    }

    /// Signal to the render thread that it may be time to render a frame.
    pub fn render(&mut self) {
        LemnaUI::render(self)
    }

    /// Add a font to the font cache.
    pub fn add_font(&mut self, name: String, bytes: &'static [u8]) -> Result<(), &'static str> {
        LemnaUI::add_font(self, name, bytes)
    }

    /// Update the application with a message.
    pub fn update(&mut self, msg: crate::Message) {
        LemnaUI::update(self, msg)
    }

    /// Mutate application state.
    pub fn state_mut<S, F>(&mut self, f: F)
    where
        F: Fn(&mut S),
        S: 'static,
    {
        LemnaUI::state_mut(self, f)
    }

    /// Handle input events.
    pub fn handle_input(&mut self, input: &Input) {
        LemnaUI::handle_input(self, input)
    }
}

#[cfg(not(feature = "std"))]
impl<
    A: Component + Default + Send + Sync + 'static,
    D: embedded_graphics::draw_target::DrawTarget<Color = C, Error = E>,
    C: crate::render::RgbColor,
    E: core::fmt::Debug,
> UI<A, D, C, E>
{
    /// Signal to draw the app.
    pub fn draw(&mut self) {
        LemnaUI::draw(self)
    }

    /// Signal to render a frame.
    pub fn render(&mut self) {
        LemnaUI::render(self)
    }

    /// Add a font to the font cache.
    pub fn add_font(&mut self, name: String, bytes: &'static [u8]) -> Result<(), &'static str> {
        LemnaUI::add_font(self, name, bytes)
    }

    /// Update the application with a message.
    pub fn update(&mut self, msg: crate::Message) {
        LemnaUI::update(self, msg)
    }

    /// Mutate application state.
    pub fn state_mut<S, F>(&mut self, f: F)
    where
        F: Fn(&mut S),
        S: 'static,
    {
        LemnaUI::state_mut(self, f)
    }

    /// Handle input events.
    pub fn handle_input(&mut self, input: &Input) {
        LemnaUI::handle_input(self, input)
    }

    /// Send a signal to the application.
    pub fn signal(&mut self, msg: crate::Message, target: u64) {
        LemnaUI::signal(self, msg, target)
    }
}
