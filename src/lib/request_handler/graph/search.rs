//! Performs a graph search on a network of KIPA nodes.

use error::*;
use key::Key;
use node::Node;
use request_handler::graph::key_space::KeySpace;

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

pub enum SearchCallbackReturn<T> {
    Continue(),
    Return(T),
    #[allow(unused)]
    Exit(),
}

type GetNeighboursFn = Arc<Fn(&Node, &Key) -> Result<Vec<Node>> + Send + Sync>;
type FoundNodeCallback<T> =
    Arc<Fn(&Node) -> Result<SearchCallbackReturn<T>> + Send + Sync>;
type ExploredNodeCallback<T> =
    Arc<Fn(&Node) -> Result<SearchCallbackReturn<T>> + Send + Sync>;

macro_rules! return_callback {
    ($callback_value:expr) => {
        match $callback_value {
            SearchCallbackReturn::Continue() => {},
            SearchCallbackReturn::Return(t) => return Ok(Some(t)),
            SearchCallbackReturn::Exit() => return Ok(None),
        }
    }
}

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
    pub fn search<T>(
        &self,
        key: &Key,
        start_nodes: Vec<Node>,
        found_node_callback: FoundNodeCallback<T>,
        explored_node_callback: ExploredNodeCallback<T>,
    ) -> Result<Option<T>> {
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
            return_callback!(found_node_callback(&n)?);
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

                // Handle returning callback values
                return_callback!(found_node_callback(&n)?);
                // Otherwise, add it to the explore list
                insert(&mut to_explore, n.clone());
            }

            return_callback!(explored_node_callback(&next_node.node)?);
            explored.insert(next_node.node);
        }

        trace!("Failed to find key {}", key);
        Ok(None)
    }
}
