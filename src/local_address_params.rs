//! Parameters for creating the local address of a client

use api::Address;
use error::*;

use std::net::IpAddr;

use pnet::datalink;
use slog::Logger;

/// The default port for server communication
pub const DEFAULT_PORT: &str = "10842";

/// Contains information of the local address
pub struct LocalAddressParams {
    port: u16,
    interface_name: Option<String>,
    force_ipv6: bool,
}

impl LocalAddressParams {
    #[allow(missing_docs)]
    pub fn new(port: u16, interface_name: Option<String>, force_ipv6: bool) -> Self {
        LocalAddressParams {
            port,
            interface_name,
            force_ipv6,
        }
    }

    /// Get the local address on a specified interface
    pub fn create_address(self, log: Logger) -> InternalResult<Address> {
        let LocalAddressParams {
            port,
            interface_name,
            force_ipv6,
        } = self;
        for interface in datalink::interfaces() {
            // Skip interfaces that are loopback or have no IPs
            if interface_name.is_none() && (interface.name == "lo") || interface.ips.is_empty() {
                continue;
            }

            // Skip if we've specified a name, and this interface doesn't match
            if interface_name.is_some() && interface.name != interface_name.clone().unwrap() {
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
                return Err(InternalError::private(ErrorKind::IpAddressError(format!(
                    "Could not find any IP address on interface {}, \
                     found: {:?}",
                    interface.name, ips
                ))));
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
}
