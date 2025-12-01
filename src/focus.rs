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
