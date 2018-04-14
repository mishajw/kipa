//! Holds the KIPA API `Request`s and `Response`s.

use key::Key;
use node::Node;

use std::fmt;

/// A request for the request handler.
pub enum RequestPayload {
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
pub enum ResponsePayload {
    /// Response for a [`Request::SearchRequest`].
    SearchResponse(Option<Node>),
    /// Response for a [`Request::QueryRequest`].
    QueryResponse(Vec<Node>),
    /// Response for a [`Request::ConnectRequest`]
    ConnectResponse(),
}

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
        match self {
            &MessageSender::Node(ref n) => n.fmt(f),
            &MessageSender::Cli() => write!(f, "CLI"),
        }
    }
}

/// Generic message type that holds a payload and a sender.
pub struct Message<T> {
    /// The payload of the message.
    pub payload: T,
    /// The sender of the message.
    pub sender: MessageSender,
}

impl<T> Message<T> {
    /// Construct a new message with a payload and sender.
    pub fn new(payload: T, sender: MessageSender) -> Self {
        Message {
            payload: payload,
            sender: sender,
        }
    }
}

/// Messages for requests with request payloads
pub type RequestMessage = Message<RequestPayload>;

/// Messages for responses with response payloads
pub type ResponseMessage = Message<ResponsePayload>;
