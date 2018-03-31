use error::*;
use key::Key;
use node::Node;
use request_handler::{RequestHandler, Request, Response};
use server::RemoteServer;

pub struct GraphRequestHandler {
    #[allow(dead_code)]
    key: Key,
    remote_server: Box<RemoteServer>,
    #[allow(dead_code)]
    neighbours: Vec<Node>
}

impl GraphRequestHandler {
    pub fn new(
            key: Key,
            remote_server: Box<RemoteServer>,
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

