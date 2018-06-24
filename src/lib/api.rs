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

/// Generic message type that holds a payload and a sender.
pub struct Message<T> {
    /// The payload of the message
    pub payload: T,
    /// The sender of the message
    pub sender: MessageSender,
    /// The identifier of the message
    pub id: u32,
    /// The version of the sender of the message
    pub version: String,
}

impl<T> Message<T> {
    /// Construct a new message with a payload and sender
    pub fn new(
        payload: T,
        sender: MessageSender,
        id: u32,
        version: String,
    ) -> Self
    {
        Message {
            payload,
            sender,
            id,
            version,
        }
    }
}

/// Messages for requests with request payloads
pub type RequestMessage = Message<RequestPayload>;

/// Messages for responses with response payloads
pub type ResponseMessage = Message<ApiResult<ResponsePayload>>;

/// A request for the request handler.
pub enum RequestPayload {
    /// Request a search for some key.
    ///
    /// This prompts the node to perform a search in the KIPA network it is
    /// connected to, looking for the [`Node`] that owns the [`Key`] provided.
    SearchRequest(Key),
    /// Request a query for some key.
    ///
    /// This returns the [`Node`]s that the local node is connected to, that
    /// are closest to the [`Key`] given.
    QueryRequest(Key),
    /// Connect to a `Node`, and search for potential neighbours in the node's
    /// network.
    ConnectRequest(Node),
    /// List all of the neighbour `Node`s.
    ListNeighboursRequest(),
}

/// The response for a given request
pub enum ResponsePayload {
    /// Response for a [`Request::SearchRequest`].
    SearchResponse(Option<Node>),
    /// Response for a [`Request::QueryRequest`].
    QueryResponse(Vec<Node>),
    /// Response for a [`Request::ConnectRequest`]
    ConnectResponse(),
    /// Response for a [`Request::ListNeighboursRequest`].
    ListNeighboursResponse(Vec<Node>),
}

impl RequestPayload {
    /// Check if the request is visible in a API visibility.
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

/// Types of API errors, used as error codes when reporting to user
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
    /// Create with message
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

/// Store the sender of a request
#[derive(Clone)]
pub enum MessageSender {
    /// The request was sent from an external node
    Node(Node),
    /// The request was sent from the command line argument tool
    Cli(),
}

impl fmt::Display for MessageSender {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MessageSender::Node(ref n) => n.fmt(f),
            MessageSender::Cli() => write!(f, "CLI"),
        }
    }
}
