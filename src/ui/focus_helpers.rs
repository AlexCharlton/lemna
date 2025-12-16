//! Helper functions for focus change handling that can work without requiring
//! `&mut self` reference to the UI. This allows sharing focus logic between
//! the LemnaUI trait methods and the draw_thread.

extern crate alloc;
use alloc::{string::String, vec::Vec};

use hashbrown::{HashMap, HashSet};

use crate::event::{self, Event, EventCache, EventInput, Signal, Target};
use crate::focus::FocusState;
use crate::node::Node;
use crate::{Dirty, NodeId, Scalable};

/// Context for focus operations containing all the state needed.
/// This can be used by both the LemnaUI trait methods and the draw_thread.
pub struct FocusContext<'a> {
    pub node: &'a mut Node,
    pub focus_state: &'a mut FocusState,
    pub references: &'a HashMap<String, u64>,
    pub scale_factor: f32,
    pub dirty: Dirty,
}

impl<'a> FocusContext<'a> {
    pub fn new(
        node: &'a mut Node,
        focus_state: &'a mut FocusState,
        references: &'a HashMap<String, u64>,
        scale_factor: f32,
    ) -> Self {
        Self {
            node,
            focus_state,
            references,
            scale_factor,
            dirty: Dirty::No,
        }
    }

    /// Get the currently active focus
    pub fn active_focus(&self) -> NodeId {
        self.focus_state.active()
    }

    /// Get the focus stack
    pub fn focus_stack(&self) -> Vec<NodeId> {
        self.focus_state.stack().to_vec()
    }

    /// Set focus on a node
    pub fn set_focus(&mut self, focus: Option<NodeId>, event_stack: &[NodeId]) {
        self.focus_state
            .set_active(focus, event_stack, self.node.id);
    }

    /// Get a reference node id by name
    pub fn get_reference(&self, reference: &str) -> Option<NodeId> {
        self.references.get(reference).copied()
    }

    /// Send a blur event to the specified target node
    pub fn send_blur_event_to(
        &mut self,
        target: NodeId,
        focus_stack: Vec<NodeId>,
        suppress_scroll_to: bool,
        previously_focused_nodes: &mut HashSet<NodeId>,
    ) -> Event<event::Blur> {
        let event_cache = EventCache::new(self.scale_factor);
        let mut blur_event = Event::new(event::Blur, &event_cache, target);
        blur_event.set_focus_stack(focus_stack);
        blur_event.target = Some(target);
        self.node.blur(&mut blur_event);
        self.dirty += blur_event.dirty;
        self.handle_event_signals(&blur_event, previously_focused_nodes);
        if suppress_scroll_to {
            blur_event.suppress_scroll_to();
        }
        blur_event
    }

    /// Send a blur event to the currently focused node and clear focus
    pub fn send_blur_event(
        &mut self,
        suppress_scroll_to: bool,
        previously_focused_nodes: &mut HashSet<NodeId>,
    ) -> Event<event::Blur> {
        let focus = self.active_focus();
        let focus_stack = self.focus_stack();
        self.send_blur_event_to(
            focus,
            focus_stack,
            suppress_scroll_to,
            previously_focused_nodes,
        )
    }

    /// Send a focus event to the specified target node
    pub fn send_focus_event_to(
        &mut self,
        target: NodeId,
        focus_stack: Vec<NodeId>,
        suppress_scroll_to: bool,
        previously_focused_nodes: &mut HashSet<NodeId>,
    ) -> Event<event::Focus> {
        let event_cache = EventCache::new(self.scale_factor);
        let mut focus_event = Event::new(event::Focus, &event_cache, target);
        focus_event.set_focus_stack(focus_stack);
        focus_event.target = Some(target);
        self.node.set_focus(&mut focus_event);
        self.dirty += focus_event.dirty;

        if suppress_scroll_to {
            focus_event.suppress_scroll_to();
        }
        self.handle_event_signals(&focus_event, previously_focused_nodes);

        // Scroll to the focused node if it's not already in view
        if !focus_event.suppress_scroll_to {
            #[cfg(debug_assertions)]
            log::debug!(
                "Processing scroll to signal for node {:?} due to focus event",
                target
            );
            self.process_scroll_to_signal(target, self.scale_factor);
        }
        focus_event
    }

    /// Send a focus event to the currently focused node
    pub fn send_focus_event(
        &mut self,
        suppress_scroll_to: bool,
        previously_focused_nodes: &mut HashSet<NodeId>,
    ) {
        let focus = self.active_focus();
        let focus_stack = self.focus_stack();
        self.send_focus_event_to(
            focus,
            focus_stack,
            suppress_scroll_to,
            previously_focused_nodes,
        );
    }

