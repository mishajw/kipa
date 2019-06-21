//! Implement `PayloadHandler` using graph based searches through KIPA net

pub mod neighbour_gc;
mod neighbours_store;
mod search;

pub use graph::neighbours_store::{
    NeighboursStore, DEFAULT_ANGLE_WEIGHTING, DEFAULT_DISTANCE_WEIGHTING,
    DEFAULT_MAX_NUM_NEIGHBOURS,
};

use api::request::MessageMode;
use api::{Address, Key, Node};
use api::{RequestPayload, ResponsePayload};
use error::*;
use graph::search::{GetNeighboursFn, GraphSearch, SearchCallbackReturn};
use key_space_manager::KeySpaceManager;
use message_handler::MessageHandlerClient;
use payload_handler::PayloadHandler;

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

/// Default size of thread pool for conducting searches
pub const DEFAULT_SEARCH_THREAD_POOL_SIZE: &str = "10";

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
        search_thread_pool_size: usize,
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
            graph_search: Arc::new(GraphSearch::new(
                key_space_manager,
                search_thread_pool_size,
            )),
            neighbours_store,
            log,
        }
    }

    fn search(
        &self,
        key: &Key,
        mode: &MessageMode,
        log: Logger,
    ) -> InternalResult<Option<Node>>
    {
        remotery_scope!("graph_search");

        let callback_key = key.clone();
        let found_log = self.log.new(o!());
        let found_message_handler_server = self.message_handler_client.clone();
        let found_timeout = Duration::from_secs(self.search_timeout_sec as u64);
        let found_mode = mode.clone();
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
                if let Err(err) = found_message_handler_server.send_message(
                    n,
                    RequestPayload::VerifyRequest(),
                    found_timeout,
                    &found_mode,
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
            self.create_get_neighbours_fn(mode.clone()),
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
        remotery_scope!("graph_connect");

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
            // Use fast mode, as we don't care about secrecy when connecting:
            // the only possible gained information is that we are searching
            // for ourselves, and therefore are connecting, which
            // is useless information
            self.create_get_neighbours_fn(MessageMode::Fast()),
            Arc::new(found_callback),
            Arc::new(explored_callback),
            self.max_num_search_threads,
            self.search_timeout_sec,
            log,
        );
        to_internal_result(result)?;

        Ok(())
    }

    fn create_get_neighbours_fn(&self, mode: MessageMode) -> GetNeighboursFn {
        let neighbours_store = self.neighbours_store.clone();
        let key = self.key.clone();
        let timeout = Duration::from_secs(self.search_timeout_sec as u64);
        let message_handler_client = self.message_handler_client.clone();
        Arc::new(move |n, k: &Key| {
            if n.key == key {
                return Ok(neighbours_store.get_all());
            }

            let response = message_handler_client.send_message(
                n,
                RequestPayload::QueryRequest(k.clone()),
                timeout,
                &mode,
            );

            match response {
                Ok(ResponsePayload::QueryResponse(ref nodes)) => {
                    Ok(nodes.clone())
                }
                Ok(_) => to_internal_result(Err(ErrorKind::ResponseError(
                    "Incorrect response for query request".into(),
                )
                .into())),
                Err(err) => Err(err),
            }
        })
    }
}

impl PayloadHandler for GraphPayloadHandler {
    fn receive(
        &self,
        payload: &RequestPayload,
        sender: Option<Node>,
        message_id: u32,
    ) -> InternalResult<ResponsePayload>
    {
        remotery_scope!("graph_receive");

        info!(
            self.log,
            "Received request";
            "sender" => sender.clone()
                .map(|n| n.to_string()).unwrap_or("none".into()));

        if let Some(n) = sender {
            remotery_scope!("consider_sender_for_neighbour");
            self.neighbours_store.consider_candidate(&n, true);
        }

        match *payload {
            RequestPayload::QueryRequest(ref key) => {
                remotery_scope!("graph_query_request");
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
            RequestPayload::SearchRequest(ref key, ref mode) => {
                remotery_scope!("graph_search_request");
                trace!(
                    self.log,
                    "Received search request";
                    "key" => %key);
                Ok(ResponsePayload::SearchResponse(self.search(
                    &key,
                    &mode,
                    self.log.new(o!(
                        "message_id" => message_id,
                        "search_request" => true,
                        "search_mode" => mode.to_string(),
                    )),
                )?))
            }
            RequestPayload::ConnectRequest(ref node) => {
                remotery_scope!("graph_connect_request");
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
                remotery_scope!("graph_list_neighbours_request");
                trace!(self.log, "Recieved list neigbours request");
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
                remotery_scope!("graph_verify_request");
                trace!(self.log, "Received verify request");
                Ok(ResponsePayload::VerifyResponse())
            }
        }
    }
}
