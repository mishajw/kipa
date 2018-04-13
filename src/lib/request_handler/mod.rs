//! Handle providing `Response`s for `Request`s.

use api::{RequestMessage, ResponsePayload};
use error::*;

#[cfg(feature = "use-graph")]
pub mod graph;

#[cfg(feature = "use-black-hole")]
pub mod black_hole;

/// Trait for any type that handles requests.
pub trait RequestHandler: Send + Sync {
    /// Process a `RequestMessage` and return the correct `ResponseMessage`.
    fn receive(&self, req: &RequestMessage) -> Result<ResponsePayload>;
}
