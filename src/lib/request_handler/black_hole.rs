//! Implement `RequestHandler` but never return anything. Used for testing.

use error::*;
use api::{RequestPayload, ResponsePayload, RequestMessage};
use request_handler::RequestHandler;

/// The request handler that returns nothing.
pub struct BlackHoleRequestHandler {}

impl BlackHoleRequestHandler {
    /// Create a new black hole request handler
    pub fn new() -> Self {
        BlackHoleRequestHandler {}
    }
}

impl RequestHandler for BlackHoleRequestHandler {
    fn receive(&self, request: &RequestMessage) -> Result<ResponsePayload> {
        match request.payload {
            RequestPayload::QueryRequest(_) => {
                trace!("Received query request");
                Ok(ResponsePayload::QueryResponse(vec![]))
            }
            RequestPayload::SearchRequest(_) => {
                trace!("Received search request");
                Ok(ResponsePayload::SearchResponse(None))
            }
            RequestPayload::ConnectRequest(_) => {
                trace!("Received connect request");
                Ok(ResponsePayload::ConnectResponse())
            }
        }
    }
}
