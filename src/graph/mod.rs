//! Implement `PayloadHandler` using graph based searches through KIPA net

#[macro_use]
mod search_callback;

pub mod neighbour_gc;
mod neighbours_store;
mod search;
mod search_node;

pub use graph::neighbours_store::{
    NeighboursStore, DEFAULT_ANGLE_WEIGHTING, DEFAULT_DISTANCE_WEIGHTING,
    DEFAULT_MAX_NUM_NEIGHBOURS,
};

use api::{Address, Key, Node};
use api::{RequestPayload, ResponsePayload};
use error::*;
use graph::search::{GraphSearch, SearchParams};
use key_space_manager::KeySpaceManager;
use log_event::LogEvent;
use message_handler::MessageHandlerClient;
use payload_handler::PayloadHandler;

use graph::search_callback::SearchCallback;
use graph::search_callback::SearchCallbackAction;
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
    local_key: Key,
    search_breadth: usize,
    connect_search_breadth: usize,
    max_num_search_threads: usize,
    search_timeout: Duration,
    message_handler_client: Arc<MessageHandlerClient>,
    neighbours_store: Arc<NeighboursStore>,
    graph_search: Arc<GraphSearch>,
    log: Logger,
}

impl GraphPayloadHandler {
    /// Create a new graph request handler
    ///
    /// - `local_key` is the key for the local node.
    /// - `remote_server` is used for communicating with other nodes.
    /// - `initial_node` is the initial other node in KIPA network.
    pub fn new(
        local_key: Key,
        search_breadth: usize,
        connect_search_breadth: usize,
        max_num_search_threads: usize,
        search_timeout_sec: usize,
        message_handler_client: Arc<MessageHandlerClient>,
        key_space_manager: Arc<KeySpaceManager>,
        neighbours_store: Arc<NeighboursStore>,
        search_thread_pool_size: usize,
        log: Logger,
    ) -> Self {
        GraphPayloadHandler {
            local_key,
            search_breadth,
            connect_search_breadth,
            max_num_search_threads,
            search_timeout: Duration::from_secs(search_timeout_sec as u64),
            message_handler_client,
            graph_search: Arc::new(GraphSearch::new(key_space_manager, search_thread_pool_size)),
            neighbours_store,
            log,
        }
    }

    fn search(&self, search_key: &Key, log: Logger) -> InternalResult<Option<Node>> {
        remotery_scope!("graph_search");
        let callback = SearchRequestCallback {
            search_key: search_key.clone(),
            message_handler_client: self.message_handler_client.clone(),
            timeout: self.search_timeout,
            wrapped_client: WrappedClient {
                message_handler_client: self.message_handler_client.clone(),
                local_key: self.local_key.clone(),
                neighbours_store: self.neighbours_store.clone(),
                timeout: self.search_timeout,
                log: log.new(o!("wrapped_client" => true)),
            },
            log: log.new(o!("search_callback" => true)),
        };
        let search_result = self.graph_search.search(
            &search_key,
            vec![Node::new(
                // Address never used, when querying for self we return results straight from a
                // NeighboursStore.
                Address::new(vec![0, 0, 0, 0], 0),
                self.local_key.clone(),
            )],
            callback,
            SearchParams {
                breadth: self.search_breadth,
                max_num_active_threads: self.max_num_search_threads,
                timeout: self.search_timeout,
            },
            log.clone(),
        );
        match search_result {
            Ok(Some(ref node)) => LogEvent::search_succeeded(node, &log),
            Ok(None) => LogEvent::search_not_found(search_key, &log),
            Err(_) => LogEvent::search_error(search_key, &log),
        };
        to_internal_result(search_result)
    }

    fn connect(&self, node: &Node, log: Logger) -> InternalResult<()> {
        remotery_scope!("graph_connect");
        // Check we can contact the node first. This failing is the only way we report that the
        // connection was a failure, as we don't use the result of the search below.
        self.message_handler_client.send_request(
            node,
            RequestPayload::VerifyRequest(),
            self.search_timeout,
        )?;
        let callback = ConnectRequestCallback {
            neighbours_store: self.neighbours_store.clone(),
            wrapped_client: WrappedClient {
                message_handler_client: self.message_handler_client.clone(),
                local_key: self.local_key.clone(),
                neighbours_store: self.neighbours_store.clone(),
                timeout: self.search_timeout,
                log: log.new(o!("wrapped_client" => true)),
            },
            log: log.new(o!("connect_callback" => true)),
        };
        let result: Result<Option<()>> = self.graph_search.search(
            &self.local_key,
            vec![node.clone()],
            callback,
            SearchParams {
                breadth: self.connect_search_breadth,
                max_num_active_threads: self.max_num_search_threads,
                timeout: self.search_timeout,
            },
            log,
        );
        to_internal_result(result)?;
        Ok(())
    }
}

