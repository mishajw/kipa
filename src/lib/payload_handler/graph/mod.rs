//! Implement `PayloadHandler` using graph based searches through KIPA net.

mod key_space;
mod neighbours_store;
mod search;

pub use payload_handler::graph::neighbours_store::{NeighboursStore,
                                                   DEFAULT_ANGLE_WEIGHTING,
                                                   DEFAULT_DISTANCE_WEIGHTING,
                                                   DEFAULT_MAX_NUM_NEIGHBOURS};
pub use payload_handler::graph::key_space::{KeySpaceManager,
                                            DEFAULT_KEY_SPACE_SIZE};

use address::Address;
use api::{RequestPayload, ResponsePayload};
use error::*;
use key::Key;
use message_handler::PayloadClient;
use node::Node;
use payload_handler::PayloadHandler;
use payload_handler::graph::search::{GetNeighboursFn, GraphSearch,
                                     SearchCallbackReturn};

use std::sync::{Arc, Mutex};
use slog::Logger;

/// The default breadth to use when searching
pub const DEFAULT_SEARCH_BREADTH: usize = 3;

/// The default breadth of the search to use when connecting
pub const DEFAULT_CONNECT_SEARCH_BREADTH: usize = 3;

/// The default maximum number of concurrent threads to have when searching
pub const DEFAULT_MAX_NUM_SEARCH_THREADS: usize = 3;

/// Contains graph search information.
pub struct GraphPayloadHandler {
    key: Key,
    search_breadth: usize,
    connect_search_breadth: usize,
    max_num_search_threads: usize,
    neighbours_store: Arc<Mutex<NeighboursStore>>,
    graph_search: Arc<GraphSearch>,
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
        search_breadth: usize,
        connect_search_breadth: usize,
        max_num_search_threads: usize,
        key_space_manager: Arc<KeySpaceManager>,
        neighbours_store: Arc<Mutex<NeighboursStore>>,
        log: Logger,
    ) -> Self {
        GraphPayloadHandler {
            key: key.clone(),
            search_breadth,
            connect_search_breadth,
            max_num_search_threads,
            graph_search: Arc::new(GraphSearch::new(key_space_manager.clone())),
            neighbours_store: neighbours_store,
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
                info!(found_log, "Search success"; "node" => %n);
                Ok(SearchCallbackReturn::Return(n.clone()))
            } else {
                Ok(SearchCallbackReturn::Continue())
            }
        };

        let explored_log = self.log.new(o!());

        self.graph_search.search_with_breadth(
            &key,
            self.search_breadth,
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
            self.max_num_search_threads,
            log,
        )
    }

    fn connect(
        &self,
        node: &Node,
        payload_client: Arc<PayloadClient>,
        log: Logger,
    ) -> Result<()> {
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
            Ok(SearchCallbackReturn::Continue())
        };

        let explored_log = self.log.new(o!());
        let explored_callback = move |n: &Node| {
            trace!(
                explored_log,
                "Explored node when connecting";
                "node" => %n);
            Ok(SearchCallbackReturn::Continue())
        };

        self.graph_search.search_with_breadth::<()>(
            &self.key,
            self.connect_search_breadth,
            vec![node.clone()],
            self.create_get_neighbours_fn(payload_client.clone()),
            Arc::new(found_callback),
            Arc::new(explored_callback),
            self.max_num_search_threads,
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
