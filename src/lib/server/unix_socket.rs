//! Handles sending and receiving requests on a unix pipe for local processes,
//! such as the CLI

use address::Address;
use api::{
    ApiVisibility, MessageSender, RequestMessage, RequestPayload,
    ResponseMessage,
};
use data_transformer::DataTransformer;
use error::*;
use message_handler::MessageHandler;
use server::{LocalClient, LocalServer};
use socket_server::{SocketHandler, SocketServer};
use versioning;

use std::fs;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use slog::Logger;

/// The default unix socket path
pub const DEFAULT_UNIX_SOCKET_PATH: &str = "/tmp/kipa";

/// Listens for local requests on a unix socket file
#[derive(Clone)]
pub struct UnixSocketLocalServer {
    message_handler: Arc<MessageHandler>,
    data_transformer: Arc<DataTransformer>,
    socket_path: String,
    log: Logger,
}

impl UnixSocketLocalServer {
    /// Create a new unix socket local receive server that listens on some file
    /// `socket_path`
    pub fn new(
        message_handler: Arc<MessageHandler>,
        data_transformer: Arc<DataTransformer>,
        socket_path: String,
        log: Logger,
    ) -> Result<Self>
    {
        Ok(UnixSocketLocalServer {
            message_handler,
            data_transformer,
            socket_path,
            log,
        })
    }
}

impl LocalServer for UnixSocketLocalServer {
    fn start(&self) -> Result<thread::JoinHandle<()>> {
        // Remove the old unix socket file if it exists
        if fs::metadata(&self.socket_path).is_ok() {
            fs::remove_file(&self.socket_path)
                .chain_err(|| "Error on removing old KIPA socket file")?;
        }

        let listener = UnixListener::bind(&self.socket_path).chain_err(|| {
            format!("Error on binding to socket path: {}", self.socket_path)
        })?;
        debug!(
            self.log,
            "Started listening on unix socket";
            "path" => &self.socket_path
        );

        // See `kipa_lib::server` for relevent TODO
        let arc_self = Arc::new(self.clone());
        let join_handle = thread::spawn(move || {
            listener.incoming().for_each(move |socket| {
                let spawn_self = arc_self.clone();
                thread::spawn(move || {
                    spawn_self.handle_socket_result(
                        socket.chain_err(|| "Failed to create socket"),
                        spawn_self.message_handler.clone(),
                        spawn_self.data_transformer.clone(),
                    )
                });
            });
        });

        Ok(join_handle)
    }
}

impl SocketHandler for UnixSocketLocalServer {
    type SocketType = UnixStream;

    fn set_socket_timeout(
        &self,
        _socket: &mut UnixStream,
        _timeout: Option<Duration>,
    ) -> Result<()>
    {
        // Ignore timeouts on unix sockets, as there should be little to no
        // delay
        Ok(())
    }

    fn get_socket_peer_address(
        &self,
        _socket: &Self::SocketType,
    ) -> Option<Address>
    {
        None
    }
}

impl SocketServer for UnixSocketLocalServer {
    fn get_log(&self) -> &Logger { &self.log }

    fn check_request(&self, request: &RequestMessage) -> Result<()> {
        if !request.payload.is_visible(&ApiVisibility::Local()) {
            Err(ErrorKind::RequestError(
                "Request is not locally available".into(),
            ).into())
        } else {
            Ok(())
        }
    }
}

/// Send requests to a local KIPA daemon through a unix socket file
pub struct UnixSocketLocalClient {
    data_transformer: Arc<DataTransformer>,
    socket_path: String,
    log: Logger,
}

impl UnixSocketLocalClient {
    /// Create a new sender, which uses a `DataTransformer` to serialize packets
    /// before going on the line
    pub fn new(
        data_transformer: Arc<DataTransformer>,
        socket_path: &str,
        log: Logger,
    ) -> Self
    {
        UnixSocketLocalClient {
            socket_path: socket_path.to_string(),
            data_transformer,
            log,
        }
    }
}

impl SocketHandler for UnixSocketLocalClient {
    type SocketType = UnixStream;

    fn set_socket_timeout(
        &self,
        _socket: &mut UnixStream,
        _timeout: Option<Duration>,
    ) -> Result<()>
    {
        // Ignore timeouts on unix sockets, as there should be little to no
        // delay
        Ok(())
    }

    fn get_socket_peer_address(
        &self,
        _socket: &Self::SocketType,
    ) -> Option<Address>
    {
        None
    }
}

impl LocalClient for UnixSocketLocalClient {
    fn send(
        &self,
        request_payload: RequestPayload,
        message_id: u32,
    ) -> Result<ResponseMessage>
    {
        let request = RequestMessage::new(
            request_payload,
            MessageSender::Cli(),
            message_id,
            versioning::get_version(),
        );
        let request_bytes = self.data_transformer.request_to_bytes(&request)?;

        trace!(self.log, "Setting up socket to daemon");
        let mut socket = UnixStream::connect(&self.socket_path)
            .chain_err(|| "Error on trying to connect to node")?;

        trace!(self.log, "Sending request to daemon");
        self.send_data(&request_bytes, &mut socket, None)?;

        trace!(self.log, "Reading response from daemon");
        let response_data = self.receive_data(&mut socket, None)?;

        trace!(self.log, "Got response bytes");
        self.data_transformer
            .bytes_to_response(&response_data, None)
    }
}
