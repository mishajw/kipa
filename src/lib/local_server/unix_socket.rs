//! Handles sending and receiving requests on a unix pipe for local processes,
//! such as the CLI.

use error::*;
use request_handler::RequestHandler;
use api::{Response, Request};
use data_transformer::DataTransformer;
use local_server::{LocalReceiveServer, LocalSendServer};
use server::{ReceiveServer, send_data, receive_data};

use std::os::unix::net::{UnixListener, UnixStream};
use std::thread;
use std::sync::Arc;
use std::mem::swap;

/// The default unix socket path.
pub const DEFAULT_UNIX_SOCKET_PATH: &str = "/tmp/kipa";

/// Listens for local requests on a unix socket file.
pub struct UnixSocketLocalReceiveServer {
    thread: Option<thread::JoinHandle<()>>
}

impl UnixSocketLocalReceiveServer {
    /// Create a new unix socket local receive server that listens on some file
    /// `socket_path`.
    pub fn new(
            request_handler: Arc<RequestHandler>,
            data_transformer: Arc<DataTransformer>,
            socket_path: &String) -> Result<Self> {

        let listener = UnixListener::bind(socket_path)
            .chain_err(|| format!(
                "Error on binding to socket path: {}", socket_path))?;
        trace!("Started listening on unix socket at path {}", socket_path);

        let t = thread::spawn(move || {
            listener.incoming()
                .for_each(|s| Self::handle_socket_result(
                    s.chain_err(|| "Error on local unix socket connection"),
                    request_handler.clone(),
                    data_transformer.clone()
                ))
        });

        Ok(UnixSocketLocalReceiveServer{ thread: Some(t) })
    }
}

impl LocalReceiveServer for UnixSocketLocalReceiveServer {
    fn join(&mut self) -> Result<()> {
        let mut thread: Option<thread::JoinHandle<()>> = None;
        swap(&mut self.thread, &mut thread);
        match thread.map(|t| t.join()) {
            Some(Ok(())) => Ok(()),
            Some(Err(_)) => Err(ErrorKind::JoinError(
                "Error on joining server thread".into()).into()),
            None => Err(ErrorKind::JoinError(
                "Thread already joined".into()).into())
        }
    }
}

impl ReceiveServer for UnixSocketLocalReceiveServer {
    type SocketType = UnixStream;
}

/// Send requests to a local KIPA daemon through a unix socket file.
pub struct UnixSocketLocalSendServer {
    data_transformer: Arc<DataTransformer>,
    socket_path: String
}

impl UnixSocketLocalSendServer {
    /// Create a new sender, which uses a `DataTransformer` to serialize packets
    /// before going on the line.
    pub fn new(
            data_transformer: Arc<DataTransformer>,
            socket_path: &String) -> Self {
        UnixSocketLocalSendServer {
            socket_path: socket_path.clone(),
            data_transformer: data_transformer
        }
    }
}

impl LocalSendServer for UnixSocketLocalSendServer {
    fn receive<'a>(&self, request: &Request) -> Result<Response> {
        let request_bytes =
            self.data_transformer.request_to_bytes(request)?;

        trace!("Setting up socket to another node");
        let mut socket = UnixStream::connect(&self.socket_path)
            .chain_err(|| "Error on trying to connect to node")?;

        trace!("Sending request to another node");
        send_data(&request_bytes, &mut socket)?;

        trace!("Reading response from another node");
        let response_data = receive_data(&mut socket)?;

        trace!("Got response bytes");
        self.data_transformer.bytes_to_response(&response_data)
    }
}

