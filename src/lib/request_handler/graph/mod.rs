//! Implement `RequestHandler` using graph based searches through KIPA net.

mod search;
mod neighbours_store;
mod key_space;

use error::*;
use server::Client;
use key::Key;
use node::Node;
use address::Address;
use request_handler::graph::neighbours_store::NeighboursStore;
use request_handler::graph::key_space::{sort_key_relative, KeySpace};
use request_handler::graph::search::{GraphSearch, SearchCallbackReturn};
use api::{MessageSender, RequestMessage, RequestPayload, ResponsePayload};
use request_handler::RequestHandler;

use std::sync::{Arc, Mutex};
use slog::Logger;

/// The default size of the neighbours store
pub const DEFAULT_NEIGHBOURS_SIZE: usize = 3;

/// The default dimension size for key space
pub const DEFAULT_KEY_SPACE_SIZE: usize = 2;

/// The default bredth of the search to use when connecting
pub const DEFAULT_CONNECT_SEARCH_SIZE: usize = 3;

/// Contains graph search information.
pub struct GraphRequestHandler {
    key: Key,
    neighbours_store: Arc<Mutex<NeighboursStore>>,
    graph_search: Arc<GraphSearch>,
    key_space_size: usize,
    log: Logger,
}

impl GraphRequestHandler {
    /// Create a new graph request handler.
    ///
    /// - `key` is the key for the local node.
    /// - `remote_server` is used for communicating with other nodes.
    /// - `initial_node` is the initial other node in KIPA network.
    pub fn new(
        key: Key,
        remote_server: Arc<Client>,
        neighbours_size: usize,
        key_space_size: usize,
        log: Logger,
    ) -> Self {
        let remote_server_clone = remote_server.clone();

        let neighbours_store = Arc::new(Mutex::new(NeighboursStore::new(
            key.clone(),
            neighbours_size,
            key_space_size,
            log.new(o!("neighbours-store" => true)),
        )));

        let graph_search_key = key.clone();
        let graph_search_neighbours_store = neighbours_store.clone();
        let graph_search = GraphSearch::new(
            Arc::new(move |n, k: &Key| {
                if n.key == graph_search_key {
                    return Ok(graph_search_neighbours_store
                        .lock()
                        .unwrap()
                        .get_all());
                }

                let response = remote_server_clone
                    .receive(n, RequestPayload::QueryRequest(k.clone()))?;

                match response.payload {
                    ResponsePayload::QueryResponse(ref nodes) => {
                        Ok(nodes.clone())
                    }
                    _ => Err(ErrorKind::ResponseError(
                        "Incorrect response for query request".into(),
                    ).into()),
                }
            }),
            log.new(o!("search" => true)),
        );

        GraphRequestHandler {
            key: key,
            neighbours_store: neighbours_store,
            graph_search: Arc::new(graph_search),
            key_space_size: key_space_size,
            log: log,
        }
    }

    fn search(&self, key: &Key) -> Result<Option<Node>> {
        let callback_key = key.clone();
        let found_log = self.log.new(o!());
        let found_callback = move |n: &Node| {
            trace!(
                found_log, "Found node when searching"; "node" => %n);
            if n.key == callback_key {
                Ok(SearchCallbackReturn::Return(n.clone()))
            } else {
                Ok(SearchCallbackReturn::Continue())
            }
        };

        let explored_log = self.log.new(o!());

        self.graph_search.search(
            &key,
            vec![
                Node::new(
                    Address::new(vec![0, 0, 0, 0], 10842),
                    self.key.clone(),
                ),
            ],
            Arc::new(found_callback),
            Arc::new(move |n| {
                trace!(
                    explored_log,
                    "Explored node when searching";
                    "node" => %n);
                Ok(SearchCallbackReturn::Continue())
            }),
        )
    }

