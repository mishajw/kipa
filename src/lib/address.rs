use error::*;

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use pnet::datalink;
use slog::Logger;

/// An address of a node
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Address {
    /// The IP address data
    ///
    /// Can either be 4 bytes long for IPv4, or 16 bytes long for IPv6.
    pub ip_data: Vec<u8>,
    /// The 16-bit port number of the address
    pub port: u16,
}

impl Address {
    /// Create a new address with some address and port
    pub fn new(ip_data: Vec<u8>, port: u16) -> Self {
        Address { ip_data, port }
    }

    /// Create a new address from a string
    pub fn from_string(s: &str) -> InternalResult<Address> {
        let socket_addr: SocketAddr = s.parse().map_err(|_| {
            InternalError::public(
                "Error on parsing IP address",
                ApiErrorType::Parse,
            )
        })?;
        to_internal_result(Self::from_socket_addr(&socket_addr))
    }

    /// Get the local address on a specified interface
    pub fn get_local(
        local_params: LocalAddressParams,
        log: Logger,
    ) -> InternalResult<Address>
    {
        let LocalAddressParams {
            port,
            interface_name,
            force_ipv6,
        } = local_params;
        for interface in datalink::interfaces() {
            // Skip interfaces that are loopback or have no IPs
            if interface_name.is_none() && (interface.name == "lo")
                || interface.ips.is_empty()
            {
                continue;
            }

            // Skip if we've specified a name, and this interface doesn't match
            if interface_name.is_some()
                && interface.name != interface_name.clone().unwrap()
            {
                continue;
            }

            // Get the list of IPs to be all if we're not forcing IPv6,
            // otherwise filter for IPv6
            let ips = if !force_ipv6 {
                interface.ips
            } else {
                interface
                    .ips
                    .iter()
                    .filter(|ip| ip.is_ipv6())
                    .map(|ip| *ip)
                    .collect()
            };

            if ips.is_empty() {
                return Err(InternalError::private(ErrorKind::IpAddressError(
                    format!(
                        "Could not find any IP address on interface {}, \
                         found: {:?}",
                        interface.name, ips
                    ),
                )));
            }

            if ips.len() > 1 {
                warn!(
                    log, "Found multiple IPs on interface, selecting first";
                    "interface_name" => interface.name,
                    "selected_ip" => %ips[0],
                    "other_ips" => ips.iter().skip(1).map(|ip| ip.to_string())
                        .collect::<Vec<_>>().join(", "))
            }

            let ip_data = match ips[0].ip() {
                IpAddr::V4(addr) => addr.octets().to_vec(),
                IpAddr::V6(addr) => addr.octets().to_vec(),
            };

            return Ok(Address {
                ip_data,
                port: port,
            });
        }

        Err(InternalError::public(
            "Could not find matching interface name",
            ApiErrorType::External,
        ))
    }

    /// Create an `Address` from a `SocketAddr`
    pub fn from_socket_addr(socket_addr: &SocketAddr) -> Result<Self> {
        let (octets, port) = match socket_addr {
            SocketAddr::V4(addr) => (addr.ip().octets().to_vec(), addr.port()),
            SocketAddr::V6(addr) => (addr.ip().octets().to_vec(), addr.port()),
        };
        Ok(Address {
            ip_data: octets,
            port: port,
        })
    }

    /// Get the `SocketAddr` for the address
    pub fn to_socket_addr(&self) -> Result<SocketAddr> {
        if self.ip_data.len() == 4 {
            Ok(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(
                    self.ip_data[0],
                    self.ip_data[1],
                    self.ip_data[2],
                    self.ip_data[3],
                )),
                self.port,
            ))
        } else if self.ip_data.len() == 16 {
            fn to_u16(x: u8, y: u8) -> u16 {
                use byteorder::{NetworkEndian, ReadBytesExt};
                use std::io::Cursor;
                let mut cursor = Cursor::new(vec![x, y]);
                cursor
                    .read_u16::<NetworkEndian>()
                    .expect("Error on casting two u8's to a u16")
            }

            Ok(SocketAddr::new(
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
            ))
        } else {
            Err(ErrorKind::IpAddressError(format!(
                "Invalid number of IP octets: {}",
                self.ip_data.len()
            )).into())
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_socket_addr().unwrap(),)
    }
}

/// Parameters for creating the local address of a client
pub struct LocalAddressParams {
    port: u16,
    interface_name: Option<String>,
    force_ipv6: bool,
}

impl LocalAddressParams {
    #[allow(missing_docs)]
    pub fn new(
        port: u16,
        interface_name: Option<String>,
        force_ipv6: bool,
    ) -> Self
    {
        LocalAddressParams {
            port,
            interface_name,
            force_ipv6,
        }
    }
}
