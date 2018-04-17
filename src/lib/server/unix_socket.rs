//! Handles sending and receiving requests on a unix pipe for local processes,
//! such as the CLI.

use error::*;
use request_handler::RequestHandler;
use api::{MessageSender, RequestMessage, RequestPayload, ResponseMessage,
          ResponsePayload};
use data_transformer::DataTransformer;
use server::{LocalClient, Server};
use socket_server::{receive_data, send_data, SocketServer};

use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;

/// The default unix socket path.
pub const DEFAULT_UNIX_SOCKET_PATH: &str = "/tmp/kipa";

/// Listens for local requests on a unix socket file.
pub struct UnixSocketLocalServer {
    request_handler: Arc<RequestHandler>,
    data_transformer: Arc<DataTransformer>,
    socket_path: String,
}

impl UnixSocketLocalServer {
    /// Create a new unix socket local receive server that listens on some file
    /// `socket_path`.
    pub fn new(
        request_handler: Arc<RequestHandler>,
        data_transformer: Arc<DataTransformer>,
        socket_path: String,
    ) -> Result<Self> {
        Ok(UnixSocketLocalServer {
            request_handler: request_handler,
            data_transformer: data_transformer,
            socket_path: socket_path,
        })
    }
}

impl Server for UnixSocketLocalServer {
    fn start(&self) -> Result<()> {
        let listener = UnixListener::bind(&self.socket_path).chain_err(|| {
            format!("Error on binding to socket path: {}", self.socket_path)
        })?;
        trace!(
            "Started listening on unix socket at path {}",
            self.socket_path
        );

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

impl SocketServer for UnixSocketLocalServer {
    type SocketType = UnixStream;

    fn payload_to_response(
        &self,
        response_payload: ResponsePayload,
    ) -> ResponseMessage {
        ResponseMessage::new(response_payload, MessageSender::Cli())
    }
}

/// Send requests to a local KIPA daemon through a unix socket file.
pub struct UnixSocketLocalClient {
    data_transformer: Arc<DataTransformer>,
    socket_path: String,
}

impl UnixSocketLocalClient {
    /// Create a new sender, which uses a `DataTransformer` to serialize packets
    /// before going on the line.
    pub fn new(
        data_transformer: Arc<DataTransformer>,
        socket_path: &String,
    ) -> Self {
        UnixSocketLocalClient {
            socket_path: socket_path.clone(),
            data_transformer: data_transformer,
        }
    }
}

impl LocalClient for UnixSocketLocalClient {
    fn receive<'a>(
        &self,
        request_payload: RequestPayload,
    ) -> Result<ResponseMessage> {
        let request =
            RequestMessage::new(request_payload, MessageSender::Cli());
        let request_bytes = self.data_transformer.request_to_bytes(&request)?;

        trace!("Setting up socket to daemon");
        let mut socket = UnixStream::connect(&self.socket_path)
            .chain_err(|| "Error on trying to connect to node")?;

        trace!("Sending request to daemon");
        send_data(&request_bytes, &mut socket)?;

        trace!("Reading response from daemon");
        let response_data = receive_data(&mut socket)?;

        trace!("Got response bytes");
        self.data_transformer.bytes_to_response(&response_data)
    }
}