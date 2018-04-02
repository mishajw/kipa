//! Implementation of servers using TCP sockets.

use data_transformer::DataTransformer;
use error::*;
use node::Node;
use request_handler::{RequestHandler, Request, Response};
use server::{PublicServer, RemoteServer};

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use error_chain::ChainedError;
use std::io::Cursor;
use std::io::{Read, Write};
use std::mem::{size_of, swap};
use std::net::{SocketAddr, Ipv4Addr, IpAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

/// Server that listens on a specified TCP socket.
pub struct TcpPublicServer {
    thread: Option<thread::JoinHandle<()>>
}

impl TcpPublicServer {
    /// Create a new TCP server.
    /// - `request_hanlder` is what to send requests to.
    /// - `data_transformer` used to decode requests.
    /// - `port` the port used to listen on.
    pub fn new(
            request_handler: Arc<RequestHandler>,
            data_transformer: Arc<DataTransformer>,
            port: u16) -> Result<Self> {
        let local_address = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
        let listener = TcpListener::bind(&local_address)
            .chain_err(|| "Error on bind to TCP socket")?;

        trace!("Setting up server on port {}", port);
        let t = thread::spawn(move || {
            listener.incoming()
                .for_each(move |socket_result| {
                    trace!("Received new connection");
                    let result = socket_result
                        .chain_err(|| "Error on creating socket")
                        .and_then(
                            |mut socket|
                            TcpPublicServer::handle_socket(
                                &*request_handler,
                                &*data_transformer,
                                &mut socket));

                    if let Err(err) = result {
                        println!("{}", err.display_chain().to_string());
                        error!(
                            "Exception when handling socket: {}",
                            err.display_chain());
                    }
                });
        });
        Ok(TcpPublicServer {
            thread: Some(t)
        })
    }

    fn handle_socket(
            request_handler: &RequestHandler,
            data_transformer: &DataTransformer,
            socket: &mut TcpStream) -> Result<()> {
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

impl PublicServer for TcpPublicServer {
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

/// Implementation of sending requests to TCP servers.
pub struct TcpRemoteServer {
    data_transformer: Arc<DataTransformer>
}

impl TcpRemoteServer {
    /// Create a new sender, which uses a `DataTransformer` to serialize packets
    /// before going on the line.
    pub fn new(data_transformer: Arc<DataTransformer>) -> Self {
        TcpRemoteServer {
            data_transformer: data_transformer
        }
    }
}

impl RemoteServer for TcpRemoteServer {
    fn receive<'a>(&self, node: &Node, request: &Request) -> Result<Response> {
        let request_bytes =
            self.data_transformer.request_to_bytes(request)?;

        trace!("Setting up socket to another node");
        let mut socket = TcpStream::connect(&node.address.get_socket_addr())
            .chain_err(|| "Error on waiting for socket")?;

        trace!("Sending request to another node");
        send_data(&request_bytes, &mut socket)?;

        trace!("Reading response from another node");
        let response_data = receive_data(&mut socket)?;

        trace!("Got response bytes");
        self.data_transformer.bytes_to_response(&response_data)
    }
}

fn send_data(data: &Vec<u8>, socket: &mut TcpStream) -> Result<()> {
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

fn receive_data(socket: &mut TcpStream) -> Result<Vec<u8>> {
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

