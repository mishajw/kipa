//! Handle providing `Response`s for `Request`s.

use api::{RequestPayload, ResponsePayload};
use node::Node;
use message_handler::PayloadClient;
use error::*;

use std::sync::Arc;

#[cfg(use_graph)]
pub mod graph;

#[cfg(use_black_hole)]
pub mod black_hole;

/// Trait for any type that handles requests.
pub trait PayloadHandler: Send + Sync {
    /// Process a `RequestMessage` and return the correct `ResponseMessage`.
    fn receive(
        &self,
        payload: &RequestPayload,
        sender: Option<&Node>,
        payload_client: Arc<PayloadClient>,
    ) -> Result<ResponsePayload>;
}
