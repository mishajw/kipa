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

#[derive(Clone)]
struct SearchNode {
    node: Node,
    cost: f32,
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
        let mut found: HashSet<Key> = HashSet::new();

        let into_search_node = |n: Node| -> SearchNode {
            let cost = &KeySpace::from_key(&n.key, 2) - &key_space;
            SearchNode {
                node: n,
                cost: cost,
            }
        };

        // Check if search key is in `start_nodes`.
        // If not, add to `to_explore`
        for n in start_nodes {
            return_callback!(found_node_callback(&n)?);
            let search_node = into_search_node(n);
            found.insert(search_node.node.key.clone());
            to_explore.push(search_node);
        }

        while let Some(next_node) = to_explore.pop() {
            trace!(
                "found: {:?}",
                found
                    .iter()
                    .map(|k| k.get_key_id())
                    .collect::<Vec<&String>>()
            );
            trace!(
                "Current node is {}, have {} to explore",
                next_node.node,
                to_explore.len()
            );

            // Get the neighbours of the node
            let neighbours = (*self.get_neighbours_fn)(&next_node.node, key)?;
            trace!(
                "Found neighbours for node {}: {:?}",
                next_node.node,
                neighbours
                    .iter()
                    .map(|n| n.key.get_key_id())
                    .collect::<Vec<&String>>()
            );

            for n in neighbours {
                let search_node = into_search_node(n);

                // If we've seen it before, ignore it
                if found.contains(&search_node.node.key) {
                    trace!("seen {} before", search_node.node.key);
                    continue;
                }

                trace!("first encounter with {}", search_node.node.key);
                found.insert(search_node.node.key.clone());

                // Handle returning callback values
                return_callback!(found_node_callback(&search_node.node)?);
                // Otherwise, add it to the explore list
                to_explore.push(search_node);
            }

            return_callback!(explored_node_callback(&next_node.node)?);
        }

        trace!("Failed to find key {}", key);
        Ok(None)
    }
}