impl PayloadHandler for GraphPayloadHandler {
    fn receive(
        &self,
        payload: &RequestPayload,
        sender: Option<Node>,
        message_id: u32,
    ) -> InternalResult<ResponsePayload> {
        remotery_scope!("graph_receive");
        let log = self.log.new(o!("message_id" => message_id));

        info!(
            log,
            "Received request";
            "sender" => sender.clone()
                .map(|n| n.to_string()).unwrap_or_else(|| "none".into()));
        LogEvent::receive_request(payload, &log);

        if let Some(n) = sender {
            remotery_scope!("consider_sender_for_neighbour");
            self.neighbours_store.consider_candidate(&n, true);
        }

        match *payload {
            RequestPayload::QueryRequest(ref key) => {
                remotery_scope!("graph_query_request");
                trace!(
                    log,
                    "Received query request";
                    "key" => %key);
                let nodes = self.neighbours_store.get_n_closest(key, 3);
                trace!(
                    log,
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
                remotery_scope!("graph_search_request");
                trace!(
                    log,
                    "Received search request";
                    "key" => %key);
                Ok(ResponsePayload::SearchResponse(self.search(
                    &key,
                    log.new(o!(
                        "search_request" => true,
                    )),
                )?))
            }
            RequestPayload::ConnectRequest(ref node) => {
                remotery_scope!("graph_connect_request");
                trace!(
                    log,
                    "Received connect request";
                    "node" => %node);
                self.connect(
                    node,
                    log.new(o!(
                        "connect_request" => true
                    )),
                )?;
                Ok(ResponsePayload::ConnectResponse())
            }
            RequestPayload::ListNeighboursRequest() => {
                remotery_scope!("graph_list_neighbours_request");
                trace!(log, "Received list neighbours request");
                let neighbours = self.neighbours_store.get_all();
                trace!(
                    log,
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
                trace!(log, "Received verify request");
                Ok(ResponsePayload::VerifyResponse())
            }
        }
    }
}

/// Callback for performing `SearchRequest` searches.
struct SearchRequestCallback {
    search_key: Key,
    message_handler_client: Arc<MessageHandlerClient>,
    wrapped_client: WrappedClient,
    timeout: Duration,
    log: Logger,
}

impl SearchCallback<Node> for SearchRequestCallback {
    fn get_neighbours(&self, node: &Node, search_key: &Key) -> InternalResult<Vec<Node>> {
        self.wrapped_client.get_neighbours(node, search_key)
    }

    fn found_node(&self, node: &Node) -> Result<SearchCallbackAction<Node>> {
        trace!(self.log, "Found node"; "node" => %node);
        LogEvent::search_found(node, &self.log);
        if node.key != self.search_key {
            return Ok(SearchCallbackAction::Continue());
        }

        // Send verification message to the node to ensure that the discovered IP address is owned
        // by the requested key.
        if let Err(err) = self.message_handler_client.send_request(
            node,
            RequestPayload::VerifyRequest(),
            self.timeout,
        ) {
            // If the verification fails, then log a warning but continue the search. If we exit
            // here, it is possible to easily attack a search by returning fake nodes whenever you
            // receive a query.
            warn!(
                self.log, "Error when sending verification message after finding correct node";
                "err" => %err, "node" => %node);
            LogEvent::search_verification_failed(node, &self.log);
            return Ok(SearchCallbackAction::Continue());
        }

        info!(self.log, "Search success"; "node" => %node);
        Ok(SearchCallbackAction::Return(node.clone()))
    }

    fn explored_node(&self, node: &Node) -> Result<SearchCallbackAction<Node>> {
        trace!(self.log, "Explored node"; "node" => %node);
        LogEvent::search_explored(node, &self.log);
        Ok(SearchCallbackAction::Continue())
    }
}

/// Callback for performing `ConnectRequest` searches.
struct ConnectRequestCallback {
    neighbours_store: Arc<NeighboursStore>,
    wrapped_client: WrappedClient,
    log: Logger,
}

impl SearchCallback<()> for ConnectRequestCallback {
    fn get_neighbours(&self, node: &Node, search_key: &Key) -> InternalResult<Vec<Node>> {
        self.wrapped_client.get_neighbours(node, search_key)
    }

    fn found_node(&self, node: &Node) -> Result<SearchCallbackAction<()>> {
        trace!(self.log, "Found node"; "node" => %node);
        self.neighbours_store.consider_candidate(node, false);
        Ok(SearchCallbackAction::Continue())
    }

    fn explored_node(&self, node: &Node) -> Result<SearchCallbackAction<()>> {
        trace!(self.log, "Explored node"; "node" => %node);
        Ok(SearchCallbackAction::Continue())
    }
}

/// Wraps a `MessageHandlerClient` to get neighbours.
struct WrappedClient {
    message_handler_client: Arc<MessageHandlerClient>,
    local_key: Key,
    neighbours_store: Arc<NeighboursStore>,
    timeout: Duration,
    log: Logger,
}

impl WrappedClient {
    fn get_neighbours(&self, query_node: &Node, search_key: &Key) -> InternalResult<Vec<Node>> {
        if query_node.key == self.local_key {
            return Ok(self.neighbours_store.get_all());
        }

        let response = self.message_handler_client.send_request(
            query_node,
            RequestPayload::QueryRequest(search_key.clone()),
            self.timeout,
        );

        match response {
            Ok(ResponsePayload::QueryResponse(ref nodes)) => {
                LogEvent::query_succeeded(query_node, nodes, &self.log);
                Ok(nodes.clone())
            }
            Ok(_) => to_internal_result(Err(ErrorKind::ResponseError(
                "Incorrect response for query request".into(),
            )
            .into())),
            Err(err) => {
                LogEvent::query_failed(query_node, &self.log);
                Err(err)
            }
        }
    }
}
