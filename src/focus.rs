use hashbrown::HashMap;

use crate::NodeId;

/// Tracks all focus contexts and their parent-child relationships
/// This is rebuilt during the view phase
#[derive(Debug, Default)]
pub struct FocusTree {
    /// Map from node_id -> parent focus context's node_id
    parents: HashMap<NodeId, NodeId>,

    /// Map from node_id -> priority (higher = handles events first among siblings)
    priorities: HashMap<NodeId, i32>,
}

impl FocusTree {
    /// Register a focus context during view traversal
    /// `parent_focus_id` is the node_id of the nearest ancestor that also has focus
    pub fn register(&mut self, node_id: NodeId, parent_focus_id: NodeId, priority: i32) {
        self.parents.insert(node_id, parent_focus_id);
        self.priorities.insert(node_id, priority);
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

    /// The node_id of the currently focused leaf element
    active: NodeId,

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

    pub fn inherit_active(&mut self, old_state: &Self) {
        self.stack = old_state.stack.clone();
        self.active = old_state.active;
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
            } else {
                if stack.is_empty() {
                    stack.push(root_id);
                }
                self.active = stack.last().cloned().unwrap();
                self.stack = stack;
            }
        }
    }

    /// Try to set the active focus, respecting priority rules.
    /// A focus change is allowed if:
    /// 1. The new node has equal or higher priority than the current node, OR
    /// 2. The current node is an ancestor of the new node (child can take focus from parent)
    ///
    /// Returns true if the focus was changed, false if it was blocked by priority rules.
    pub fn try_set_active(
        &mut self,
        node_id: Option<NodeId>,
        event_stack: &[NodeId],
        root_id: NodeId,
    ) -> bool {
        // If trying to set no focus, allow it
        if node_id.is_none() {
            self.set_active(node_id, event_stack, root_id);
            return true;
        }

        let new_node = node_id.unwrap();
        let current_node = self.active;

        // If it's the same node, no change needed
        if new_node == current_node {
            return false;
        }

        // Get priorities
        let new_priority = self.tree.priority(new_node).unwrap_or(0);
        let current_priority = self.tree.priority(current_node).unwrap_or(0);

        // Allow focus change if:
        // 1. New node has equal or higher priority, OR
        // 2. Current node is an ancestor of new node (descendant can take focus)
        let allow_change =
            new_priority >= current_priority || self.tree.is_ancestor_of(current_node, new_node);

        if allow_change {
            self.set_active(node_id, event_stack, root_id);
            true
        } else {
            false
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