    fn connect(&self, node: &Node) -> Result<()> {
        // Continue the graph search looking for ourselves, until the `n`
        // closest nodes to ourselves have also been explored.

        // List of tuples of the `n` closest nodes, where first is the node,
        // and second is a boolean telling whether it has been explored.
        let n_closest: Arc<Mutex<Vec<(Node, bool)>>> = Arc::new(Mutex::new(
            Vec::with_capacity(DEFAULT_CONNECT_SEARCH_SIZE),
        ));

        let local_key_space =
            Arc::new(KeySpace::from_key(&self.key, self.key_space_size));

        let found_n_closest = n_closest.clone();
        let found_key_space_size = self.key_space_size.clone();
        let found_neighbours_store = self.neighbours_store.clone();
        let found_log = self.log.new(o!());
        let found_callback = move |n: &Node| {
            trace!(
                found_log,
                "Found node when connecting";
                "node" => %n);
            // Consider the connected node as a candidate
            found_neighbours_store
                .lock()
                .expect("Failed to lock found_neighbours_store")
                .consider_candidate(n);

            // Add the new node to `n_closest`, sort it, and remove the last
            let mut n_closest_local = found_n_closest
                .lock()
                .expect("Failed to lock found_n_closest");
            n_closest_local.push((n.clone(), false));
            sort_key_relative(
                &mut n_closest_local,
                &|&(ref n, _)| KeySpace::from_key(&n.key, found_key_space_size),
                &local_key_space,
            );
            while n_closest_local.len() > DEFAULT_CONNECT_SEARCH_SIZE {
                n_closest_local.pop();
            }

            Ok(SearchCallbackReturn::Continue())
        };

        let explored_n_closest = n_closest.clone();
        let explored_log = self.log.new(o!());
        let explored_callback = move |n: &Node| {
            trace!(
                explored_log,
                "Explored node when connecting";
                "node" => %n);

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

            if all_explored
                && n_closest_local.len() == DEFAULT_CONNECT_SEARCH_SIZE
            {
                Ok(SearchCallbackReturn::Return(()))
            } else {
                Ok(SearchCallbackReturn::Continue())
            }
        };

        self.graph_search.search(
            &self.key,
            vec![node.clone()],
            Arc::new(found_callback),
            Arc::new(explored_callback),
        )?;
        Ok(())
    }
}

impl RequestHandler for GraphRequestHandler {
    fn receive(&self, request: &RequestMessage) -> Result<ResponsePayload> {
        info!(
            self.log,
            "Received request";
            "sender" => %request.sender);

        match &request.sender {
            &MessageSender::Node(ref n) => {
                self.neighbours_store.lock().unwrap().consider_candidate(&n)
            }
            &MessageSender::Cli() => {}
        };

        match &request.payload {
            &RequestPayload::QueryRequest(ref key) => {
                trace!(
                    self.log,
                    "Received query request";
                    "key" => %key);
                let nodes =
                    self.neighbours_store.lock().unwrap().get_n_closest(key, 3);
                trace!(
                    self.log,
                    "Replying";
                    "response" => nodes
                        .iter()
                        .map(|n| n.key.get_key_id().clone())
                        .collect::<Vec<String>>()
                        .join(", ")
                );

                Ok(ResponsePayload::QueryResponse(nodes))
            }
            &RequestPayload::SearchRequest(ref key) => {
                trace!(
                    self.log,
                    "Received search request";
                    "key" => %key);
                Ok(ResponsePayload::SearchResponse(self.search(&key)?))
            }
            &RequestPayload::ConnectRequest(ref node) => {
                trace!(
                    self.log,
                    "Received connect request";
                    "node" => %node);
                self.connect(node)?;
                Ok(ResponsePayload::ConnectResponse())
            }
            &RequestPayload::ListNeighboursRequest() => {
                trace!(self.log, "Replying recieved list neigbours request");
                let neighbours =
                    self.neighbours_store.lock().unwrap().get_all();
                trace!(
                    self.log,
                    "Replying to list neighbours request";
                    "list-neighbours" => true,
                    "reply" => true,
                    "neighbours" => neighbours
                        .iter()
                        .map(|n| n.key.get_key_id().clone())
                        .collect::<Vec<String>>()
                        .join(", "));
                Ok(ResponsePayload::ListNeighboursResponse(neighbours))
            }
        }
    }
}
