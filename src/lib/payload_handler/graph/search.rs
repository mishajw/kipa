//! Performs a graph search on a network of KIPA nodes.

use error::*;
use key::Key;
use node::Node;
use payload_handler::graph::key_space::KeySpaceManager;

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

use slog::Logger;

pub enum SearchCallbackReturn<T> {
    Continue(),
    Return(T),
    #[allow(unused)]
    Exit(),
}

pub type GetNeighboursFn =
    Arc<Fn(&Node, &Key) -> Result<Vec<Node>> + Send + Sync>;
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
    key_space_manager: Arc<KeySpaceManager>,
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
    pub fn new(key_space_manager: Arc<KeySpaceManager>) -> Self {
        GraphSearch {
            key_space_manager: key_space_manager,
        }
    }

    /// Search for a key using the `GetNeighboursFn`.
    pub fn search<T>(
        &self,
        key: &Key,
        start_nodes: Vec<Node>,
        get_neighbours_fn: GetNeighboursFn,
        found_node_callback: FoundNodeCallback<T>,
        explored_node_callback: ExploredNodeCallback<T>,
        log: Logger,
    ) -> Result<Option<T>> {
        info!(log, "Starting graph search"; "key" => %key);

        let key_space = self.key_space_manager.create_from_key(key);
        // Create structures for the search.
        let mut to_explore = BinaryHeap::new();
        let mut found: HashSet<Key> = HashSet::new();

        let into_search_node = |n: Node| -> SearchNode {
            let cost = self.key_space_manager.distance(
                &self.key_space_manager.create_from_key(&n.key),
                &key_space,
            );
            SearchNode {
                node: n,
                cost: -cost,
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
                log,
                "Search loop iteration";
                "current_node" => %next_node.node,
                "current_cost" => next_node.cost,
                "previously_found" => found
                    .iter()
                    .map(|k| k.get_key_id().clone())
                    .collect::<Vec<String>>()
                    .join(", "),
                "left_to_explore" => to_explore.len()
            );

            // Get the neighbours of the node
            let neighbours = (*get_neighbours_fn)(&next_node.node, key)?;
            trace!(
                log,
                "Found neighbours for node";
                "found" => true,
                "node" => %next_node.node,
                "neighbours" => neighbours
                    .iter()
                    .map(|n| n.key.get_key_id().clone())
                    .collect::<Vec<String>>()
                    .join(", ")
            );

            for n in neighbours {
                let search_node = into_search_node(n);

                // If we've seen it before, ignore it
                if found.contains(&search_node.node.key) {
                    trace!(
                        log,
                        "Seen before";
                        "node" => %search_node.node.key);
                    continue;
                }

                trace!(
                    log,
                    "First encounter";
                    "node" => %search_node.node.key);
                found.insert(search_node.node.key.clone());

                // Handle returning callback values
                return_callback!(found_node_callback(&search_node.node)?);
                // Otherwise, add it to the explore list
                to_explore.push(search_node);
            }

            return_callback!(explored_node_callback(&next_node.node)?);
        }

        info!(log, "Failed to find key"; "key" => %key);
        Ok(None)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use address::Address;
    use key::Key;

    use slog;
    use std::sync::Mutex;
    use spectral::assert_that;

    #[test]
    fn test_search_order() {
        let test_log = Logger::root(slog::Discard, o!());
        const NUM_NODES: usize = 100;
        const START_INDEX: usize = 50;

        let nodes = (0..NUM_NODES)
            .map(|i| {
                Node::new(
                    Address::new(vec![0, 0, 0, i as u8], i as u16),
                    Key::new(format!("{:08}", i), vec![i as u8]),
                )
            })
            .collect::<Vec<_>>();
        let nodes = Arc::new(nodes);

        let search = GraphSearch::new(Arc::new(KeySpaceManager::new(1)));
        let explored_nodes = Arc::new(Mutex::new(vec![]));

        let search_nodes = nodes.clone();
        let search_explored_nodes = explored_nodes.clone();
        search
            .search::<()>(
                &nodes[0].key,
                vec![
                    nodes[START_INDEX].clone(),
                    nodes[START_INDEX + 1].clone(),
                ],
                Arc::new(move |n, _k| {
                    let node_index = n.address.port as usize;
                    let neighbours: Vec<Node> =
                        if node_index > 0 && node_index < NUM_NODES - 1 {
                            vec![
                                search_nodes[node_index - 1].clone(),
                                search_nodes[node_index + 1].clone(),
                            ]
                        } else if node_index <= 0 {
                            vec![search_nodes[node_index + 1].clone()]
                        } else if node_index >= NUM_NODES - 1 {
                            vec![search_nodes[node_index - 1].clone()]
                        } else {
                            vec![]
                        };
                    Ok(neighbours)
                }),
                Arc::new(|_n| Ok(SearchCallbackReturn::Continue())),
                Arc::new(move |n| {
                    search_explored_nodes.lock().unwrap().push(n.clone());
                    Ok(SearchCallbackReturn::Continue())
                }),
                test_log,
            )
            .unwrap();

        let found_indices: Vec<usize> = explored_nodes
            .lock()
            .unwrap()
            .iter()
            .map(|n| n.address.port as usize)
            .collect();
        let mut expected_found: Vec<usize> =
            (0..START_INDEX + 1).rev().collect();
        expected_found
            .extend((START_INDEX + 1..NUM_NODES).collect::<Vec<usize>>());
        assert_that!(expected_found).is_equal_to(found_indices);
    }
}
