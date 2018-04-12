//! Traits for sending and receiving requests between KIPA nodes

use error::*;
use node::Node;
use api::{Request, Response};

#[cfg(feature = "use-tcp")]
pub mod tcp;

/// Create a server that can listen for requests from remote KIPA nodes and pass =
/// them to `RequestHandler`.
pub trait GlobalReceiveServer {
    /// Join on the server, waiting for all child threads to terminate.
    ///
    /// If there are no child threads, do nothing.
    fn join(&mut self) -> Result<()>;
}

/// Listen for requests from other KIPA nodes.
pub trait GlobalSendServer: Send + Sync {
    /// Send a request to another `Node` and get the `Response`.
    fn receive<'a>(&self, node: &Node, request: &Request) -> Result<Response>;
}
