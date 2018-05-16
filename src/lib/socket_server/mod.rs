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
use std::thread;

use slog::Logger;

/// The default port for server communication.
pub const DEFAULT_PORT: u16 = 10842;

/// Create a server that can listen for requests and pass onto a
/// `PayloadHandler`.
pub trait SocketServer: Send + Sync {
    /// The type of the socket to use for sending/receiveing data
    type SocketType: Read + Write + Send + Sync + 'static;

    /// Get the logger for this instance
    fn get_log(&self) -> &Logger;

    /// Handle a socket that the server has receieved wrapped in a result.
    fn handle_socket_result(
        &self,
        socket_result: Result<Self::SocketType>,
        message_handler: Arc<MessageHandler>,
        data_transformer: Arc<DataTransformer>,
    ) {
        if let &Err(ref err) = &socket_result {
            error!(
                self.get_log(),
                "Exception when receiving socket";
                "exception" => %err.display_chain());
        }

        self.handle_socket(
            socket_result.unwrap(),
            message_handler,
            data_transformer,
        )
    }

    /// Handle a socket that the server has received.
    fn handle_socket(
        &self,
        socket: Self::SocketType,
        message_handler: Arc<MessageHandler>,
        data_transformer: Arc<DataTransformer>,
    ) {
        let log = self.get_log().new(o!());
        let callback = move || -> Result<()> {
            let mut inner_socket = socket;
            trace!(log, "Reading request from socket");
            let request_data = receive_data(&mut inner_socket)?;

            trace!(log, "Processing request");
            let request =
                data_transformer.bytes_to_request(&request_data.to_vec())?;

            trace!(log, "Sending response");
            let response = message_handler.receive(request)?;
            let response_data = data_transformer.response_to_bytes(&response)?;
            send_data(&response_data, &mut inner_socket)?;
            trace!(log, "Sent response bytes");
            Ok(())
        };

        let thread_log = self.get_log().new(o!());
        // TODO: Can we move the `thread::spawn` to earlier in the socket
        // creation in order to speed up ability to process multiple requests
        // quickly?
        thread::spawn(move || {
            if let Err(err) = callback() {
                error!(
                    thread_log,
                    "Exception when handling socket";
                     "exception" => %err.display_chain());
            }
        });
    }

    /// Check that the request is OK to process.
    fn check_request(&self, request: &RequestMessage) -> Result<()>;
}

/// Functionality for sending requests to other KIPA servers on a socket.
pub trait SocketClient {
    /// The socket type to send/receive data from.
    type SocketType: Read + Write;

    /// Get the logger for this instance
    fn get_log(&self) -> &Logger;

    /// Create a socket to connect to the `node`.
    fn create_socket(&self, node: &Node) -> Result<Self::SocketType>;

    /// Send a request to another `Node` and get the `Response`.
    fn receive<'a>(
        &self,
        node: &Node,
        request: RequestMessage,
        data_transformer: &DataTransformer,
    ) -> Result<ResponseMessage> {
        let request_bytes = data_transformer.request_to_bytes(&request)?;

        trace!(
            self.get_log(),
            "Setting up socket";
            "node" => %node
        );
        let mut socket = self.create_socket(node)?;

        trace!(self.get_log(), "Sending request to another node");
        send_data(&request_bytes, &mut socket)?;

        trace!(self.get_log(), "Reading response from another node");
        let response_data = receive_data(&mut socket)?;

        trace!(self.get_log(), "Got response bytes");
        data_transformer.bytes_to_response(&response_data)
    }
}

/// Send data down a socket. Handles writing the length of the data.
pub fn send_data<SocketType: Write>(
    data: &Vec<u8>,
    socket: &mut SocketType,
) -> Result<()> {
    let mut len_data = vec![];
    len_data
        .write_u32::<NetworkEndian>(data.len() as u32)
        .chain_err(|| "Error on encoding length as byte array")?;
    socket
        .write(&len_data)
        .chain_err(|| "Error on writing length")?;
    socket
        .write(&data)
        .chain_err(|| "Error on writing response data")?;
    Ok(())
}

/// Receive data from a socket. Handles reading the length of the data.
pub fn receive_data<SocketType: Read>(
    socket: &mut SocketType,
) -> Result<Vec<u8>> {
    const SIZE_OF_LEN: usize = size_of::<u32>();
    let mut len_data: [u8; SIZE_OF_LEN] = [0; SIZE_OF_LEN];
    socket
        .read_exact(&mut len_data)
        .chain_err(|| "Error on reading length data")?;
    let mut cursor = Cursor::new(len_data);
    let len = cursor
        .read_u32::<NetworkEndian>()
        .chain_err(|| "Error on casting length data to u32")?;
    let mut data = vec![0 as u8; len as usize];
    socket
        .read_exact(&mut data)
        .chain_err(|| "Error on read main data")?;

    Ok(data)
}
