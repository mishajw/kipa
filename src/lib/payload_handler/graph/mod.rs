//! Implement `PayloadHandler` using graph based searches through KIPA net

pub mod neighbour_gc;
mod neighbours_store;
mod search;

pub use payload_handler::graph::neighbours_store::{
    NeighboursStore, DEFAULT_ANGLE_WEIGHTING, DEFAULT_DISTANCE_WEIGHTING,
    DEFAULT_MAX_NUM_NEIGHBOURS,
};

use address::Address;
use api::{RequestPayload, ResponsePayload};
use error::*;
use key::Key;
use key_space::KeySpaceManager;
use message_handler::MessageHandlerClient;
use node::Node;
use payload_handler::graph::search::{
    GetNeighboursFn, GraphSearch, SearchCallbackReturn,
};
use payload_handler::{InternalResult, PayloadHandler};

use slog::Logger;
use std::sync::Arc;
use std::time::Duration;

/// The default breadth to use when searching
pub const DEFAULT_SEARCH_BREADTH: &str = "3";

/// The default breadth of the search to use when connecting
pub const DEFAULT_CONNECT_SEARCH_BREADTH: &str = "3";

/// The default maximum number of concurrent threads to have when searching
pub const DEFAULT_MAX_NUM_SEARCH_THREADS: &str = "3";

/// The default timeout for queries when performing a search
pub const DEFAULT_SEARCH_TIMEOUT_SEC: &str = "2";

/// Contains graph search information
pub struct GraphPayloadHandler {
    key: Key,
    search_breadth: usize,
    connect_search_breadth: usize,
    max_num_search_threads: usize,
    search_timeout_sec: usize,
    message_handler_client: Arc<MessageHandlerClient>,
    neighbours_store: Arc<NeighboursStore>,
    graph_search: Arc<GraphSearch>,
    log: Logger,
}

impl GraphPayloadHandler {
    /// Create a new graph request handler
    ///
    /// - `key` is the key for the local node.
    /// - `remote_server` is used for communicating with other nodes.
    /// - `initial_node` is the initial other node in KIPA network.
    pub fn new(
        key: &Key,
        search_breadth: usize,
        connect_search_breadth: usize,
        max_num_search_threads: usize,
        search_timeout_sec: usize,
        message_handler_client: Arc<MessageHandlerClient>,
        key_space_manager: Arc<KeySpaceManager>,
        neighbours_store: Arc<NeighboursStore>,
        log: Logger,
    ) -> Self
    {
        GraphPayloadHandler {
            key: key.clone(),
            search_breadth,
            connect_search_breadth,
            max_num_search_threads,
            search_timeout_sec,
            message_handler_client,
            graph_search: Arc::new(GraphSearch::new(key_space_manager)),
            neighbours_store,
            log,
        }
    }

    fn search(&self, key: &Key, log: Logger) -> InternalResult<Option<Node>> {
        let callback_key = key.clone();
        let found_log = self.log.new(o!());
        let found_message_handler_server = self.message_handler_client.clone();
        let found_timeout = Duration::from_secs(self.search_timeout_sec as u64);
        let found_callback = move |n: &Node| {
            trace!(
                found_log, "Found node when searching"; "node" => %n);
            if n.key == callback_key {
                // Send verification message to the node to ensure that the
                // discovered IP address is owned by the requested key
                //
                // If the verification fails, then log a warning but continue
                // the search. If we exit here, it is possible to easily attack
                // a search by returning fake nodes whenever you receive a query
                if let Err(err) = found_message_handler_server.send(
                    n,
                    RequestPayload::VerifyRequest(),
                    found_timeout,
                ) {
                    warn!(
                        found_log, "Error when sending verification message \
                        after finding correct node";
                        "err" => %err, "node" => %n);
                    return Ok(SearchCallbackReturn::Continue());
                }

                info!(found_log, "Search success"; "node" => %n);
                Ok(SearchCallbackReturn::Return(n.clone()))
            } else {
                Ok(SearchCallbackReturn::Continue())
            }
        };

        let explored_log = self.log.new(o!());

        let search_result = self.graph_search.search_with_breadth(
            &key,
            self.search_breadth,
            vec![Node::new(
                Address::new(vec![0, 0, 0, 0], 10842),
                self.key.clone(),
            )],
            self.create_get_neighbours_fn(),
            Arc::new(found_callback),
            Arc::new(move |n| {
                trace!(
                    explored_log,
                    "Explored node when searching";
                    "node" => %n);
                Ok(SearchCallbackReturn::Continue())
            }),
            self.max_num_search_threads,
            self.search_timeout_sec,
            log,
        );

        to_internal_result(search_result)
    }

