//! Defines the API used to communicate between instances.
//!
//! API endpoints are defined and documented by the [`RequestPayload`] and
//! [`ResponsePayload`] variants.
//!
//! Both daemon-to-daemon and CLI-to-daemon communication use the same
//! constructs. Each request is tied with a [visibility] scope, allowing some
//! types of requests to only be visible from the CLI or from another node.
//!
//! [visibility]: ./enum.ApiVisibility.html
//! [`RequestPayload`]: ./enum.RequestPayload.html
//! [`ResponsePayload`]: ./enum.ResponsePayload.html

pub use serde::Serialize;

pub mod address;
mod key;
mod key_space;
mod node;
pub use api::address::Address;
pub use api::key::{Key, SecretKey};
pub use api::key_space::KeySpace;
pub use api::node::Node;

pub mod error;
pub mod request;

/// Message passed between nodes, with a generic payload.
///
/// Holds metadata about the payload, including the sender and the message
/// identifier.
pub struct MessageBody<T> {
    /// The payload of the message.
    pub payload: T,
    /// The identifier of the message.
    pub id: u32,
    /// The version of the sender of the message.
    pub version: String,
}

impl<T> MessageBody<T> {
    #[allow(missing_docs)]
    pub fn new(payload: T, id: u32, version: String) -> Self {
        MessageBody {
            payload,
            id,
            version,
        }
    }
}

#[allow(missing_docs)]
pub type RequestBody = MessageBody<RequestPayload>;
#[allow(missing_docs)]
pub type ResponseBody = MessageBody<error::ApiResult<ResponsePayload>>;

/// Different types of requests and their payloads.
#[derive(Clone, Serialize, PartialEq, Eq)]
pub enum RequestPayload {
    /// Search for a key in the network.
    ///
    /// This prompts the node to perform a search in the KIPA network it is
    /// connected to, looking for the [`Node`] that owns the [`Key`] provided.
    SearchRequest(Key),
    /// Query for the closest known nodes to some key (in key space).
    ///
    /// This returns the [`Node`]s that the local node is connected to, that
    /// are closest to the [`Key`] given.
    QueryRequest(Key),
    /// Connect to a network through a node that is already connected.
    ConnectRequest(Node),
    /// List all neighbour nodes.
    ListNeighboursRequest(),
    /// Verify that a node is alive, and owned by a specific key.
    ///
    /// Neither the request or response contain any fields. This is because the
    /// response will be signed by the correct key, and therefore the
    /// verification is correct. And due to message identifiers, the
    /// verification is known to be up-to-date.
    VerifyRequest(),
}

/// The response for a given request.
pub enum ResponsePayload {
    /// Response for a
    /// [`SearchRequest`](./enum.RequestPayload.html#variant.SearchRequest).
    SearchResponse(Option<Node>),
    /// Response for a
    /// [`QueryResponse`](./enum.RequestPayload.html#variant.QueryResponse).
    QueryResponse(Vec<Node>),
    /// Response for a
    /// [`ConnectRequest`](./enum.RequestPayload.html#variant.ConnectRequest).
    ConnectResponse(),
    /// Response for a
    /// [`ListNeighboursRequest`](
    /// ./enum.RequestPayload.html#variant.ListNeighboursRequest).
    ListNeighboursResponse(Vec<Node>),
    /// Response for a
    /// [`VerifyRequest`](./enum.RequestPayload.html#variant.VerifyRequest).
    VerifyResponse(),
}

impl RequestPayload {
    /// Check if the request is visible in a API visibility.
    pub fn is_visible(&self, visibility: &ApiVisibility) -> bool {
        match *self {
            RequestPayload::SearchRequest(..) => visibility == &ApiVisibility::Local(),
            RequestPayload::QueryRequest(_) => visibility == &ApiVisibility::Global(),
            RequestPayload::ConnectRequest(_) => visibility == &ApiVisibility::Local(),
            RequestPayload::ListNeighboursRequest() => visibility == &ApiVisibility::Local(),
            RequestPayload::VerifyRequest() => visibility == &ApiVisibility::Global(),
        }
    }
}

/// The visibility of an API call.
#[derive(PartialEq, Eq)]
pub enum ApiVisibility {
    /// The API call is available for local connections from the CLI.
    Local(),
    /// The API call is available for remote connections from other KIPA nodes.
    Global(),
}
