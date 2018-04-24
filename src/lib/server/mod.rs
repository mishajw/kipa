//! Traits for sending and receiving requests between KIPA nodes

use error::*;
use node::Node;
use api::{RequestPayload, ResponseMessage};

#[cfg(feature = "use-tcp")]
pub mod tcp;

#[cfg(feature = "use-unix-socket")]
pub mod unix_socket;

/// Create a server that can listen for requests from remote KIPA nodes and pass
/// them to `PayloadHandler`.
pub trait Server: Send + Sync {
    /// Start the server.
    fn start(&self) -> Result<()>;
}

/// Listen for requests from other KIPA nodes.
pub trait Client: Send + Sync {
    /// Send a request to another `Node` and get the `Response`.
    fn send<'a>(
        &self,
        node: &Node,
        request: RequestPayload,
    ) -> Result<ResponseMessage>;
}

/// Trait for sending requests to local KIPA daemon.
pub trait LocalClient: Send + Sync {
    /// Send a request to local KIPA daemon
    fn send<'a>(&self, request: RequestPayload) -> Result<ResponseMessage>;
}
