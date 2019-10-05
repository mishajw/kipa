//! Request handling.

use api::{Node, RequestPayload, ResponsePayload};
use error::*;

#[cfg(feature = "use-black-hole")]
pub mod black_hole;

#[cfg(feature = "use-random-response")]
pub mod random_response;

/// Given a request payload, generates a response payload.
pub trait PayloadHandler: Send + Sync {
    /// Processes a `RequestMessage` and returns a `ResponseMessage`.
    fn receive(
        &self,
        payload: &RequestPayload,
        sender: Option<Node>,
        message_id: u32,
    ) -> InternalResult<ResponsePayload>;
}
