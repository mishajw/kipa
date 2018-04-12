//! Implement `RequestHandler` using graph based searches through KIPA net.

mod search;
mod neighbours_store;
mod key_space;

use error::*;
use global_server::GlobalSendServer;
use key::Key;
use request_handler::graph::neighbours_store::NeighboursStore;
use request_handler::graph::search::GraphSearch;
use request_handler::{RequestHandler, Request, Response};

use std::sync::{Arc, Mutex};

/// Contains graph search information.
pub struct GraphRequestHandler {
    neighbours_store: Arc<Mutex<NeighboursStore>>,
    graph_search: Arc<GraphSearch>
}

impl GraphRequestHandler {
    /// Create a new graph request handler.
    ///
    /// - `key` is the key for the local node.
    /// - `remote_server` is used for communicating with other nodes.
    /// - `initial_node` is the initial other node in KIPA network.
    pub fn new(
            key: Key,
            remote_server: Arc<GlobalSendServer>) -> Self {

        let remote_server_clone = remote_server.clone();
        let graph_search = GraphSearch::new(Arc::new(move |n, k: &Key| {
            let response = remote_server_clone
                .receive(n, &Request::QueryRequest(k.clone()))?;

            match response {
                Response::QueryResponse(ref nodes) => Ok(nodes.clone()),
                _ => Err(ErrorKind::ResponseError(
                        "Incorrect response for query request".into()).into())
            }
        }));

        let neighbours_store = NeighboursStore::new(key.clone(), 3);

        GraphRequestHandler {
            neighbours_store: Arc::new(Mutex::new(neighbours_store)),
            graph_search: Arc::new(graph_search)
        }
    }
}

impl RequestHandler for GraphRequestHandler {
    fn receive(&self, request: &Request) -> Result<Response> {
        match request {
            &Request::QueryRequest(ref key) => {
                trace!("Received query request");
                Ok(Response::QueryResponse(
                    self.neighbours_store
                        .lock().unwrap()
                        .get_n_closest(key, 1)))
            },
            &Request::SearchRequest(ref key) => {
                trace!("Received search request for key {}", key);
                let initial_nodes =
                    self.neighbours_store.lock().unwrap().get_all();
                Ok(Response::SearchResponse(
                    self.graph_search.search(key, initial_nodes)?))
            }
            &Request::ConnectRequest(ref node) => {
                trace!("Received connect request for node {}", node);
                self.neighbours_store.lock().unwrap().consider_candidate(node);
                Ok(Response::ConnectResponse())
            }
        }
    }
}

