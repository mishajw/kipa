//! Contain server-based code for communicating between two nodes.
//!
//! Servers in this file use generic socket types to read and write data from
//! sockets, and use `DataHandler` types to convert these into `Request`s and
//! `Response`s.

use api::{RequestMessage, ResponseMessage};
use data_transformer::DataTransformer;
use error::*;
use message_handler::MessageHandler;
use node::Node;

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use error_chain::ChainedError;
use std::io::Cursor;
use std::io::{Read, Write};
use std::mem::size_of;
use std::sync::Arc;
use std::time::{Duration, Instant};

use slog::Logger;

/// The default port for server communication.
pub const DEFAULT_PORT: u16 = 10842;

/// Type for structs that interface with sockets
pub trait SocketHandler {
    /// The type of the socket to use for sending/receiveing data
    type SocketType: Read + Write + Send + Sync + 'static;

    /// Set the timeout of a `SocketType`
    fn set_socket_timeout(
        &self,
        socket: &mut Self::SocketType,
        timeout: Option<Duration>,
    ) -> Result<()>;

    /// Send data down a socket. Handles writing the length of the data.
    fn send_data(
        &self,
        data: &Vec<u8>,
        socket: &mut Self::SocketType,
        deadline: Option<Instant>,
    ) -> Result<()> {
        let mut len_data = vec![];
        len_data
            .write_u32::<NetworkEndian>(data.len() as u32)
            .chain_err(|| "Error on encoding length as byte array")?;

        self.set_socket_timeout(socket, deadline.map(|d| d - Instant::now()))?;
        socket
            .write(&len_data)
            .chain_err(|| "Error on writing length")?;

        self.set_socket_timeout(socket, deadline.map(|d| d - Instant::now()))?;
        socket
            .write(&data)
            .chain_err(|| "Error on writing response data")?;

        Ok(())
    }

    /// Receive data from a socket. Handles reading the length of the data.
    fn receive_data(
        &self,
        socket: &mut Self::SocketType,
        deadline: Option<Instant>,
    ) -> Result<Vec<u8>> {
        const SIZE_OF_LEN: usize = size_of::<u32>();
        let mut len_data: [u8; SIZE_OF_LEN] = [0; SIZE_OF_LEN];

        self.set_socket_timeout(socket, deadline.map(|d| d - Instant::now()))?;
        socket
            .read_exact(&mut len_data)
            .chain_err(|| "Error on reading length data")?;

        let mut cursor = Cursor::new(len_data);
        let len = cursor
            .read_u32::<NetworkEndian>()
            .chain_err(|| "Error on casting length data to u32")?;
        let mut data = vec![0 as u8; len as usize];

        self.set_socket_timeout(socket, deadline.map(|d| d - Instant::now()))?;
        socket
            .read_exact(&mut data)
            .chain_err(|| "Error on read main data")?;

        Ok(data)
    }
}

/// Create a server that can listen for requests and pass onto a
/// `PayloadHandler`.
pub trait SocketServer: SocketHandler + Send + Sync {
    /// Get the logger for this instance
    fn get_log(&self) -> &Logger;

    /// Handle a socket that the server has receieved wrapped in a result.
    fn handle_socket_result(
        &self,
        socket_result: Result<Self::SocketType>,
        message_handler: Arc<MessageHandler>,
        data_transformer: Arc<DataTransformer>,
    ) {
        let result = socket_result
            .map(|s| self.handle_socket(s, message_handler, data_transformer));

        if let &Err(ref err) = &result {
            error!(
                self.get_log(),
                "Exception when handling socket";
                "exception" => %err.display_chain());
        }
    }

    /// Handle a socket that the server has received.
    fn handle_socket(
        &self,
        socket: Self::SocketType,
        message_handler: Arc<MessageHandler>,
        data_transformer: Arc<DataTransformer>,
    ) -> Result<()> {
        let log = self.get_log();

        let mut inner_socket = socket;
        trace!(log, "Reading request from socket");
        let request_data = self.receive_data(&mut inner_socket, None)?;

        trace!(log, "Processing request");
        let request =
            data_transformer.bytes_to_request(&request_data.to_vec())?;

        trace!(log, "Sending response");
        let response = message_handler.receive(request)?;
        let response_data = data_transformer.response_to_bytes(&response)?;
        self.send_data(&response_data, &mut inner_socket, None)?;
        trace!(log, "Sent response bytes");

        Ok(())
    }

    /// Check that the request is OK to process.
    fn check_request(&self, request: &RequestMessage) -> Result<()>;
}

/// Functionality for sending requests to other KIPA servers on a socket.
pub trait SocketClient: SocketHandler {
    /// Get the logger for this instance
    fn get_log(&self) -> &Logger;

    /// Create a socket to connect to the `node`.
    fn create_socket(
        &self,
        node: &Node,
        timeout: Duration,
    ) -> Result<Self::SocketType>;

    /// Send a request to another `Node` and get the `Response`.
    fn send<'a>(
        &self,
        node: &Node,
        request: RequestMessage,
        data_transformer: &DataTransformer,
        timeout: Duration,
    ) -> Result<ResponseMessage> {
        let deadline = Instant::now() + timeout;

        let request_bytes = data_transformer.request_to_bytes(&request)?;

        trace!(
            self.get_log(),
            "Setting up socket";
            "node" => %node
        );
        let mut socket = self.create_socket(node, deadline - Instant::now())?;

        trace!(
            self.get_log(),
            "Sending request to another node"
        );
        self.send_data(&request_bytes, &mut socket, Some(deadline))?;

        trace!(
            self.get_log(),
            "Reading response from another node"
        );
        let response_data = self.receive_data(&mut socket, Some(deadline))?;

        trace!(self.get_log(), "Got response bytes");
        data_transformer.bytes_to_response(&response_data)
    }
}
