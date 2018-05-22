//! Handle providing `Response`s for `Request`s.

use api::{RequestPayload, ResponsePayload};
use error::*;
use message_handler::PayloadClient;
use node::Node;

use std::sync::Arc;

#[cfg(feature = "use-graph")]
pub mod graph;

#[cfg(feature = "use-black-hole")]
pub mod black_hole;

/// Trait for any type that handles requests.
pub trait PayloadHandler: Send + Sync {
    /// Process a `RequestMessage` and return the correct `ResponseMessage`.
    fn receive(
        &self,
        payload: &RequestPayload,
        sender: Option<&Node>,
        payload_client: Arc<PayloadClient>,
        message_id: u32,
    ) -> Result<ResponsePayload>;
}
