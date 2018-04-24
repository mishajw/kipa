//! Handle providing `Response`s for `Request`s.

use api::{RequestPayload, ResponsePayload};
use node::Node;

use error::*;

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
    ) -> Result<ResponsePayload>;
}
