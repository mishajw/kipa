//! Handle creating responses for requests and performing any required
//! operations

use api::{RequestPayload, ResponsePayload};
use error::*;
use message_handler::OutgoingMessageHandler;
use node::Node;

use std::sync::Arc;

#[cfg(feature = "use-graph")]
pub mod graph;

#[cfg(feature = "use-black-hole")]
pub mod black_hole;

/// Trait for any type that handles requests
pub trait PayloadHandler: Send + Sync {
    /// Process a `RequestMessage` and return the correct `ResponseMessage`
    fn receive(
        &self,
        payload: &RequestPayload,
        sender: Option<&Node>,
        outgoing_message_handler: Arc<OutgoingMessageHandler>,
        message_id: u32,
    ) -> InternalResult<ResponsePayload>;
}