    fn connect(&self, node: &Node, log: Logger) -> InternalResult<()> {
        let found_neighbours_store = self.neighbours_store.clone();
        let found_log = self.log.new(o!());
        let found_callback = move |n: &Node| {
            trace!(
                found_log,
                "Found node when connecting";
                "node" => %n);
            // Consider the connected node as a candidate
            found_neighbours_store.consider_candidate(n, false);
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

        let result = self.graph_search.search_with_breadth::<()>(
            &self.key,
            self.connect_search_breadth,
            vec![node.clone()],
            self.create_get_neighbours_fn(),
            Arc::new(found_callback),
            Arc::new(explored_callback),
            self.max_num_search_threads,
            self.search_timeout_sec,
            log,
        );
        to_internal_result(result)?;

        Ok(())
    }

    fn create_get_neighbours_fn(&self) -> GetNeighboursFn {
        let neighbours_store = self.neighbours_store.clone();
        let key = self.key.clone();
        let timeout = Duration::from_secs(self.search_timeout_sec as u64);
        let message_handler_client = self.message_handler_client.clone();
        Arc::new(move |n, k: &Key| {
            if n.key == key {
                return Ok(neighbours_store.get_all());
            }

            let response = message_handler_client.send(
                n,
                RequestPayload::QueryRequest(k.clone()),
                timeout,
            );

            match response {
                Ok(ResponsePayload::QueryResponse(ref nodes)) => {
                    Ok(nodes.clone())
                }
                Ok(_) => to_internal_result(Err(ErrorKind::ResponseError(
                    "Incorrect response for query request".into(),
                ).into())),
                Err(err) => Err(err),
            }
        })
    }
}

impl PayloadHandler for GraphPayloadHandler {
    fn receive(
        &self,
        payload: &RequestPayload,
        sender: Option<&Node>,
        message_id: u32,
    ) -> InternalResult<ResponsePayload>
    {
        info!(
            self.log,
            "Received request";
            "sender" =>
                sender.map(|n| n.to_string()).unwrap_or_else(|| "none".into()));

        if sender.is_some() {
            self.neighbours_store
                .consider_candidate(&sender.unwrap(), true);
        }

        match *payload {
            RequestPayload::QueryRequest(ref key) => {
                trace!(
                    self.log,
                    "Received query request";
                    "key" => %key);
                let nodes = self.neighbours_store.get_n_closest(key, 3);
                trace!(
                    self.log,
                    "Replying";
                    "response" => nodes
                        .iter()
                        .map(|n| n.key.key_id.clone())
                        .collect::<Vec<String>>()
                        .join(", ")
                );

                Ok(ResponsePayload::QueryResponse(nodes))
            }
            RequestPayload::SearchRequest(ref key) => {
                trace!(
                    self.log,
                    "Received search request";
                    "key" => %key);
                Ok(ResponsePayload::SearchResponse(self.search(
                    &key,
                    self.log.new(o!(
                            "message_id" => message_id,
                            "search_request" => true
                        )),
                )?))
            }
            RequestPayload::ConnectRequest(ref node) => {
                trace!(
                    self.log,
                    "Received connect request";
                    "node" => %node);
                self.connect(
                    node,
                    self.log.new(o!(
                            "message_id" => message_id,
                            "connect_request" => true
                        )),
                )?;
                Ok(ResponsePayload::ConnectResponse())
            }
            RequestPayload::ListNeighboursRequest() => {
                trace!(self.log, "Replying recieved list neigbours request");
                let neighbours = self.neighbours_store.get_all();
                trace!(
                    self.log,
                    "Replying to list neighbours request";
                    "list_neighbours" => true,
                    "reply" => true,
                    "neighbours" => neighbours
                        .iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<String>>()
                        .join(", "),
                    "neighbour_keys" => neighbours
                        .iter()
                        .map(|n| n.key.key_id.clone())
                        .collect::<Vec<String>>()
                        .join(", "));
                Ok(ResponsePayload::ListNeighboursResponse(neighbours))
            }
            RequestPayload::VerifyRequest() => {
                Ok(ResponsePayload::VerifyResponse())
            }
        }
    }
}
