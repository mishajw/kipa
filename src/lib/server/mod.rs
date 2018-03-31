use error::*;
use node::Node;
use request_handler::{Request, Response};

#[cfg(feature = "use-tcp")]
pub mod tcp;

pub trait PublicServer {
    fn join(&mut self) -> Result<()>;
}

pub trait RemoteServer: Send + Sync {
    fn receive<'a>(&self, node: &Node, request: &Request) -> Result<Response>;
}

