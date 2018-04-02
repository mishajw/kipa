//! Traits for sending and receiving requests on a local machine.

use error::*;
use request_handler::{Request, Response};

#[cfg(feature = "use-unix-socket")]
pub mod unix_socket;

/// Create a server that can listen for local requests and pass onto a
/// `RequestHandler`.
pub trait LocalReceiveServer {
    /// Join on the server, waiting for all child threads to terminate.
    ///
    /// If there are no child threads, do nothing.
    fn join(&mut self) -> Result<()>;
}

/// Trait for sending requests to local KIPA daemon.
pub trait LocalSendServer: Send + Sync {
    /// Send a request to local KIPA daemon
    fn receive<'a>(&self, request: &Request) -> Result<Response>;
}

