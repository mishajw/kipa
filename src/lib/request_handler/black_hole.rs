//! Implement `RequestHandler` but never return anything. Used for testing.

use error::*;
use api::{RequestMessage, RequestPayload, ResponsePayload};
use request_handler::RequestHandler;

use slog::Logger;

/// The request handler that returns nothing.
pub struct BlackHoleRequestHandler {
    log: Logger,
}

impl BlackHoleRequestHandler {
    /// Create a new black hole request handler
    pub fn new(log: Logger) -> Self {
        BlackHoleRequestHandler { log: log }
    }
}

impl RequestHandler for BlackHoleRequestHandler {
    fn receive(&self, request: &RequestMessage) -> Result<ResponsePayload> {
        match request.payload {
            RequestPayload::QueryRequest(_) => {
                trace!(self.log, "Received query request");
                Ok(ResponsePayload::QueryResponse(vec![]))
            }
            RequestPayload::SearchRequest(_) => {
                trace!(self.log, "Received search request");
                Ok(ResponsePayload::SearchResponse(None))
            }
            RequestPayload::ConnectRequest(_) => {
                trace!(self.log, "Received connect request");
                Ok(ResponsePayload::ConnectResponse())
            }
        }
    }
}
