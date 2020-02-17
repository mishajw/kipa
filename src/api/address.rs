//! IPv4 and IPv6 addresses used for communication.

use error::*;

use serde;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// An address of a node.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Address {
    /// The IP address data.
    ///
    /// Can either be 4 bytes long for IPv4, or 16 bytes long for IPv6.
    pub ip_data: Vec<u8>,
    /// The 16-bit port number of the address.
    pub port: u16,
}

impl Address {
    /// Create a new address with some address and port.
    pub fn new(ip_data: Vec<u8>, port: u16) -> Self {
        assert!(ip_data.len() == 4 || ip_data.len() == 16);
        Address { ip_data, port }
    }

    /// Create a new address from a string.
    pub fn from_string(s: &str) -> InternalResult<Address> {
        let socket_addr: SocketAddr = s.parse().map_err(|err| {
            InternalError::public_with_error(
                "Error on parsing IP address. Expected '<IPV4/V6 address>:<port>'",
                ApiErrorType::Parse,
                err,
            )
        })?;
        Ok(Self::from_socket_addr(&socket_addr))
    }

    /// Create an `Address` from a `SocketAddr`.
    pub fn from_socket_addr(socket_addr: &SocketAddr) -> Self {
        match socket_addr {
            SocketAddr::V4(addr) => Address::new(addr.ip().octets().to_vec(), addr.port()),
            SocketAddr::V6(addr) => Address::new(addr.ip().octets().to_vec(), addr.port()),
        }
    }

    /// Get the `SocketAddr` for the address.
    pub fn to_socket_addr(&self) -> SocketAddr {
        if self.ip_data.len() == 4 {
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(
                    self.ip_data[0],
                    self.ip_data[1],
                    self.ip_data[2],
                    self.ip_data[3],
                )),
                self.port,
            )
        } else if self.ip_data.len() == 16 {
            fn to_u16(x: u8, y: u8) -> u16 {
                use byteorder::{NetworkEndian, ReadBytesExt};
                use std::io::Cursor;
                let mut cursor = Cursor::new(vec![x, y]);
                cursor
                    .read_u16::<NetworkEndian>()
                    .expect("Error on casting two u8's to a u16")
            }

            SocketAddr::new(
                IpAddr::V6(Ipv6Addr::new(
                    to_u16(self.ip_data[0], self.ip_data[1]),
                    to_u16(self.ip_data[2], self.ip_data[3]),
                    to_u16(self.ip_data[4], self.ip_data[5]),
                    to_u16(self.ip_data[6], self.ip_data[7]),
                    to_u16(self.ip_data[8], self.ip_data[9]),
                    to_u16(self.ip_data[10], self.ip_data[11]),
                    to_u16(self.ip_data[12], self.ip_data[13]),
                    to_u16(self.ip_data[14], self.ip_data[15]),
                )),
                self.port,
            )
        } else {
            panic!("Unexpected number of IP address bytes.")
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_socket_addr())
    }
}

impl serde::Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_socket_addr().to_string())
    }
}
