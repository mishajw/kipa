//! Implement `PayloadHandler` using graph based searches through KIPA net.

mod search;
mod neighbours_store;
mod key_space;

use error::*;
use key::Key;
use node::Node;
use address::Address;
use payload_handler::graph::neighbours_store::NeighboursStore;
use payload_handler::graph::key_space::{sort_key_relative, KeySpace};
use payload_handler::graph::search::{GetNeighboursFn, GraphSearch,
                                     SearchCallbackReturn};
use api::{RequestPayload, ResponsePayload};
use payload_handler::PayloadHandler;
use message_handler::PayloadClient;

use std::sync::{Arc, Mutex};
use slog::Logger;

/// The default size of the neighbours store
pub const DEFAULT_NEIGHBOURS_SIZE: usize = 3;

/// The default dimension size for key space
pub const DEFAULT_KEY_SPACE_SIZE: usize = 2;

/// The default bredth of the search to use when connecting
pub const DEFAULT_CONNECT_SEARCH_SIZE: usize = 3;

/// Contains graph search information.
pub struct GraphPayloadHandler {
    key: Key,
    neighbours_store: Arc<Mutex<NeighboursStore>>,
    graph_search: Arc<GraphSearch>,
    key_space_size: usize,
    log: Logger,
}

impl GraphPayloadHandler {
    /// Create a new graph request handler.
    ///
    /// - `key` is the key for the local node.
    /// - `remote_server` is used for communicating with other nodes.
    /// - `initial_node` is the initial other node in KIPA network.
    pub fn new(
        key: Key,
        neighbours_size: usize,
        key_space_size: usize,
        log: Logger,
    ) -> Self {
        GraphPayloadHandler {
            key: key.clone(),
            neighbours_store: Arc::new(Mutex::new(NeighboursStore::new(
                key,
                neighbours_size,
                key_space_size,
                log.new(o!("neighbours_store" => true)),
            ))),
            graph_search: Arc::new(GraphSearch::new()),
            key_space_size: key_space_size,
            log: log,
        }
    }

    fn search(
        &self,
        key: &Key,
        payload_client: Arc<PayloadClient>,
        log: Logger,
    ) -> Result<Option<Node>> {
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
            self.create_get_neighbours_fn(payload_client.clone()),
            Arc::new(found_callback),
            Arc::new(move |n| {
                trace!(
                    explored_log,
                    "Explored node when searching";
                    "node" => %n);
                Ok(SearchCallbackReturn::Continue())
            }),
            log,
        )
    }

    fn connect(
        &self,
        node: &Node,
        payload_client: Arc<PayloadClient>,
        log: Logger,
    ) -> Result<()> {
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
            self.create_get_neighbours_fn(payload_client.clone()),
            Arc::new(found_callback),
            Arc::new(explored_callback),
            log,
        )?;
        Ok(())
    }

    fn create_get_neighbours_fn(
        &self,
        payload_client: Arc<PayloadClient>,
    ) -> GetNeighboursFn {
        let neighbours_store = self.neighbours_store.clone();
        let key = self.key.clone();
        Arc::new(move |n, k: &Key| {
            if n.key == key {
                return Ok(neighbours_store.lock().unwrap().get_all());
            }

            let response = payload_client
                .send(n, RequestPayload::QueryRequest(k.clone()))?;

            match response {
                ResponsePayload::QueryResponse(ref nodes) => Ok(nodes.clone()),
                _ => Err(ErrorKind::ResponseError(
                    "Incorrect response for query request".into(),
                ).into()),
            }
        })
    }
}

impl PayloadHandler for GraphPayloadHandler {
    fn receive(
        &self,
        payload: &RequestPayload,
        sender: Option<&Node>,
        payload_client: Arc<PayloadClient>,
        message_id: u32,
    ) -> Result<ResponsePayload> {
        info!(
            self.log,
            "Received request";
            "sender" => sender.map(|n| n.to_string()).unwrap_or("none".into()));

        if sender.is_some() {
            self.neighbours_store
                .lock()
                .unwrap()
                .consider_candidate(&sender.unwrap());
        }

        match payload {
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
                Ok(ResponsePayload::SearchResponse(self.search(
                    &key,
                    payload_client,
                    self.log.new(o!(
                            "message_id" => message_id,
                            "search_request" => true
                        )),
                )?))
            }
            &RequestPayload::ConnectRequest(ref node) => {
                trace!(
                    self.log,
                    "Received connect request";
                    "node" => %node);
                self.connect(
                    node,
                    payload_client,
                    self.log.new(o!(
                            "message_id" => message_id,
                            "connect_request" => true
                        )),
                )?;
                Ok(ResponsePayload::ConnectResponse())
            }
            &RequestPayload::ListNeighboursRequest() => {
                trace!(self.log, "Replying recieved list neigbours request");
                let neighbours =
                    self.neighbours_store.lock().unwrap().get_all();
                trace!(
                    self.log,
                    "Replying to list neighbours request";
                    "list_neighbours" => true,
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
