//! Defines the API used to communicate from daemon-to-daemon, and from
//! CLI-to-daemon
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

use key::Key;
use node::Node;

use std::fmt;

/// Wraps a message with security guarantees: secrecy and authenticity
pub struct SecureMessage {
    /// The sender of the message
    pub sender: Node,
    /// Signature of the decrypted body, signed by sender's private key
    pub body_signature: Vec<u8>,
    /// The contents of the body encrypted with the recipient's public key
    pub encrypted_body: Vec<u8>,
}

impl SecureMessage {
    #[allow(missing_docs)]
    pub fn new(
        sender: Node,
        body_signature: Vec<u8>,
        encrypted_body: Vec<u8>,
    ) -> Self
    {
        SecureMessage {
            sender,
            body_signature,
            encrypted_body,
        }
    }
}

/// Message passed between nodes, with a generic payload
///
/// Holds metadata about the payload, including the sender and the message
/// identifier.
pub struct MessageBody<T> {
    /// The payload of the message
    pub payload: T,
    /// The identifier of the message
    pub id: u32,
    /// The version of the sender of the message
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
pub type ResponseBody = MessageBody<ApiResult<ResponsePayload>>;

/// Different types of requests and their payloads
pub enum RequestPayload {
    /// Search for a key in the network
    ///
    /// This prompts the node to perform a search in the KIPA network it is
    /// connected to, looking for the [`Node`] that owns the [`Key`] provided.
    SearchRequest(Key),
    /// Query for the closest known nodes to some key (in key space)
    ///
    /// This returns the [`Node`]s that the local node is connected to, that
    /// are closest to the [`Key`] given.
    QueryRequest(Key),
    /// Connect to a network through a node that is already connected
    ConnectRequest(Node),
    /// List all neighbour nodes
    ListNeighboursRequest(),
}

/// The response for a given request
pub enum ResponsePayload {
    /// Response for a
    /// [`SearchRequest`](./enum.RequestPayload.html#variant.SearchRequest)
    SearchResponse(Option<Node>),
    /// Response for a
    /// [`QueryResponse`](./enum.RequestPayload.html#variant.QueryResponse)
    QueryResponse(Vec<Node>),
    /// Response for a
    /// [`ConnectRequest`](./enum.RequestPayload.html#variant.ConnectRequest)
    ConnectResponse(),
    /// Response for a
    /// [`ListNeighboursRequest`](
    /// ./enum.RequestPayload.html#variant.ListNeighboursRequest)
    ListNeighboursResponse(Vec<Node>),
}

impl RequestPayload {
    /// Check if the request is visible in a API visibility
    pub fn is_visible(&self, visibility: &ApiVisibility) -> bool {
        match *self {
            RequestPayload::SearchRequest(_) => {
                visibility == &ApiVisibility::Local()
            }
            RequestPayload::QueryRequest(_) => {
                visibility == &ApiVisibility::Global()
            }
            RequestPayload::ConnectRequest(_) => {
                visibility == &ApiVisibility::Local()
            }
            RequestPayload::ListNeighboursRequest() => {
                visibility == &ApiVisibility::Local()
            }
        }
    }
}

/// Possible API errors
#[derive(Clone, Debug)]
pub enum ApiErrorType {
    /// No error occurred
    NoError = 0,
    /// Error in parsing user input
    Parse = 1,
    /// Error in configuration of daemon/CLI
    Configuration = 2,
    /// Error caused by an external library/tool
    External = 3,
    /// Misc errors that are not exposed to user
    Internal = 4,
}

/// Error returned when a request has failed
#[derive(Clone, Debug)] // Derive `Debug` to return from main function
pub struct ApiError {
    /// Description of the error
    pub message: String,
    /// Type of the error
    pub error_type: ApiErrorType,
}

impl ApiError {
    #[allow(missing_docs)]
    pub fn new(message: String, error_type: ApiErrorType) -> Self {
        ApiError {
            message,
            error_type,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ApiError({})", self.message)
    }
}

/// Result for `ApiError`s
pub type ApiResult<T> = Result<T, ApiError>;

/// The visibility of an API call
#[derive(PartialEq)]
pub enum ApiVisibility {
    /// The API call is available for local connections from the CLI
    Local(),
    /// The API call is available for remote connections from other KIPA nodes
    Global(),
}
impl Eq for ApiVisibility {}
