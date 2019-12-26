use api::{Key, Node};
use error::*;
use key_space_manager::KeySpaceManager;
use thread_manager::ThreadManager;

use std::collections::{BinaryHeap, HashSet};
use std::sync::{
    mpsc::{channel, Receiver},
    Arc, Mutex,
};
use std::time::Duration;

use graph::search_callback::{SearchCallback, SearchCallbackAction};
use graph::search_node::SearchNode;
use slog::Logger;

/// Performs searches on a graph of KIPA nodes.
pub struct GraphSearch {
    key_space_manager: Arc<KeySpaceManager>,
    thread_manager: Arc<ThreadManager>,
}

impl GraphSearch {
    #[allow(missing_docs)]
    pub fn new(key_space_manager: Arc<KeySpaceManager>, thread_pool_size: usize) -> Self {
        GraphSearch {
            key_space_manager,
            thread_manager: Arc::new(ThreadManager::from_size(
                "graph_search".into(),
                thread_pool_size,
            )),
        }
    }

    /// Search for a key through looking up the neighbours of nodes in the KIPA network.
    ///
    /// Simple heuristic-based greedy-first search (GFS), where the heuristic is the distance in key
    /// space, provided by `GraphSearch::key_space_manager`.
    ///
    /// Differs from normal GFS as neighbour queries are done in parallel, with a maximum amount of
    /// queries running simultaneously (defined by `max_num_active_threads`). All querying threads
    /// push results into a thread-safe priority queue. This means that the GFS is impacted by which
    /// one of the queries resolve first.
    ///
    /// The other key difference is that the search does not exit when we find the correct node -
    /// we only exit when the `found_node_callback` or the `explored_node_callback` functions return
    /// `SearchCallbackAction::{Return(...),Exit}`
    ///
    /// Algorithm outline is described [here](
    /// https://github.com/mishajw/kipa/blob/master/docs/overview.md#graph-search).
    pub fn search<T: 'static>(
        &self,
        key: &Key,
        start_nodes: Vec<Node>,
        callback: impl SearchCallback<T>,
        params: SearchParams,
        log: Logger,
    ) -> Result<Option<T>> {
        remotery_scope!("graph_search_logic");

        info!(log, "Starting graph search"; "key" => %key);

        // Wrap in BreadthCallback to add handling for failure case.
        let callback = Arc::new(BreadthCallback {
            underlying: callback,
            breadth: params.breadth,
            search_key: key.clone(),
            n_closest_nodes: Mutex::new(Vec::with_capacity(params.breadth)),
            key_space_manager: self.key_space_manager.clone(),
            log: log.new(o!("breadth_callback" => true)),
        });

        let key_space = self.key_space_manager.create_from_key(key);
        // Double the timeout for waiting for threads to resolve to allow some slack in threads
        // responding.
        let timeout = params.timeout.mul_f32(2.0);

        // Create structures for the search
        let mut to_explore = BinaryHeap::new();
        let mut found: HashSet<Key> = HashSet::new();

        // Set up channels for returning results from spawned threads
        let (explored_channel_tx, explored_channel_rx) =
            channel::<(Node, InternalResult<Vec<Node>>)>();

        // Counter of active threads
        let mut num_active_threads = 0 as usize;

        // Cast a `Node` into a `SearchNode` so it can be compared in
        // `to_explore`
        let into_search_node = |n: Node| -> SearchNode {
            let cost = self
                .key_space_manager
                .distance(&self.key_space_manager.create_from_key(&n.key), &key_space);
            SearchNode {
                node: n,
                cost: -cost,
            }
        };

        let wait_explored_channel_tx = explored_channel_tx.clone();
        let wait_for_threads = |rx: &Receiver<(Node, InternalResult<Vec<Node>>)>| -> Result<()> {
            remotery_scope!("wait_for_threads");

            // Wait for `recv` to resolve
            let recv = rx
                .recv_timeout(timeout)
                .chain_err(|| "Error on `recv` when waiting for threads")?;
            // And send it back down the channel
            wait_explored_channel_tx
                .send(recv)
                .chain_err(|| "Error on `send` when waiting for threads")?;

            Ok(())
        };

        // Set up found and to_explore.
        for n in start_nodes {
            execute_callback_action!(callback.found_node(&n)?);
            let search_node = into_search_node(n);
            found.insert(search_node.node.key.clone());
            to_explore.push(search_node);
        }

        loop {
            remotery_scope!("search_loop");

            // Pop everything we can off the channel and into `found` and
            // `to_explore`
            while let Ok((explored_node, found_nodes)) = explored_channel_rx.try_recv() {
                remotery_scope!("processing_explored_channel");

                // If we pop something off the channel, a thread has finished
                num_active_threads -= 1;

                // Strip errors from result - if there's an error, set to an
                // empty list. Logging of the error has already been done, so
                // we can ignore it here
                let flattened_found_nodes: Vec<Node> = found_nodes.unwrap_or_else(|_| vec![]);
                // Check all found nodes
                for found_node in flattened_found_nodes {
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
                    execute_callback_action!(callback.found_node(&search_node.node)?);
                    // Otherwise, add it to the explore list
                    to_explore.push(search_node);
                }

                execute_callback_action!(callback.explored_node(&explored_node)?);
            }

            if to_explore.is_empty() && num_active_threads == 0 {
                // If we have nothing left to explore, and no working threads,
                // the search has failed
                info!(log, "Failed to find key"; "key" => %key);
                return Ok(None);
            } else if to_explore.is_empty() && num_active_threads > 0 {
                // If there's nothing left to explore, we can wait for a thread
                // to finish with some results
                trace!(log, "Nothing to explore, waiting for a thread to finish");
                wait_for_threads(&explored_channel_rx)?;
                continue;
            } else if num_active_threads >= params.max_num_active_threads {
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
            assert!(!to_explore.is_empty());
            let current_node = to_explore.pop().unwrap();
            trace!(
                log, "Search loop iteration";
                "current_node" => %current_node.node,
                "current_cost" => current_node.cost,
                "previously_found" => found
                    .iter()
                    .map(|k| k.key_id.clone())
                    .collect::<Vec<String>>()
                    .join(", "),
                "left_to_explore" => to_explore.len(),
                "num_active_threads" => num_active_threads,
                "max_num_active_threads" => params.max_num_active_threads);

            // Spawn a new thread to get the neighbours of `current_node`
            trace!(
                log, "Spawning new thread to explore node";
                "node" => %current_node.node);
            assert!(num_active_threads < params.max_num_active_threads);
            num_active_threads += 1;
            let spawn_key = key.clone();
            let spawn_callback = callback.clone();
            let spawn_explored_channel_tx = explored_channel_tx.clone();
            let spawn_log = log.new(o!());
            self.thread_manager.spawn(move || {
                remotery_scope!("exploring_node");

                trace!(
                    spawn_log, "Getting neighbours";
                    "making_request" => true,
                    "node" => %current_node.node);
                let neighbours = spawn_callback.get_neighbours(&current_node.node, &spawn_key);
                match neighbours {
                    Ok(ref neighbours) => trace!(
                        spawn_log, "Found neighbours for node";
                        "found" => true,
                        "node" => %current_node.node,
                        "neighbours" => neighbours
                            .iter()
                            .map(|n| n.key.key_id.clone())
                            .collect::<Vec<String>>()
                            .join(", ")),
                    Err(ref err) => warn!(
                        spawn_log, "Error on querying for neighbours";
                        "node" => %current_node.node,
                        "err" => %err),
                }

                if let Err(err) =
                    spawn_explored_channel_tx.send((current_node.node.clone(), neighbours))
                {
                    error!(
                        spawn_log,
                        "Failed to send found nodes to explored channel";
                        "err" => %err);
                }
            });
        }
    }
}

