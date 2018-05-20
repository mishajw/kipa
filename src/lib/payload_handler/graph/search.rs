//! Performs a graph search on a network of KIPA nodes.

use error::*;
use key::Key;
use node::Node;
use payload_handler::graph::key_space::KeySpaceManager;

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::sync::{Arc, Mutex, mpsc::{channel, Receiver}};
use std::thread;
use std::time::Duration;

use error_chain::ChainedError;
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
            SearchCallbackReturn::Continue() => {}
            SearchCallbackReturn::Return(t) => return Ok(Some(t)),
            SearchCallbackReturn::Exit() => return Ok(None),
        }
    };
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

    /// Search for a key through looking up the neighbours of nodes in the KIPA
    /// network.
    ///
    /// Simple heuristic-based greedy-first search (GFS), where the heuristic is
    /// the distance in key space, provided by `GraphSearch::key_space_manager`.
    ///
    /// Differs from normal GFS as neighbour queries are done in parallel, with
    /// some maximum (TODO: Defined by which variable?) amount of queries
    /// running simultaneously. All querying threads push results into a
    /// thread-safe priority queue. This means that the GFS is impacted by which
    /// one of the queries resolve first.
    ///
    /// The other key difference is that the search does not exit when we find
    /// the correct node - we only exit when the `found_node_callback` or the
    /// `explored_node_callback` functions return
    /// `SearchCallbackReturn::{Return(...),Exit}`
    pub fn search<T: 'static>(
        &self,
        key: &Key,
        start_nodes: Vec<Node>,
        get_neighbours_fn: GetNeighboursFn,
        found_node_callback: FoundNodeCallback<T>,
        explored_node_callback: ExploredNodeCallback<T>,
        max_num_active_threads: usize,
        timeout_sec: usize,
        log: Logger,
    ) -> Result<Option<T>> {
        // Algorithm outline:
        // 1. Set up:
        //   a. Set `to_explore` to contain initial node(s)
        //   b. Set `found` to empty
        //   d. Set up `explored_channel` for communicating nodes explored/found
        //      by threads
        // 3. Consume from `explored_channel` until empty, each explored/found
        //    node is passed to `{explored,found}_node_callback` with option to
        //    exit the search
        // 4. Check conditions:
        //   a. If `num_threads == 0 && to_explore.empty()`, then exit
        //   b. If `num_threads > 0 && to_explore.empty()`, then wait for thread
        //      to finish and then go to (2)
        //   c. If `num_threads >= max_threads`, then wait for threads to finish
        //      and then go to (2)
        //   d. If `num_threads < max_threads`, then continue
        // 5. Pop node off `to_explore`, prioritized by key space distance
        // 7. Spawn thread for exploring popped node, which does:
        //   a. Ask node for neighbours
        //   b. Send node explored and found nodes down `explored_channel`
        // 8. Go to (2)

        info!(log, "Starting graph search"; "key" => %key);

        let key_space = self.key_space_manager.create_from_key(key);
        // Double the timeout for waiting for threads to resolve to allow some
        // slack in threads responding
        let timeout = Duration::from_secs((timeout_sec * 2) as u64);

        // Create structures for the search.
        let mut to_explore = BinaryHeap::new();
        let mut found: HashSet<Key> = HashSet::new();

        // Set up channels for returning results from spawned threads
        let (explored_channel_tx, explored_channel_rx) =
            channel::<(Node, Result<Vec<Node>>)>();

        // Counter of active threads
        let mut num_active_threads = 0 as usize;

        // Cast a `Node` into a `SearchNode` so it can be compared in
        // `to_explore`
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

        let wait_explored_channel_tx = explored_channel_tx.clone();
        let wait_for_threads =
            |rx: &Receiver<(Node, Result<Vec<Node>>)>| -> Result<()> {
                // Wait for `recv` to resolve
                let recv = rx.recv_timeout(timeout)
                    .chain_err(|| "Error on `recv` when waiting for threads")?;
                // And send it back down the channel
                wait_explored_channel_tx
                    .send(recv)
                    .chain_err(|| "Error on `send` when waiting for threads")?;

                Ok(())
            };

        // Add all nodes in `start_nodes` into `found` and `to_explore`, while
        // calling the `found_node_callback`
        for n in start_nodes {
            return_callback!(found_node_callback(&n)?);
            let search_node = into_search_node(n);
            found.insert(search_node.node.key.clone());
            to_explore.push(search_node);
        }

        loop {
            // Pop everything we can off the channel and into `found` and
            // `to_explore`
            while let Ok((explored_node, found_nodes)) =
                explored_channel_rx.try_recv()
            {
                // If we pop something off the channel, a thread has finished
                num_active_threads -= 1;

                // Check all found nodes
                for found_node in found_nodes.unwrap_or(vec![]) {
                    let search_node = into_search_node(found_node);

                    // If we've seen it before, ignore it
                    if found.contains(&search_node.node.key) {
                        trace!(
                            log, "Seen before";
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

                return_callback!(explored_node_callback(&explored_node)?);
            }

            if to_explore.len() == 0 && num_active_threads == 0 {
                // If we have nothing left to explore, and no working threads,
                // the search has failed
                info!(log, "Failed to find key"; "key" => %key);
                return Ok(None);
            } else if to_explore.len() == 0 && num_active_threads > 0 {
                // If there's nothing left to explore, we can wait for a thread
                // to finish with some results
                trace!(
                    log,
                    "Nothing to explore, waiting for a thread to finish"
                );
                wait_for_threads(&explored_channel_rx)?;
                continue;
            } else if num_active_threads >= max_num_active_threads {
                // If there's too many active threads, wait for another to
                // finish before starting another
                trace!(
                    log,
                    "Too many threads executing, waiting for a thread to \
                     finish"
                );
                wait_for_threads(&explored_channel_rx)?;
                continue;
            }

            // Pop a node off the `to_explore` queue
            assert!(to_explore.len() > 0);
            let current_node = to_explore.pop().unwrap();
            trace!(
                log, "Search loop iteration";
                "current_node" => %current_node.node,
                "current_cost" => current_node.cost,
                "previously_found" => found
                    .iter()
                    .map(|k| k.get_key_id().clone())
                    .collect::<Vec<String>>()
                    .join(", "),
                "left_to_explore" => to_explore.len());

            // Spawn a new thread to get the neighbours of `current_node`
            trace!(
                log, "Spawning new thread to explore node";
                "node" => %current_node.node);
            assert!(num_active_threads < max_num_active_threads);
            num_active_threads += 1;
            let spawn_key = key.clone();
            let spawn_explored_channel_tx = explored_channel_tx.clone();
            let spawn_get_neighbours_fn = get_neighbours_fn.clone();
            let spawn_log = log.new(o!());
            thread::spawn(move || {
                trace!(
                    spawn_log, "Getting neighbours";
                    "making_request" => true,
                    "node" => %current_node.node);
                let neighbours =
                    spawn_get_neighbours_fn(&current_node.node, &spawn_key);
                match &neighbours {
                    &Ok(ref neighbours) => trace!(
                        spawn_log, "Found neighbours for node";
                        "found" => true,
                        "node" => %current_node.node,
                        "neighbours" => neighbours
                            .iter()
                            .map(|n| n.key.get_key_id().clone())
                            .collect::<Vec<String>>()
                            .join(", ")),
                    &Err(ref err) => info!(
                        spawn_log, "Error on querying for neighbours";
                        "node" => %current_node.node,
                        "err" => %err.display_chain()),
                }

                spawn_explored_channel_tx
                    .send((current_node.node.clone(), neighbours))
                    .chain_err(|| "Error on `send` when getting neighbours")
                    .expect("3");
            });
        }
    }

    pub fn search_with_breadth<T: 'static>(
        &self,
        key: &Key,
        breadth: usize,
        start_nodes: Vec<Node>,
        get_neighbours_fn: GetNeighboursFn,
        found_node_callback: FoundNodeCallback<T>,
        explored_node_callback: ExploredNodeCallback<T>,
        num_active_threads: usize,
        timeout_sec: usize,
        log: Logger,
    ) -> Result<Option<T>> {
        // Continue the graph search looking for a key, until the `n`
        // closest nodes have also been explored.

        // List of tuples of the `n` closest nodes, where first is the node,
        // and second is a boolean telling whether it has been explored.
        let n_closest: Arc<Mutex<Vec<(Node, bool)>>> =
            Arc::new(Mutex::new(Vec::with_capacity(breadth)));

        let key_space = Arc::new(self.key_space_manager.create_from_key(key));

        let found_n_closest = n_closest.clone();
        let found_key_space_manager = self.key_space_manager.clone();
        let wrapped_found_node_callback = move |n: &Node| {
            // Add the new node to `n_closest`, sort it, and remove the last
            let mut n_closest_local = found_n_closest
                .lock()
                .expect("Failed to lock found_n_closest");
            n_closest_local.push((n.clone(), false));
            found_key_space_manager.sort_key_relative(
                &mut n_closest_local,
                &|&(ref n, _)| found_key_space_manager.create_from_key(&n.key),
                &key_space,
            );
            while n_closest_local.len() > breadth {
                n_closest_local.pop();
            }

            // Return the value from the callback passed in
            found_node_callback(n)
        };

        let explored_n_closest = n_closest.clone();
        let wrapped_explored_node_callback = move |n: &Node| {
            // Set the `n_closest` value to explored
            let mut n_closest_local = explored_n_closest
                .lock()
                .expect("Failed to lock explored_n_closest");
            for tuple in &mut *n_closest_local {
                if n == &tuple.0 {
                    tuple.1 = true;
                }
            }

            // Check if all of the `n_closest` has been explored
            let all_explored = n_closest_local.iter().all(&|&(_, ref e)| *e);

            if all_explored && n_closest_local.len() == breadth {
                Ok(SearchCallbackReturn::Exit())
            } else {
                // Return the value from the callback passed in
                explored_node_callback(n)
            }
        };

        self.search(
            key,
            start_nodes,
            get_neighbours_fn,
            Arc::new(wrapped_found_node_callback),
            Arc::new(wrapped_explored_node_callback),
            num_active_threads,
            timeout_sec,
            log,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use address::Address;
    use key::Key;

    use slog;
    use spectral::assert_that;
    use std::sync::Mutex;

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
                    search_explored_nodes
                        .lock()
                        .unwrap()
                        .push(n.clone());
                    Ok(SearchCallbackReturn::Continue())
                }),
                1,
                1,
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
