//! Holds the KIPA API `Request`s and `Response`s.

use key::Key;
use node::Node;

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
    QueryRequest(Key),
    /// Connect to a `Node`, and search for potential neighbours in the node's
    /// network.
    ConnectRequest(Node),
}

/// The response for a given request.
pub enum Response {
    /// Response for a [`Request::SearchRequest`].
    SearchResponse(Option<Node>),
    /// Response for a [`Request::QueryRequest`].
    QueryResponse(Vec<Node>),
    /// Response for a [`Request::ConnectRequest`]
    ConnectResponse()
}

