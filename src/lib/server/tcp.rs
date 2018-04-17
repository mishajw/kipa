//! Implementation of servers using TCP sockets.

use api::{MessageSender, RequestMessage, RequestPayload, ResponseMessage,
          ResponsePayload};
use data_transformer::DataTransformer;
use error::*;
use server::{Client, Server};
use node::Node;
use request_handler::RequestHandler;
use socket_server::{SocketClient, SocketServer};

use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;

/// Server that listens for global requests on a specified TCP socket.
pub struct TcpGlobalServer {
    request_handler: Arc<RequestHandler>,
    data_transformer: Arc<DataTransformer>,
    local_node: Node,
}

impl TcpGlobalServer {
    /// Create a new TCP server.
    /// - `request_hanlder` is what to send requests to.
    /// - `data_transformer` used to decode requests.
    /// - `port` the port used to listen on.
    pub fn new(
        request_handler: Arc<RequestHandler>,
        data_transformer: Arc<DataTransformer>,
        local_node: Node,
    ) -> Self {
        TcpGlobalServer {
            request_handler: request_handler,
            data_transformer: data_transformer,
            local_node: local_node,
        }
    }
}

impl Server for TcpGlobalServer {
    fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            self.local_node.address.port,
        )).chain_err(|| "Error on bind to TCP socket")?;
        trace!("Setting up server on port {}", self.local_node.address.port);

        listener.incoming().for_each(|socket| {
            self.handle_socket_result(
                socket.chain_err(|| "Failed to create socket"),
                self.request_handler.clone(),
                self.data_transformer.clone(),
            )
        });

        Ok(())
    }
}

impl SocketServer for TcpGlobalServer {
    type SocketType = TcpStream;

    fn payload_to_response(
        &self,
        response_payload: ResponsePayload,
    ) -> ResponseMessage {
        ResponseMessage::new(
            response_payload,
            MessageSender::Node(self.local_node.clone()),
        )
    }
}

/// Implementation of sending global requests to TCP servers.
pub struct TcpGlobalClient {
    data_transformer: Arc<DataTransformer>,
    local_node: Node,
}

impl TcpGlobalClient {
    /// Create a new sender, which uses a `DataTransformer` to serialize packets
    /// before going on the line.
    pub fn new(
        data_transformer: Arc<DataTransformer>,
        local_node: Node,
    ) -> Self {
        TcpGlobalClient {
            data_transformer: data_transformer,
            local_node: local_node,
        }
    }
}

impl SocketClient for TcpGlobalClient {
    type SocketType = TcpStream;

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