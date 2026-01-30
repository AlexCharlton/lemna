extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use hashbrown::{HashMap, HashSet};

use crate::{Node, NodeId};

/// Tracks all focus contexts and their parent-child relationships
/// This is rebuilt during the view phase
#[derive(Debug, Default)]
pub struct FocusTree {
    /// Map from node_id -> parent focus context's node_id
    parents: HashMap<NodeId, NodeId>,
    /// Map from node_id -> priority (higher = handles events first among siblings)
    priorities: HashMap<NodeId, i32>,

    #[cfg(debug_assertions)]
    names: HashMap<NodeId, String>,
}

impl FocusTree {
    /// Register a focus context during view traversal
    /// `parent_focus_id` is the node_id of the nearest ancestor that also has focus
    pub fn register(&mut self, node: &Node, parent_focus_id: NodeId) {
        self.parents.insert(node.id, parent_focus_id);
        let parent_priority = self.priority(parent_focus_id).unwrap_or(0);
        // Priority is inherited from the parent, plus the node's own priority
        self.priorities
            .insert(node.id, node.focus_priority + parent_priority);
        #[cfg(debug_assertions)]
        self.names
            .insert(node.id, node.component.name().to_string());
    }

    /// Check if a node is registered as a focus context
    pub fn contains(&self, node_id: NodeId) -> bool {
        self.parents.contains_key(&node_id)
    }

    /// Get the priority of a node
    pub fn priority(&self, node_id: NodeId) -> Option<i32> {
        self.priorities.get(&node_id).copied()
    }

    /// Check if node_a is an ancestor of node_b in the focus tree
    pub fn is_ancestor_of(&self, ancestor: NodeId, descendant: NodeId) -> bool {
        let mut current = Some(descendant);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.parents.get(&id).copied();
        }
        false
    }

    /// Compute the path from root to the given node, including only focus contexts
    pub fn path_to(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut path = Vec::new();
        let mut current = Some(node_id);

        while let Some(id) = current {
            path.push(id);
            if self.parents.contains_key(&id) {
                current = self.parents.get(&id).copied()
            } else {
                break;
            }
        }

        path.reverse(); // Root first, leaf last
        path
    }
}

/// The complete focus management state
#[derive(Debug, Default)]
pub struct FocusState {
    /// The tree of all focus contexts (rebuilt each view phase)
    tree: FocusTree,

    /// The node_id of the currently focused element
    active: NodeId,

    /// The node_id of the most specific focus context
    active_focus_context: Option<NodeId>,
    /// Was there a new node that requested focus?
    focus_from_new_node: Option<NodeId>,
    /// Was there an event that requested focus?
    focus_from_event: Option<NodeId>,

    /// The computed focus stack from root to active leaf
    /// Cached and recomputed when active changes
    stack: Vec<NodeId>,
}

impl FocusState {
    /// Access the tree for registration during view phase
    pub fn tree_mut(&mut self) -> &mut FocusTree {
        &mut self.tree
    }

    /// Check if a node is registered as a focus context
    pub fn tree_contains(&self, node_id: NodeId) -> bool {
        self.tree.contains(node_id)
    }

    /// Decide whether to inherit the active focus from the previous state, or to use the new focus from the new nodes. Order of inheritance is:
    /// 1. New node focus
    /// 2. Keep existing event-based focus if the node is still there
    /// 3. Use the active focus context if it has changed
    /// 4. Inherit the active focus from the previous state
    pub fn inherit_active(
        &mut self,
        old_state: &Self,
        all_new_nodes: &HashSet<NodeId>,
        root_id: NodeId,
    ) {
        log::debug!(
            "Inheriting active focus from previous state: {:?}\nstarting state: {:?}",
            old_state,
            self
        );
        if let Some(focus_from_new_node) = self.focus_from_new_node {
            // If the focus was from a new node, use that
            log::debug!(
                "Inheriting active focus from new node: {:?}",
                focus_from_new_node
            );
            self.active = focus_from_new_node;
            self.stack = self.tree.path_to(focus_from_new_node);
        } else if let Some(focus_from_event) = old_state.focus_from_event
            && all_new_nodes.contains(&focus_from_event)
        {
            // If the previous focus is from an event, and the node is still there, we can inherit the active focus
            log::debug!("Inheriting active focus from event: {:?}", focus_from_event);

            if self._inherit_active(old_state, all_new_nodes, root_id) {
                self.focus_from_event = None;
            } else {
                self.focus_from_event = Some(self.active);
            }
        } else if let Some(focus_from_context) = self.active_focus_context
            && (self.active_focus_context != old_state.active_focus_context
                || !all_new_nodes.contains(&old_state.active))
        {
            // If the active focus context has changed, or the previous active focus is not in the new nodes, use the focus context
            log::debug!(
                "Inheriting active focus from active focus context: {:?}",
                focus_from_context
            );
            self.active = focus_from_context;
            self.stack = self.tree.path_to(focus_from_context);
        } else {
            // Inherit the active focus from the previous state
            log::debug!("Inheriting active focus from previous state");
            self._inherit_active(old_state, all_new_nodes, root_id);
        }
    }

