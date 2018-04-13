//! Implementation of servers using TCP sockets.

use api::{RequestPayload, ResponseMessage};
use data_transformer::DataTransformer;
use error::*;
use global_server::{GlobalReceiveServer, GlobalSendServer};
use node::Node;
use request_handler::RequestHandler;
use server::{ReceiveServer, SendServer};

use std::mem::swap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

/// Server that listens for global requests on a specified TCP socket.
pub struct TcpGlobalReceiveServer {
    thread: Option<thread::JoinHandle<()>>,
}

impl TcpGlobalReceiveServer {
    /// Create a new TCP server.
    /// - `request_hanlder` is what to send requests to.
    /// - `data_transformer` used to decode requests.
    /// - `port` the port used to listen on.
    pub fn new(
        request_handler: Arc<RequestHandler>,
        data_transformer: Arc<DataTransformer>,
        port: u16,
    ) -> Result<Self> {
        let local_address =
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
        let listener = TcpListener::bind(&local_address)
            .chain_err(|| "Error on bind to TCP socket")?;
        trace!("Setting up server on port {}", port);

        let t = thread::spawn(move || {
            listener.incoming().for_each(|s| {
                Self::handle_socket_result(
                    s.chain_err(|| "Error on creating socket"),
                    request_handler.clone(),
                    data_transformer.clone(),
                )
            });
        });

        Ok(TcpGlobalReceiveServer { thread: Some(t) })
    }
}

impl GlobalReceiveServer for TcpGlobalReceiveServer {
    fn join(&mut self) -> Result<()> {
        let mut thread: Option<thread::JoinHandle<()>> = None;
        swap(&mut self.thread, &mut thread);
        match thread.map(|t| t.join()) {
            Some(Ok(())) => Ok(()),
            Some(Err(_)) => Err(ErrorKind::JoinError(
                "Error on joining server thread".into(),
            ).into()),
            None => {
                Err(ErrorKind::JoinError("Thread already joined".into()).into())
            }
        }
    }
}

impl ReceiveServer for TcpGlobalReceiveServer {
    type SocketType = TcpStream;
}

/// Implementation of sending global requests to TCP servers.
pub struct TcpGlobalSendServer {
    data_transformer: Arc<DataTransformer>,
}

impl TcpGlobalSendServer {
    /// Create a new sender, which uses a `DataTransformer` to serialize packets
    /// before going on the line.
    pub fn new(data_transformer: Arc<DataTransformer>) -> Self {
        TcpGlobalSendServer {
            data_transformer: data_transformer,
        }
    }
}

impl SendServer for TcpGlobalSendServer {
    type SocketType = TcpStream;

    fn create_socket(&self, node: &Node) -> Result<TcpStream> {
        TcpStream::connect(&node.address.get_socket_addr())
            .chain_err(|| "Error on trying to connect to node")
    }
}

impl GlobalSendServer for TcpGlobalSendServer {
    fn receive<'a>(
        &self,
        node: &Node,
        request: RequestPayload,
    ) -> Result<ResponseMessage> {
        SendServer::receive(self, node, request, &*self.data_transformer)
    }
}
