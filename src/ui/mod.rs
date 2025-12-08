extern crate alloc;
use alloc::{string::String, vec::Vec};

use hashbrown::HashSet;

use crate::component::Component;
use crate::event::{self, Event, EventCache, EventInput};
use crate::instrumenting::{inst, inst_end};
use crate::node::Node;
use crate::time::Instant;
use crate::{Dirty, NodeId, base_types::*, input::*};

mod focus_helpers;
use focus_helpers::FocusContext;

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

    fn set_node_dirty(&mut self, dirty: Dirty);

    fn event_cache(&mut self) -> &mut EventCache;
    fn resize(&mut self) {}
    fn exit(&mut self);

    fn focus_stack(&self) -> Vec<NodeId>;
    fn active_focus(&self) -> NodeId;
    fn set_focus(&mut self, focus: Option<NodeId>, event_stack: &[NodeId]);

    #[allow(dead_code)]
    fn get_reference(&self, reference: &str) -> Option<NodeId>;

    /// Execute a closure with a FocusContext, which provides access to focus-related operations.
    /// This is the bridge between the trait-based API and the FocusContext helper.
    fn with_focus_context<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut FocusContext) -> R;

    fn root_id(&mut self) -> NodeId {
        self.with_node(|node| node.id)
    }

    /// Calls [`Component#update`][Component#method.update] with `msg` on the root Node of the application. This will always trigger a redraw.
    fn update(&mut self, msg: crate::Message) {
        self.with_node(|node| node.component.update(msg));
        self.set_node_dirty(Dirty::Full);
    }

    /// Calls the equivalent of [`state_mut`][crate::state_component_impl] on the root Node of the application, and passes it as an arg to given closure `f`.
    fn state_mut<S, F>(&mut self, f: F)
    where
        F: Fn(&mut S),
        S: 'static,
    {
        let mut dirty = Dirty::No;
        {
            self.with_node(|node| {
                if let Some(mut state) = node.component.take_state() {
                    if let Some(s) = state.as_mut().downcast_mut::<S>() {
                        f(s);
                    }
                    node.component.replace_state(state);
                    dirty = Dirty::Full;
                }
            });
        }
        self.set_node_dirty(dirty);
    }

    fn send_blur_event(
        &mut self,
        event_stack: &[NodeId],
        suppress_scroll_to: bool,
        previously_focused_nodes: &mut HashSet<NodeId>,
    ) -> Event<event::Blur> {
        self.with_focus_context(|ctx| {
            ctx.send_blur_event(event_stack, suppress_scroll_to, previously_focused_nodes)
        })
    }

    fn send_focus_event(
        &mut self,
        suppress_scroll_to: bool,
        previously_focused_nodes: &mut HashSet<NodeId>,
    ) {
        self.with_focus_context(|ctx| {
            ctx.send_focus_event(suppress_scroll_to, previously_focused_nodes);
        });
    }

    fn blur(&mut self, event_stack: &[NodeId], suppress_scroll_to: bool) {
        let mut previously_focused_nodes = HashSet::new();
        self.do_blur(
            event_stack,
            suppress_scroll_to,
            &mut previously_focused_nodes,
        );
    }

    fn do_blur(
        &mut self,
        event_stack: &[NodeId],
        suppress_scroll_to: bool,
        previously_focused_nodes: &mut HashSet<NodeId>,
    ) {
        let prev_focus = self.active_focus();
        let blur_event =
            self.send_blur_event(event_stack, suppress_scroll_to, previously_focused_nodes);
        // Blur means we're removing focus, pass None, and remove the last element from the event stack
        self.set_focus(None, &event_stack[..event_stack.len() - 1]);

        let new_focus = self.active_focus();
        // We've passed focus to some new Node, so focus it
        if new_focus != prev_focus {
            self.send_focus_event(blur_event.suppress_scroll_to, previously_focused_nodes);
        }
    }

    fn handle_focus_or_blur<T: EventInput>(
        &mut self,
        event: &Event<T>,
        previously_focused_nodes: &mut HashSet<NodeId>,
    ) {
        if event.focus.is_none() {
            self.do_blur(
                &event.stack,
                event.suppress_scroll_to,
                previously_focused_nodes,
            );
        } else if event.focus != Some(self.active_focus()) {
            // First blur the old focus
            let blur_event = self.send_blur_event(
                &event.stack,
                event.suppress_scroll_to,
                previously_focused_nodes,
            );

            // Then set the new focus
            let focus_node = event.focus.unwrap_or(self.root_id());
            self.set_focus(Some(focus_node), &event.stack);
            self.send_focus_event(blur_event.suppress_scroll_to, previously_focused_nodes);
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
        let mut previously_focused_nodes = HashSet::new();
        self.handle_focus_or_blur(event, &mut previously_focused_nodes);
        self.set_node_dirty(event.dirty);
        self.handle_event_signals(event, &mut previously_focused_nodes);
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
        self.set_node_dirty(event.dirty);
        let mut previously_focused_nodes = HashSet::new();
        self.handle_event_signals(event, &mut previously_focused_nodes);
    }

    // We need to track previously_focused_nodes so that we don't wind up in an infinite loop
    fn handle_event_signals<T: EventInput>(
        &mut self,
        event: &Event<T>,
        previously_focused_nodes: &mut HashSet<NodeId>,
    ) {
        self.with_focus_context(|ctx| {
            ctx.handle_event_signals(event, previously_focused_nodes);
        });
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
                            Event::new(event::DragStart { button }, self.event_cache(), focus);
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
                let mut event =
                    Event::new(event::MouseDown { button: *b }, self.event_cache(), focus);
                self.handle_event(Node::mouse_down, &mut event, None);
            }
            Input::Release(Button::Mouse(b)) => {
                let focus = self.active_focus();
                let mut event =
                    Event::new(event::MouseUp { button: *b }, self.event_cache(), focus);
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
                            Event::new(event::Click { button: *b }, self.event_cache(), focus);
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
                    if drag_end_event.current_node_id != Some(self.active_focus())
                    // Ignore the root node, which is the default focus
                        && self.active_focus() != self.root_id()
                    {
                        // We don't want this to cause a scroll_to
                        self.blur(&drag_end_event.stack, true);
                    }

                // Clean up event cache
                } else if self.event_cache().is_mouse_button_held(*b) {
                    // Resolve click
                    let focus = self.active_focus();
                    let (event_current_node_id, event_stack) = if is_double_click {
                        let mut event = Event::new(
                            event::DoubleClick { button: *b },
                            self.event_cache(),
                            focus,
                        );
                        self.handle_event(Node::double_click, &mut event, None);
                        (event.current_node_id, event.stack)
                    } else {
                        let mut event =
                            Event::new(event::Click { button: *b }, self.event_cache(), focus);
                        self.handle_event(Node::click, &mut event, None);
                        (event.current_node_id, event.stack)
                    };

                    // Unfocus when clicking a thing not focused
                    if event_current_node_id != Some(self.active_focus())
                        // Ignore the root node, which is the default focus
                            && self.active_focus() != self.root_id()
                    {
                        // We don't want this to cause a scroll_to
                        self.blur(&event_stack, true);
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
                let mut event = Event::new(event::KeyDown { key: *k }, self.event_cache(), focus);
                event.set_focus_stack(self.focus_stack());
                self.handle_event(Node::key_down, &mut event, Some(focus));
            }
            Input::Release(Button::Keyboard(k)) => {
                if self.event_cache().key_held(*k) {
                    self.event_cache().key_up(*k);
                    let focus = self.active_focus();
                    let mut event =
                        Event::new(event::KeyPress { key: *k }, self.event_cache(), focus);
                    event.set_focus_stack(self.focus_stack());
                    self.handle_event(Node::key_press, &mut event, Some(focus));
                }

                let focus = self.active_focus();
                let mut event = Event::new(event::KeyUp { key: *k }, self.event_cache(), focus);
                event.set_focus_stack(self.focus_stack());
                self.handle_event(Node::key_up, &mut event, Some(focus));
            }
            Input::Text(s) => {
                let mods = self.event_cache().modifiers_held;
                if !mods.alt && !mods.ctrl && !mods.meta {
                    let focus = self.active_focus();
                    let mut event = Event::new(
                        event::TextEntry { text: s.clone() },
                        self.event_cache(),
                        focus,
                    );
                    event.set_focus_stack(self.focus_stack());
                    self.handle_event(Node::text_entry, &mut event, Some(focus));
                }
            }
            Input::Focus(false) => {
                self.event_cache().clear();
                let focus = self.active_focus();
                let mut event = Event::new(event::Blur, self.event_cache(), focus);
                event.set_focus_stack(self.focus_stack());
                self.with_node(|node| node.component.on_blur(&mut event));
                self.set_node_dirty(event.dirty);
            }
            Input::Focus(true) => {
                let focus = self.active_focus();
                let mut event = Event::new(event::Focus, self.event_cache(), focus);
                event.set_focus_stack(self.focus_stack());
                self.with_node(|node| node.component.on_focus(&mut event));
                self.set_node_dirty(event.dirty);
            }
            Input::Timer => {
                let focus = self.active_focus();
                let mut event = Event::new(event::Tick, self.event_cache(), focus);
                self.with_node(|node| node.tick(&mut event));
                self.set_node_dirty(event.dirty);
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
                                event::DragEnter {
                                    data: self.event_cache().drag_data.clone(),
                                },
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
                    let mut event = Event::new(
                        event::DragDrop { data: data.clone() },
                        self.event_cache(),
                        focus,
                    );
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
}