    /// Handle signals from an event (like Signal::Focus)
    pub fn handle_event_signals<T: EventInput>(
        &mut self,
        event: &Event<T>,
        previously_focused_nodes: &mut HashSet<NodeId>,
    ) {
        for signal in event.signals.signals.iter() {
            let node_id = match signal {
                Signal::Focus(target) | Signal::ScrollTo(target) => match target {
                    Target::Ref(r) => self.get_reference(r),
                    Target::Child(_, node) => *node,
                },
            };
            if let Some(node_id) = node_id {
                match signal {
                    Signal::Focus(_) => {
                        if node_id != self.active_focus()
                            && !previously_focused_nodes.contains(&node_id)
                            && let Some(stack_to_node) = self.node.get_target_stack(node_id, false)
                        {
                            let stack: Vec<NodeId> =
                                stack_to_node.iter().map(|n| (*n) as u64).collect();

                            #[cfg(debug_assertions)]
                            log::debug!(
                                "Changing focus from {} to {} (stack: {:?}) due to signal from event: {:?}",
                                self.focus_state.active(),
                                node_id,
                                stack,
                                event
                            );
                            previously_focused_nodes.insert(node_id);
                            let blur_event = self.send_blur_event(
                                event.suppress_scroll_to,
                                previously_focused_nodes,
                            );

                            self.set_focus(Some(node_id), &stack);
                            self.send_focus_event(
                                blur_event.suppress_scroll_to,
                                previously_focused_nodes,
                            );
                        }
                    }
                    Signal::ScrollTo(_target) => {
                        #[cfg(debug_assertions)]
                        log::debug!(
                            "Processing scroll to signal for node {:?} due to event: {:?}",
                            node_id,
                            event
                        );
                        self.process_scroll_to_signal(node_id, event.scale_factor);
                    }
                }
            }
        }
    }

    /// Handle a focus change that occurred during view().
    /// This is the main entry point for handling focus changes in the draw thread.
    ///
    /// prev_focus: The FocusState before view() was called
    /// self.focus_state is the new FocusState after view() was called
    pub fn handle_focus_change(&mut self, prev_focus: &FocusState) {
        if self.focus_state.active() != prev_focus.active() {
            #[cfg(debug_assertions)]
            log::debug!(
                "Focus changed from {} (stack: {:?}) to {} (stack: {:?}) due to changes to the view tree",
                prev_focus.active(),
                prev_focus.stack_names(),
                self.focus_state.active(),
                self.focus_state.stack_names()
            );

            let mut previously_focused_nodes = HashSet::new();

            // Blur the previously focused node
            let blur_event = self.send_blur_event_to(
                prev_focus.active(),
                prev_focus.stack().to_vec(),
                false,
                &mut previously_focused_nodes,
            );

            // Focus the new node
            self.send_focus_event_to(
                self.focus_state.active(),
                self.focus_state.stack().to_vec(),
                blur_event.suppress_scroll_to,
                &mut previously_focused_nodes,
            );
        }
    }

    /// Process a single ScrollTo signal for the given node_id
    fn process_scroll_to_signal(&mut self, node_id: NodeId, scale_factor: f32) {
        // Get the stack of child indices leading to the target node
        if let Some(target_stack) = self.node.get_target_stack(node_id, true) {
            // First pass: collect info about target and scrollable ancestors
            let mut scroll_info = Vec::new();

            // Helper function to navigate to a node at a given depth (for reading ancestors)
            fn get_node_at_depth<'a>(root: &'a Node, stack: &[usize], depth: usize) -> &'a Node {
                let mut current = root;
                for &child_idx in stack[..depth].iter() {
                    current = &current.children[child_idx];
                }
                current
            }

            // Get target AABB using get_target_from_stack
            let target_node = self.node.get_target_from_stack(&target_stack);
            let target_aabb = target_node.aabb;

            // Walk up ancestors and collect scrollable ones
            for depth in (0..target_stack.len()).rev() {
                let ancestor = get_node_at_depth(self.node, &target_stack, depth);

                if ancestor.component.scroll_position().is_some() {
                    scroll_info.push((
                        depth,
                        // Physical dimension
                        ancestor.aabb,
                        // Scale to physical dimension
                        ancestor.inner_scale.map(|s| s.scale(scale_factor)),
                    ));
                }
            }

            // Second pass: mutate scrollable ancestors
            for (depth, ancestor_aabb, inner_scale) in scroll_info {
                // Navigate to the ancestor and call on_scroll_to
                let ancestor_stack = &target_stack[..depth];
                let ancestor = self.node.get_target_from_stack(ancestor_stack);
                if ancestor
                    .component
                    .on_scroll_to(target_aabb, ancestor_aabb, inner_scale)
                {
                    // Recalculate the position of the nodes
                    self.node.reposition(scale_factor)
                }
            }
        }
    }
}
