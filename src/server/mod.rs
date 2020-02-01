//! Traits for sending and receiving requests between KIPA nodes

use api::Node;
use error::*;

use std::thread::JoinHandle;
use std::time::Duration;

#[cfg(any(feature = "use-tcp", feature = "use-unix-socket"))]
pub mod socket_server;

#[cfg(feature = "use-tcp")]
pub mod tcp;

#[cfg(feature = "use-unix-socket")]
pub mod unix_socket;

/// Create a server that can listen for requests from remote KIPA nodes and pass
/// them to `PayloadHandler`
pub trait Server: Send + Sync {
    /// Start the server
    fn start(&self) -> InternalResult<JoinHandle<()>>;
    // TODO: Start function should be able to consume `self`, but we can't due
    // to referencing types as `: Server` which is not sized. This leads to a
    // redundant clone in implementations of `start()`, which is inexpensive but
    // messy
}

/// Listen for requests from other KIPA nodes
pub trait Client: Send + Sync {
    /// Send a request to another `Node` and get the `Response`
    fn send(&self, node: &Node, request_data: &[u8], timeout: Duration) -> InternalResult<Vec<u8>>;
}

/// Create a server that can listen for requests from local clients
///
/// Identical to `Server`.
pub trait LocalServer: Send + Sync {
    /// Start the server
    fn start(&self) -> Result<JoinHandle<()>>;
}

/// Trait for sending requests to local KIPA daemon
pub trait LocalClient: Send + Sync {
    /// Send a request to local KIPA daemon
    fn send(&self, request_data: &[u8]) -> Result<Vec<u8>>;
}
