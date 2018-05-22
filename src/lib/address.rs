use error::*;

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use pnet::datalink;
use slog::Logger;

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
        Address {
            ip_data: ip_data,
            port: port,
        }
    }

    /// Create a new address from a string.
    pub fn from_string(s: &str) -> Result<Address> {
        let socket_addr: SocketAddr =
            s.parse().chain_err(|| "Error on parsing IP address")?;

        match socket_addr {
            SocketAddr::V4(addr) => Ok(Address {
                ip_data: addr.ip().octets().to_vec(),
                port: addr.port(),
            }),
            SocketAddr::V6(_) => unimplemented!(),
        }
    }

    /// Get the local address on a specified interface
    pub fn get_local(
        port: u16,
        interface_name: Option<&str>,
        log: Logger,
    ) -> Result<Address>
    {
        for interface in datalink::interfaces() {
            // Skip interfaces that are loopback or have no IPs
            if interface_name.is_none() && (interface.name == "lo")
                || interface.ips.len() == 0
            {
                continue;
            }

            // Skip if we've specified a name, and this interface doesn't match
            if interface_name.is_some()
                && interface.name != interface_name.unwrap()
            {
                continue;
            }

            if interface.ips.len() == 0 {
                return Err(ErrorKind::IpAddressError(format!(
                    "Could not find any IP address on interface {}, found: \
                     {:?}",
                    interface.name, interface.ips
                )).into());
            }

            if interface.ips.len() > 1 {
                warn!(
                    log, "Found multiple IPs on interface, selecting first";
                    "interface_name" => interface.name,
                    "selected_ip" => %interface.ips[0])
            }

            match interface.ips[0].ip() {
                IpAddr::V4(addr) => {
                    return Ok(Address {
                        ip_data: addr.octets().to_vec(),
                        port: port,
                    })
                }
                _ => unimplemented!(),
            }
        }

        Err(ErrorKind::IpAddressError(
            "Could not find matching interface name".into(),
        ).into())
    }

    /// Get the `SocketAddr` for the address.
    pub fn get_socket_addr(&self) -> SocketAddr {
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
        } else {
            unimplemented!();
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.ip_data.len() == 4 {
            write!(
                f,
                "{}.{}.{}.{}:{}",
                self.ip_data[0],
                self.ip_data[1],
                self.ip_data[2],
                self.ip_data[3],
                self.port
            )
        } else {
            unimplemented!();
        }
    }
}
