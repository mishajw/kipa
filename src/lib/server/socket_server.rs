//! Contain socket-based operations for communicating between two nodes
//!
//! Servers in this file use generic socket types to read and write data from
//! sockets, and use `DataTransformer` types to convert these into `Request`s
//! and `Response`s.

use address::Address;
use error::*;
use message_handler::IncomingMessageHandler;
use node::Node;

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use error_chain::ChainedError;
use std::io::Cursor;
use std::io::{Read, Write};
use std::mem::size_of;
use std::sync::Arc;
use std::time::{Duration, Instant};

use slog::Logger;

/// Type for structs that interface with sockets
pub trait SocketHandler {
    /// The type of the socket to use for sending/receiving data
    type SocketType: Read + Write + Send + Sync + 'static;

    /// Set the timeout of a `SocketType`
    fn set_socket_timeout(
        &self,
        socket: &mut Self::SocketType,
        timeout: Option<Duration>,
    ) -> Result<()>;

    /// Get the address of the peer connected to the other side of the socket
    fn get_socket_peer_address(
        &self,
        socket: &Self::SocketType,
    ) -> Option<Address>;

    /// Send data down a socket. Handles writing the length of the data
    fn send_data(
        &self,
        data: &[u8],
        socket: &mut Self::SocketType,
        deadline: Option<Instant>,
    ) -> Result<()>
    {
        let mut len_data = vec![];
        len_data
            .write_u32::<NetworkEndian>(data.len() as u32)
            .chain_err(|| "Error on encoding length as byte array")?;

        self.set_socket_timeout(socket, deadline.map(deadline_to_duration))?;
        socket
            .write(&len_data)
            .chain_err(|| "Error on writing length")?;

        self.set_socket_timeout(socket, deadline.map(deadline_to_duration))?;
        socket
            .write(&data)
            .chain_err(|| "Error on writing response data")?;

        Ok(())
    }

    /// Receive data from a socket. Handles reading the length of the data
    fn receive_data(
        &self,
        socket: &mut Self::SocketType,
        deadline: Option<Instant>,
    ) -> Result<Vec<u8>>
    {
        const SIZE_OF_LEN: usize = size_of::<u32>();
        let mut len_data: [u8; SIZE_OF_LEN] = [0; SIZE_OF_LEN];

        self.set_socket_timeout(socket, deadline.map(deadline_to_duration))?;
        socket
            .read_exact(&mut len_data)
            .chain_err(|| "Error on reading length data")?;

        let mut cursor = Cursor::new(len_data);
        let len = cursor
            .read_u32::<NetworkEndian>()
            .chain_err(|| "Error on casting length data to u32")?;
        let mut data = vec![0 as u8; len as usize];

        self.set_socket_timeout(socket, deadline.map(deadline_to_duration))?;
        socket
            .read_exact(&mut data)
            .chain_err(|| "Error on read main data")?;

        Ok(data)
    }
}

/// Create a server that can listen for requests and pass onto a
/// `PayloadHandler`
pub trait SocketServer: SocketHandler + Send + Sync {
    /// Get the logger for this instance
    fn get_log(&self) -> &Logger;

    /// Handle a socket that the server has receieved wrapped in a result
    fn handle_socket_result(
        &self,
        socket_result: Result<Self::SocketType>,
        message_handler: Arc<IncomingMessageHandler>,
    )
    {
        let result =
            socket_result.and_then(|s| self.handle_socket(s, message_handler));

        if let Err(ref err) = result {
            error!(
                self.get_log(),
                "Exception when handling socket";
                "exception" => %err.display_chain());
        }
    }

    /// Handle a socket that the server has received
    fn handle_socket(
        &self,
        socket: Self::SocketType,
        message_handler: Arc<IncomingMessageHandler>,
    ) -> Result<()>
    {
        let log = self.get_log();
        let address = self.get_socket_peer_address(&socket);

        let mut inner_socket = socket;
        trace!(log, "Reading request from socket");
        let request_data = self.receive_data(&mut inner_socket, None)?;

        trace!(log, "Processing request");
        let response_data =
            message_handler.receive_bytes(&request_data, address)?;

        trace!(log, "Sending response");
        self.send_data(&response_data, &mut inner_socket, None)?;
        trace!(log, "Sent response bytes");

        Ok(())
    }
}

/// Functionality for sending requests to other KIPA servers on a socket
pub trait SocketClient: SocketHandler {
    /// Get the logger for this instance
    fn get_log(&self) -> &Logger;

    /// Create a socket to connect to the `node`
    fn create_socket(
        &self,
        node: &Node,
        timeout: Duration,
    ) -> Result<Self::SocketType>;

    /// Send a request to another `Node` and get the `Response`
    fn send(
        &self,
        node: &Node,
        request_data: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>>
    {
        let deadline = Instant::now() + timeout;
        trace!(
            self.get_log(),
            "Setting up socket";
            "node" => %node
        );

        let mut socket =
            self.create_socket(node, deadline_to_duration(deadline))?;

        trace!(self.get_log(), "Sending request to another node");
        self.send_data(&request_data, &mut socket, Some(deadline))?;

        trace!(self.get_log(), "Reading response from another node");
        self.receive_data(&mut socket, Some(deadline))
    }
}

fn deadline_to_duration(deadline: Instant) -> Duration {
    let now = Instant::now();

    if now < deadline {
        deadline - now
    } else {
        Duration::from_secs(0)
    }
}
