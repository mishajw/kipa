//! Contain server-based code for communicating between two nodes.
//!
//! Servers in this file use generic socket types to read and write data from 
//! sockets, and use `DataHandler` types to convert these into `Request`s and
//! `Response`s.

use error::*;
use node::Node;
use api::{Request, Response};
use request_handler::RequestHandler;
use data_transformer::DataTransformer;

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use error_chain::ChainedError;
use std::io::Cursor;
use std::io::{Read, Write};
use std::mem::{size_of};
use std::sync::Arc;

/// The default port for server communication.
pub const DEFAULT_PORT: u16 = 10842;

/// Create a server that can listen for requests and pass onto a
/// `RequestHandler`.
pub trait ReceiveServer {
    /// The type of the socket to use for sending/receiveing data
    type SocketType: Read + Write;

    /// Handle a socket that the server has receieved wrapped in a result.
    fn handle_socket_result(
            socket_result: Result<Self::SocketType>,
            request_handler: Arc<RequestHandler>,
            data_transformer: Arc<DataTransformer>) {
        let result = socket_result
            .and_then(
                |mut socket| Self::handle_socket(
                    &mut socket,
                    &*request_handler,
                    &*data_transformer));

        if let Err(err) = result {
            println!("{}", err.display_chain().to_string());
            error!(
                "Exception when handling socket: {}",
                err.display_chain());
        }
    }

    /// Handle a socket that the server has received.
    fn handle_socket(
            socket: &mut Self::SocketType,
            request_handler: &RequestHandler,
            data_transformer: &DataTransformer) -> Result<()> {
        trace!("Reading request from socket");
        let request_data = receive_data(socket)?;

        trace!("Processing request");
        let request = data_transformer.bytes_to_request(
            &request_data.to_vec())?;

        trace!("Sending response");
        let response = request_handler.receive(&request)?;
        let response_data = data_transformer.response_to_bytes(&response)?;
        send_data(&response_data, socket)?;
        trace!("Sent response bytes");
        Ok(())
    }
}

/// Functionality for sending requests to other KIPA servers on a socket.
pub trait SendServer {
    /// The socket type to send/receive data from.
    type SocketType: Read + Write;

    /// Create a socket to connect to the `node`.
    fn create_socket(&self, node: &Node) -> Result<Self::SocketType>;

    /// Send a request to another `Node` and get the `Response`.
    fn receive<'a>(
            &self,
            node: &Node,
            request: &Request,
            data_transformer: &DataTransformer) -> Result<Response> {

        let request_bytes =
            data_transformer.request_to_bytes(request)?;

        trace!("Setting up socket to node {}", node);
        let mut socket = self.create_socket(node)?;

        trace!("Sending request to another node");
        send_data(&request_bytes, &mut socket)?;

        trace!("Reading response from another node");
        let response_data = receive_data(&mut socket)?;

        trace!("Got response bytes");
        data_transformer.bytes_to_response(&response_data)
    }
}

/// Send data down a socket. Handles writing the length of the data.
pub fn send_data<SocketType: Write>(
        data: &Vec<u8>, socket: &mut SocketType) -> Result<()> {
    let mut len_data = vec![];
    len_data.write_u32::<NetworkEndian>(
        data.len() as u32)
        .chain_err(|| "Error on encoding length as byte array")?;
    socket.write(&len_data)
        .chain_err(|| "Error on writing length")?;
    socket.write(&data)
        .chain_err(|| "Error on writing response data")?;
    Ok(())
}

/// Receive data from a socket. Handles reading the length of the data.
pub fn receive_data<SocketType: Read>(
        socket: &mut SocketType) -> Result<Vec<u8>> {
    const SIZE_OF_LEN: usize = size_of::<u32>();
    let mut len_data: [u8; SIZE_OF_LEN] = [0; SIZE_OF_LEN];
    socket.read_exact(&mut len_data)
        .chain_err(|| "Error on reading length data")?;
    let mut cursor = Cursor::new(len_data);
    let len = cursor.read_u32::<NetworkEndian>()
        .chain_err(|| "Error on casting length data to u32")?;
    let mut data = vec![0 as u8; len as usize];
    socket.read_exact(&mut data).chain_err(|| "Error on read main data")?;

    Ok(data)
}

