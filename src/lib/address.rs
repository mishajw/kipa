use error::*;

use std::fmt;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

/// An address of a node.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Address {
    /// The IP address data.
    ///
    /// Can either be 4 bytes long for IPv4, or 16 bytes long for IPv6.
    pub ip_data: Vec<u8>,
    /// The 16-bit port number of the address.
    pub port: u16
}

impl Address {
    /// Create a new address with some address and port.
    pub fn new(ip_data: Vec<u8>, port: u16) -> Self {
        Address { ip_data: ip_data, port: port }
    }

    /// Create a new address from a string.
    pub fn from_string(s: &str) -> Result<Address> {
        let socket_addr: SocketAddr =
            s.parse().chain_err(|| "Error on parsing IP address")?;

        match socket_addr {
            SocketAddr::V4(addr) =>
                Ok(Address{
                    ip_data: addr.ip().octets().to_vec(), port: addr.port()
                }),
            SocketAddr::V6(_) => unimplemented!()
        }
    }

    /// Get the `SocketAddr` for the address.
    pub fn get_socket_addr(&self) -> SocketAddr {
        if self.ip_data.len() == 4 {
            SocketAddr::new(
                IpAddr::V4(
                    Ipv4Addr::new(
                        self.ip_data[0],
                        self.ip_data[1],
                        self.ip_data[2],
                        self.ip_data[3])),
                    self.port)
        } else {
            unimplemented!();
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.ip_data.len() == 4 {
            write!(
                f, "{}.{}.{}.{}:{}",
                self.ip_data[0],
                self.ip_data[1],
                self.ip_data[2],
                self.ip_data[3],
                self.port)
        } else {
            unimplemented!();
        }
    }
}

