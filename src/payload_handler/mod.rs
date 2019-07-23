//! Handle creating responses for requests and performing any required
//! operations

use api::{Node, RequestPayload, ResponsePayload};
use error::*;

#[cfg(feature = "use-black-hole")]
pub mod black_hole;

#[cfg(feature = "use-random-response")]
pub mod random_response;

/// Trait for any type that handles requests
pub trait PayloadHandler: Send + Sync {
    /// Process a `RequestMessage` and return the correct `ResponseMessage`
    fn receive(
        &self,
        payload: &RequestPayload,
        sender: Option<Node>,
        message_id: u32,
    ) -> InternalResult<ResponsePayload>;
}