    // Returns whether the active focus was changed
    fn _inherit_active(
        &mut self,
        old_state: &Self,
        all_new_nodes: &HashSet<NodeId>,
        root_id: NodeId,
    ) -> bool {
        self.stack = old_state.stack.clone();
        self.active = old_state.active;
        let mut nodes_to_remove = 0;
        for node in self.stack.iter().rev() {
            if !all_new_nodes.contains(node) {
                nodes_to_remove += 1;
            }
        }
        self.stack = self.stack[..self.stack.len() - nodes_to_remove].to_vec();
        if self.stack.is_empty() {
            self.stack = vec![root_id];
        }
        if !all_new_nodes.contains(&self.active) {
            self.active = self
                .stack
                .last()
                .cloned()
                .expect("The stack should always contain at least the root node");
            true
        } else {
            false
        }
    }

    pub fn focus_new_node(&mut self, node_id: NodeId) {
        self.focus_from_new_node = Some(node_id);
    }

    /// Set the active focus and compute the stack based on the event stack
    /// If event_stack is provided and not empty, finds the most specific registered node in it
    /// and builds a stack ending with the explicitly focused node (even if not registered)
    /// If event_stack is None or empty, uses root_id as fallback
    pub fn set_active(&mut self, node_id: Option<NodeId>, event_stack: &[NodeId], root_id: NodeId) {
        if let Some(focused_node) = node_id
            && self.tree_contains(focused_node)
        {
            self.stack = self.tree.path_to(focused_node);
            self.active = focused_node;
            self.focus_from_event = Some(focused_node);
        } else {
            let event_stack = if event_stack.is_empty() {
                &[root_id]
            } else {
                event_stack
            };
            let base_focused_node = self.most_specific_focus_node(event_stack);
            let mut stack = self.tree.path_to(base_focused_node);
            if let Some(focused_node) = node_id {
                stack.push(focused_node);
                self.stack = stack;
                self.active = focused_node;
                self.focus_from_event = Some(focused_node);
            } else {
                if stack.is_empty() {
                    stack.push(root_id);
                }
                self.active = stack.last().cloned().unwrap();
                self.stack = stack;
                self.focus_from_event = None;
            }
        }
    }

    pub fn try_set_active_context(&mut self, new_node: NodeId) {
        if let Some(current_node) = self.active_focus_context {
            // If it's the same node, no change needed
            if new_node == current_node {
                return;
            }

            // Get priorities
            let new_priority = self.tree.priority(new_node).unwrap_or(0);
            let current_priority = self.tree.priority(current_node).unwrap_or(0);

            // Allow focus change if:
            // 1. New node has equal or higher priority, OR
            // 2. Current node is an ancestor of new node (descendant can take focus)
            let allow_change = new_priority >= current_priority
                || self.tree.is_ancestor_of(current_node, new_node);

            if allow_change {
                self.active_focus_context = Some(new_node);
            }
        } else {
            // No current focus context, so just set the new one
            self.active_focus_context = Some(new_node);
        }
    }

    /// Get the currently active focus
    pub fn active(&self) -> NodeId {
        self.active
    }

    /// Get the focus stack root first
    pub fn stack(&self) -> &[NodeId] {
        &self.stack
    }

    #[cfg(debug_assertions)]
    pub fn stack_names(&self) -> Vec<(NodeId, String)> {
        self.stack
            .iter()
            .map(|node_id| {
                (
                    *node_id,
                    self.tree
                        .names
                        .get(node_id)
                        .cloned()
                        .unwrap_or("unknown".to_string()),
                )
            })
            .collect()
    }

    /// Find the most specific (deepest) node in the event stack that is registered as a focus context
    /// Searches from the end of the stack (most specific) to the beginning (least specific)
    pub fn most_specific_focus_node(&self, event_stack: &[NodeId]) -> NodeId {
        // Iterate from most specific (end) to least specific (beginning)
        for &node_id in event_stack.iter().rev() {
            if self.tree.contains(node_id) {
                return node_id;
            }
        }
        event_stack
            .first()
            .cloned()
            .expect("Event stack should always have at least root element")
    }
}
