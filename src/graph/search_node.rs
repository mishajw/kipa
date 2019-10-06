use api::Node;

use std::cmp::Ordering;

/// Node with a search cost.
///
/// Implemented as a struct to have different ordering and equality functions.
#[derive(Clone)]
pub struct SearchNode {
    pub node: Node,
    pub cost: f32,
}

impl PartialEq for SearchNode {
    fn eq(&self, other: &SearchNode) -> bool {
        self.node.key == other.node.key
    }
}

impl Eq for SearchNode {}

impl Ord for SearchNode {
    fn cmp(&self, other: &SearchNode) -> Ordering {
        self.cost.partial_cmp(&other.cost).unwrap()
    }
}

impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &SearchNode) -> Option<Ordering> {
        self.cost.partial_cmp(&other.cost)
    }
}
