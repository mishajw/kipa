//! Implement `RequestHandler` but never return anything. Used for testing.

use error::*;
use request_handler::{RequestHandler, Request, Response};

pub struct BlackHoleRequestHandler {}

impl BlackHoleRequestHandler {
    /// Create a new black hole request handler
    pub fn new() -> Self {
        BlackHoleRequestHandler {}
    }
}

impl RequestHandler for BlackHoleRequestHandler {
    fn receive(&self, request: &Request) -> Result<Response> {
        match request {
            &Request::QueryRequest(_) => {
                trace!("Received query request");
                Ok(Response::QueryResponse(vec![]))
            },
            &Request::SearchRequest(_) => {
                trace!("Received search request");
                Ok(Response::SearchResponse(None))
            }
        }
    }
}