/// Tunable parameters for the search.
pub struct SearchParams {
    pub breadth: usize,
    pub max_num_active_threads: usize,
    pub timeout: Duration,
}

/// Wraps a callback to exit if we reach an "explored breadth" threshold.
///
/// We exit if the N closest nodes to the `search_key` have been queried for their closest nodes to
/// the `search_key` and not returned anything closer.
struct BreadthCallback<CallbackT> {
    underlying: CallbackT,
    breadth: usize,
    search_key: Key,
    n_closest_nodes: Mutex<Vec<(Node, bool)>>,
    key_space_manager: Arc<KeySpaceManager>,
    log: Logger,
}

impl<CallbackT: SearchCallback<ReturnT>, ReturnT> SearchCallback<ReturnT>
    for BreadthCallback<CallbackT>
{
    fn get_neighbours(&self, node: &Node, search_key: &Key) -> InternalResult<Vec<Node>> {
        self.underlying.get_neighbours(node, search_key)
    }

    fn found_node(&self, node: &Node) -> Result<SearchCallbackAction<ReturnT>> {
        remotery_scope!("breadth_callback_found");

        let key_space = Arc::new(self.key_space_manager.create_from_key(&self.search_key));

        // Add the new node to `n_closest`, sort it, and remove the last
        let mut n_closest_nodes = self.n_closest_nodes.lock().unwrap();
        n_closest_nodes.push((node.clone(), false));
        self.key_space_manager.sort_key_relative(
            &mut n_closest_nodes,
            |&(ref n, _)| self.key_space_manager.create_from_key(&n.key),
            &key_space,
        );
        while n_closest_nodes.len() > self.breadth {
            n_closest_nodes.pop();
        }

        // Return the value from the callback passed in
        self.underlying.found_node(node)
    }

    fn explored_node(&self, node: &Node) -> Result<SearchCallbackAction<ReturnT>> {
        remotery_scope!("breadth_callback_explored");

        // Set the `n_closest` value to explored.
        let mut n_closest_nodes = self.n_closest_nodes.lock().unwrap();
        for tuple in &mut *n_closest_nodes {
            if node == &tuple.0 {
                tuple.1 = true;
            }
        }

        // Check if all of the `n_closest` has been explored.
        let all_explored = n_closest_nodes.iter().all(&|&(_, ref e)| *e);

        if all_explored && n_closest_nodes.len() == self.breadth {
            debug!(
                self.log, "Exiting search because the closest nodes to the goal have been explored";
                "search_breadth" => self.breadth);
            return Ok(SearchCallbackAction::Exit());
        }

        // Return the value from the callback passed in.
        self.underlying.explored_node(node)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use api::{Address, Key};

    use slog;
    use spectral::assert_that;
    use std::sync::Mutex;

    const NUM_NODES: usize = 100;
    const START_INDEX: usize = 50;

    // TODO: Re-enable test when key mocking is supported.
    // #[test]
    #[allow(unused)]
    fn test_search_order() {
        let test_log = Logger::root(slog::Discard, o!());
        let nodes = (0..NUM_NODES)
            .map(|i| {
                Node::new(
                    Address::new(vec![0, 0, 0, i as u8], i as u16),
                    Key::new(format!("{:08}", i), vec![i as u8]).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let nodes = Arc::new(nodes);

        let search = GraphSearch::new(Arc::new(KeySpaceManager::new(1)), 1);
        let explored_nodes = Arc::new(Mutex::new(vec![]));

        search
            .search(
                &nodes[0].key,
                vec![nodes[START_INDEX].clone(), nodes[START_INDEX + 1].clone()],
                TestCallback {
                    nodes: nodes.clone(),
                    explored_nodes: explored_nodes.clone(),
                },
                SearchParams {
                    breadth: 100,
                    max_num_active_threads: 1,
                    timeout: Duration::from_secs(1),
                },
                test_log,
            )
            .unwrap();

        let found_indices: Vec<usize> = explored_nodes
            .lock()
            .unwrap()
            .iter()
            .map(|n| n.address.port as usize)
            .collect();
        let mut expected_found: Vec<usize> = (0..START_INDEX + 1).rev().collect();
        expected_found.extend((START_INDEX + 1..NUM_NODES).collect::<Vec<usize>>());
        assert_that!(expected_found).is_equal_to(found_indices);
    }

    struct TestCallback {
        nodes: Arc<Vec<Node>>,
        explored_nodes: Arc<Mutex<Vec<Node>>>,
    }

    impl SearchCallback<Node> for TestCallback {
        fn get_neighbours(&self, node: &Node, _search_key: &Key) -> InternalResult<Vec<Node>> {
            let node_index = node.address.port as usize;
            let neighbours: Vec<Node> = if node_index > 0 && node_index < NUM_NODES - 1 {
                vec![
                    self.nodes[node_index - 1].clone(),
                    self.nodes[node_index + 1].clone(),
                ]
            } else if node_index <= 0 {
                vec![self.nodes[node_index + 1].clone()]
            } else if node_index >= NUM_NODES - 1 {
                vec![self.nodes[node_index - 1].clone()]
            } else {
                vec![]
            };
            Ok(neighbours)
        }

        fn found_node(&self, _node: &Node) -> Result<SearchCallbackAction<Node>> {
            Ok(SearchCallbackAction::Continue())
        }

        fn explored_node(&self, node: &Node) -> Result<SearchCallbackAction<Node>> {
            self.explored_nodes.lock().unwrap().push(node.clone());
            Ok(SearchCallbackAction::Continue())
        }
    }
}
