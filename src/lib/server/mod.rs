//! Concepts for communicating between KIPA nodes.
//!
//! Contains two different traits for receiving and sending requests.

use error::*;
use node::Node;
use request_handler::{Request, Response};

#[cfg(feature = "use-tcp")]
pub mod tcp;

/// Create a server that can listen for requests and pass onto a
/// `RequestHandler`.
pub trait ReceiveServer {
    /// Join on the server, waiting for all child threads to terminate.
    ///
    /// If there are no child threads, do nothing.
    fn join(&mut self) -> Result<()>;
}

/// Functionality for sending requests to other KIPA servers.
pub trait SendServer: Send + Sync {
    /// Send a request to another `Node` and get the `Response`.
    fn receive<'a>(&self, node: &Node, request: &Request) -> Result<Response>;
}

