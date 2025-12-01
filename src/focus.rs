use hashbrown::HashMap;

/// Tracks all focus contexts and their parent-child relationships
/// This is rebuilt during the view phase
#[derive(Debug, Default)]
pub struct FocusTree {
    /// Map from node_id -> parent focus context's node_id
    parents: HashMap<u64, u64>,

    /// Map from node_id -> priority (higher = handles events first among siblings)
    priorities: HashMap<u64, i32>,
}

impl FocusTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear the tree (called before rebuilding during view phase)
    pub fn clear(&mut self) {
        self.parents.clear();
        self.priorities.clear();
    }

    /// Register a focus context during view traversal
    /// `parent_focus_id` is the node_id of the nearest ancestor that also has focus
    pub fn register(&mut self, node_id: u64, parent_focus_id: u64, priority: i32) {
        self.parents.insert(node_id, parent_focus_id);
        self.priorities.insert(node_id, priority);
    }

    /// Check if a node is registered as a focus context
    pub fn contains(&self, node_id: u64) -> bool {
        self.parents.contains_key(&node_id)
    }

    /// Get parent focus context for a node
    pub fn parent(&self, node_id: u64) -> Option<u64> {
        self.parents.get(&node_id).copied()
    }

    /// Compute the path from root to the given node, including only focus contexts
    pub fn path_to(&self, node_id: u64) -> Vec<u64> {
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
    active: Option<u64>,

    /// The computed focus stack from root to active leaf
    /// Cached and recomputed when active changes
    stack: Vec<u64>,
}

impl FocusState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Access the tree for registration during view phase
    pub fn tree_mut(&mut self) -> &mut FocusTree {
        &mut self.tree
    }

    /// Set the active focus and recompute the stack
    pub fn set_active(&mut self, node_id: Option<u64>) {
        self.active = node_id;
        self.recompute_stack();
    }

    /// Get the currently active focus
    pub fn active(&self) -> Option<u64> {
        self.active
    }

    /// Get the focus stack root first
    pub fn stack(&self) -> &[u64] {
        &self.stack
    }

    fn recompute_stack(&mut self) {
        self.stack = match self.active {
            Some(id) => self.tree.path_to(id),
            None => Vec::new(),
        };
    }
}
