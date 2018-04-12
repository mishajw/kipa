//! Performs a graph search on a network of KIPA nodes.

use error::*;
use key::Key;
use node::Node;
use request_handler::graph::key_space::KeySpace;

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

type GetNeighboursFn = Arc<Fn(&Node, &Key) -> Result<Vec<Node>> + Send + Sync>;

/// Contains data for graph search
pub struct GraphSearch {
    get_neighbours_fn: GetNeighboursFn,
}

#[derive(PartialEq)]
struct SearchNode {
    node: Node,
    cost: f32,
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

impl GraphSearch {
    /// Create a new graph search with a function for retrieving the neighbours
    /// of the node.
    pub fn new(get_neighbours_fn: GetNeighboursFn) -> Self {
        GraphSearch {
            get_neighbours_fn: get_neighbours_fn,
        }
    }

    /// Search for a key using the `GetNeighboursFn`.
    pub fn search(
        &self,
        key: &Key,
        start_nodes: Vec<Node>,
    ) -> Result<Option<Node>> {
        info!("Starting graph search for key {}", key);

        let key_space = KeySpace::from_key(key, 2);
        // Create structures for the search.
        let mut to_explore = BinaryHeap::new();
        let mut explored: HashSet<Node> = HashSet::new();

        // Wrapper around `BinaryHeap::push` so that the node is wrapped in a
        // search node type.
        let insert = |heap: &mut BinaryHeap<SearchNode>, n: Node| {
            let cost = &KeySpace::from_key(&n.key, 2) - &key_space;
            heap.push(SearchNode {
                node: n,
                cost: cost,
            });
        };

        // Check if search key is in `start_nodes`.
        // If not, add to `to_explore`
        for n in start_nodes {
            if &n.key == key {
                trace!("Found key {} at {} in start nodes", key, n);
                return Ok(Some(n))
            }
            insert(&mut to_explore, n);
        }

        while let Some(next_node) = to_explore.pop() {
            trace!("Current node is {}", next_node.node);

            // Get the neighbours of the node
            let neighbours = (*self.get_neighbours_fn)(&next_node.node, key)?;

            for n in neighbours {
                // If we've seen it before, ignore it
                if explored.contains(&n) {
                    continue;
                }

                // If we've found the key, return the node
                if &n.key == key {
                    trace!("Found key {} at {}", key, n);
                    return Ok(Some(n));
                }

                // Otherwise, add it to the explore list
                insert(&mut to_explore, n.clone());
            }

            explored.insert(next_node.node);
        }

        trace!("Failed to find key {}", key);
        Ok(None)
    }
}
