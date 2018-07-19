//! Handles sending and receiving requests on a unix pipe for local processes,
//! such as the CLI

use address::Address;
use error::*;
use message_handler::MessageHandlerServer;
use server::socket_server::{SocketHandler, SocketServer};
use server::{LocalClient, LocalServer};

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
    message_handler_server: Arc<MessageHandlerServer>,
    socket_path: String,
    log: Logger,
}

impl UnixSocketLocalServer {
    /// Create a new unix socket local receive server that listens on some file
    /// `socket_path`
    pub fn new(
        message_handler_server: Arc<MessageHandlerServer>,
        socket_path: String,
        log: Logger,
    ) -> Result<Self>
    {
        Ok(UnixSocketLocalServer {
            message_handler_server,
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
                        spawn_self.message_handler_server.clone(),
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
}

/// Send requests to a local KIPA daemon through a unix socket file
pub struct UnixSocketLocalClient {
    socket_path: String,
    log: Logger,
}

impl UnixSocketLocalClient {
    /// Create a new sender, which uses a `DataTransformer` to serialize packets
    /// before going on the line
    pub fn new(socket_path: String, log: Logger) -> Self {
        UnixSocketLocalClient { socket_path, log }
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
    fn send(&self, request_data: &[u8]) -> Result<Vec<u8>> {
        trace!(self.log, "Setting up socket to daemon");
        let mut socket = UnixStream::connect(&self.socket_path)
            .chain_err(|| "Error on trying to connect to node")?;

        trace!(self.log, "Sending request to daemon");
        self.send_data(&request_data, &mut socket, None)?;

        trace!(self.log, "Reading response from daemon");
        self.receive_data(&mut socket, None)
    }
}
