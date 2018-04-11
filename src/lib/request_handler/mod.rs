//! Handle providing `Response`s for `Request`s.

use api::{Request, Response};
use error::*;

#[cfg(feature = "use-graph")]
pub mod graph;

#[cfg(feature = "use-black-hole")]
pub mod black_hole;

/// Trait for any type that handles requests.
pub trait RequestHandler: Send + Sync {
    /// Process a `Request` and return the correct `Response`.
    fn receive(&self, req: &Request) -> Result<Response>;
}

