//! Implement `PayloadHandler` using graph based searches through KIPA net

mod key_space;
mod neighbours_store;
mod search;

pub use payload_handler::graph::key_space::{
    KeySpaceManager, DEFAULT_KEY_SPACE_SIZE,
};
pub use payload_handler::graph::neighbours_store::{
    NeighboursStore, DEFAULT_ANGLE_WEIGHTING, DEFAULT_DISTANCE_WEIGHTING,
    DEFAULT_MAX_NUM_NEIGHBOURS,
};

use address::Address;
use api::{RequestPayload, ResponsePayload};
use error::*;
use key::Key;
use message_handler::OutgoingMessageHandler;
use node::Node;
use payload_handler::graph::search::{
    GetNeighboursFn, GraphSearch, SearchCallbackReturn,
};
use payload_handler::{InternalResult, PayloadHandler};

use slog::Logger;
use std::sync::{Arc, Mutex};
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
    neighbours_store: Arc<Mutex<NeighboursStore>>,
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
        key_space_manager: Arc<KeySpaceManager>,
        neighbours_store: Arc<Mutex<NeighboursStore>>,
        log: Logger,
    ) -> Self
    {
        GraphPayloadHandler {
            key: key.clone(),
            search_breadth,
            connect_search_breadth,
            max_num_search_threads,
            search_timeout_sec,
            graph_search: Arc::new(GraphSearch::new(key_space_manager)),
            neighbours_store,
            log,
        }
    }

    fn search(
        &self,
        key: &Key,
        outgoing_message_handler: Arc<OutgoingMessageHandler>,
        log: Logger,
    ) -> InternalResult<Option<Node>>
    {
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

        let search_result = self.graph_search.search_with_breadth(
            &key,
            self.search_breadth,
            vec![Node::new(
                Address::new(vec![0, 0, 0, 0], 10842),
                self.key.clone(),
            )],
            self.create_get_neighbours_fn(outgoing_message_handler),
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

    fn connect(
        &self,
        node: &Node,
        outgoing_message_handler: Arc<OutgoingMessageHandler>,
        log: Logger,
    ) -> InternalResult<()>
    {
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

        let result = self.graph_search.search_with_breadth::<()>(
            &self.key,
            self.connect_search_breadth,
            vec![node.clone()],
            self.create_get_neighbours_fn(outgoing_message_handler),
            Arc::new(found_callback),
            Arc::new(explored_callback),
            self.max_num_search_threads,
            self.search_timeout_sec,
            log,
        );
        to_internal_result(result)?;

        Ok(())
    }

    fn create_get_neighbours_fn(
        &self,
        outgoing_message_handler: Arc<OutgoingMessageHandler>,
    ) -> GetNeighboursFn
    {
        let neighbours_store = self.neighbours_store.clone();
        let key = self.key.clone();
        let timeout = Duration::from_secs(self.search_timeout_sec as u64);
        Arc::new(move |n, k: &Key| {
            if n.key == key {
                return Ok(neighbours_store.lock().unwrap().get_all());
            }

            let response = outgoing_message_handler.send(
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
        outgoing_message_handler: Arc<OutgoingMessageHandler>,
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
                .lock()
                .unwrap()
                .consider_candidate(&sender.unwrap());
        }

        match *payload {
            RequestPayload::QueryRequest(ref key) => {
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
            RequestPayload::SearchRequest(ref key) => {
                trace!(
                    self.log,
                    "Received search request";
                    "key" => %key);
                Ok(ResponsePayload::SearchResponse(self.search(
                    &key,
                    outgoing_message_handler,
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
                    outgoing_message_handler,
                    self.log.new(o!(
                            "message_id" => message_id,
                            "connect_request" => true
                        )),
                )?;
                Ok(ResponsePayload::ConnectResponse())
            }
            RequestPayload::ListNeighboursRequest() => {
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
                        .map(|n| n.to_string())
                        .collect::<Vec<String>>()
                        .join(", "),
                    "neighbour_keys" => neighbours
                        .iter()
                        .map(|n| n.key.get_key_id().clone())
                        .collect::<Vec<String>>()
                        .join(", "));
                Ok(ResponsePayload::ListNeighboursResponse(neighbours))
            }
        }
    }
}
