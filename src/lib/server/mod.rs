//! Traits for sending and receiving requests between KIPA nodes

use error::*;
use node::Node;
use api::{RequestMessage, RequestPayload, ResponseMessage};

use std::time::Duration;

#[cfg(use_tcp)]
pub mod tcp;

#[cfg(use_unix_socket)]
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
        request: RequestMessage,
        timeout: Duration,
    ) -> Result<ResponseMessage>;
}

/// Trait for sending requests to local KIPA daemon.
pub trait LocalClient: Send + Sync {
    /// Send a request to local KIPA daemon
    fn send<'a>(
        &self,
        request: RequestPayload,
        message_id: u32,
    ) -> Result<ResponseMessage>;
}
