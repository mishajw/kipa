//! Handle providing `Response`s for `Request`s.

use error::*;
use key::Key;
use node::Node;

#[cfg(feature = "use-graph")]
pub mod graph;

/// A request for the request handler.
pub enum Request {
    /// Request a search for some key.
    ///
    /// This prompts the node to perform a search in the KIPA network it is
    /// connected to, looking for the [`Node`] that owns the [`Key`] provided.
    SearchRequest(Key),
    /// Request a query for some key.
    ///
    /// This returns the [`Node`]s that the local node is connected to, that are
    /// closest to the [`Key`] given.
    QueryRequest(Key)
}

/// The response for a given request.
pub enum Response {
    /// Response for a [`Request::SearchRequest`].
    SearchResponse(Option<Node>),
    /// Response for a [`Request::QueryRequest`].
    QueryResponse(Vec<Node>)
}

/// Trait for any type that handles requests.
pub trait RequestHandler: Send + Sync {
    /// Process a [`Request`] and return the correct [`Response`].
    fn receive(&self, req: &Request) -> Result<Response>;
}

