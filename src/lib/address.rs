use error::*;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

#[derive(Clone)]
pub struct Address {
    pub ip_data: Vec<u8>,
    pub port: u16
}

impl Address {
    pub fn new(ip_data: Vec<u8>, port: u16) -> Self {
        Address { ip_data: ip_data, port: port }
    }

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

