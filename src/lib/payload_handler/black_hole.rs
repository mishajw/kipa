//! Implement `PayloadHandler` but never return anything. Used for testing

use api::{RequestPayload, ResponsePayload};
use error::*;
use message_handler::MessageHandlerClient;
use node::Node;
use payload_handler::PayloadHandler;

use slog::Logger;
use std::sync::Arc;

/// The request handler that returns nothing
pub struct BlackHolePayloadHandler {
    log: Logger,
}

impl BlackHolePayloadHandler {
    /// Create a new black hole request handler
    pub fn new(log: Logger) -> Self { BlackHolePayloadHandler { log: log } }
}

impl PayloadHandler for BlackHolePayloadHandler {
    fn receive(
        &self,
        payload: &RequestPayload,
        _sender: Option<&Node>,
        _message_handler_client: Arc<MessageHandlerClient>,
        _message_id: u32,
    ) -> InternalResult<ResponsePayload>
    {
        match payload {
            &RequestPayload::QueryRequest(_) => {
                trace!(self.log, "Received query request");
                Ok(ResponsePayload::QueryResponse(vec![]))
            }
            &RequestPayload::SearchRequest(_) => {
                trace!(self.log, "Received search request");
                Ok(ResponsePayload::SearchResponse(None))
            }
            &RequestPayload::ConnectRequest(_) => {
                trace!(self.log, "Received connect request");
                Ok(ResponsePayload::ConnectResponse())
            }
            &RequestPayload::ListNeighboursRequest() => {
                trace!(self.log, "Received list neighbours request");
                Ok(ResponsePayload::ListNeighboursResponse(vec![]))
            }
            &RequestPayload::VerifyRequest() => {
                trace!(self.log, "Received verify request");
                Ok(ResponsePayload::VerifyResponse())
            }
        }
    }
}
