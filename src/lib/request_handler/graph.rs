//! Implement `RequestHandler` using graph based searches through KIPA net.

use error::*;
use key::Key;
use node::Node;
use request_handler::{RequestHandler, Request, Response};
use global_server::GlobalSendServer;

use std::sync::Arc;

/// Contains graph search information.
pub struct GraphRequestHandler {
    #[allow(dead_code)]
    key: Key,
    remote_server: Arc<GlobalSendServer>,
    #[allow(dead_code)]
    neighbours: Vec<Node>
}

impl GraphRequestHandler {
    /// Create a new graph request handler.
    ///
    /// - `key` is the key for the local node.
    /// - `remote_server` is used for communicating with other nodes.
    /// - `initial_node` is the initial other node in KIPA network.
    pub fn new(
            key: Key,
            remote_server: Arc<GlobalSendServer>,
            initial_node: Node) -> Self {

        GraphRequestHandler {
            key: key,
            remote_server: remote_server,
            neighbours: vec![initial_node]
        }
    }
}

impl RequestHandler for GraphRequestHandler {
    fn receive(&self, request: &Request) -> Result<Response> {
        match request {
            &Request::QueryRequest(_) => {
                trace!("Received query request");
                unimplemented!();
            },
            &Request::SearchRequest(_) => {
                trace!("Received search request");
                unimplemented!();
            }
        }
    }
}

