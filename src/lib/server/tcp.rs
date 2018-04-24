//! Implementation of servers using TCP sockets.

use api::{ApiVisibility, MessageSender, RequestMessage, RequestPayload,
          ResponseMessage};
use data_transformer::DataTransformer;
use error::*;
use server::{Client, Server};
use node::Node;
use message_handler::MessageHandler;
use socket_server::{SocketClient, SocketServer};

use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;

use slog::Logger;

/// Server that listens for global requests on a specified TCP socket.
pub struct TcpGlobalServer {
    message_handler: Arc<MessageHandler>,
    data_transformer: Arc<DataTransformer>,
    local_node: Node,
    log: Logger,
}

impl TcpGlobalServer {
    /// Create a new TCP server.
    /// - `request_hanlder` is what to send requests to.
    /// - `data_transformer` used to decode requests.
    /// - `port` the port used to listen on.
    pub fn new(
        message_handler: Arc<MessageHandler>,
        data_transformer: Arc<DataTransformer>,
        local_node: Node,
        log: Logger,
    ) -> Self {
        TcpGlobalServer {
            message_handler: message_handler,
            data_transformer: data_transformer,
            local_node: local_node,
            log: log,
        }
    }
}

impl Server for TcpGlobalServer {
    fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            self.local_node.address.port,
        )).chain_err(|| "Error on bind to TCP socket")?;
        trace!(
            self.log,
            "Setting up server on port {}",
            self.local_node.address.port
        );

        listener.incoming().for_each(|socket| {
            self.handle_socket_result(
                socket.chain_err(|| "Failed to create socket"),
                self.message_handler.clone(),
                self.data_transformer.clone(),
            )
        });

        Ok(())
    }
}

impl SocketServer for TcpGlobalServer {
    type SocketType = TcpStream;

    fn get_log(&self) -> &Logger {
        &self.log
    }

    fn check_request(&self, request: &RequestMessage) -> Result<()> {
        if !request.payload.is_visible(&ApiVisibility::Global()) {
            Err(ErrorKind::RequestError(
                "Request is not globally available".into(),
            ).into())
        } else {
            Ok(())
        }
    }
}

/// Implementation of sending global requests to TCP servers.
pub struct TcpGlobalClient {
    data_transformer: Arc<DataTransformer>,
    local_node: Node,
    log: Logger,
}

impl TcpGlobalClient {
    /// Create a new sender, which uses a `DataTransformer` to serialize packets
    /// before going on the line.
    pub fn new(
        data_transformer: Arc<DataTransformer>,
        local_node: Node,
        log: Logger,
    ) -> Self {
        TcpGlobalClient {
            data_transformer: data_transformer,
            local_node: local_node,
            log: log,
        }
    }
}

impl SocketClient for TcpGlobalClient {
    type SocketType = TcpStream;

    fn get_log(&self) -> &Logger {
        &self.log
    }

    fn create_socket(&self, node: &Node) -> Result<TcpStream> {
        TcpStream::connect(&node.address.get_socket_addr())
            .chain_err(|| "Error on trying to connect to node")
    }
}

impl Client for TcpGlobalClient {
    fn receive<'a>(
        &self,
        node: &Node,
        request_payload: RequestPayload,
    ) -> Result<ResponseMessage> {
        let request = RequestMessage::new(
            request_payload,
            MessageSender::Node(self.local_node.clone()),
        );
        SocketClient::receive(self, node, request, &*self.data_transformer)
    }
}
