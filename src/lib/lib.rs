//! Library to create and interface with KIPA daemons and other KIPA nodes.
//!
//! Structure for communicating between nodes is a `PayloadHandler` that uses
//! `server::PublicServer` and `server::PrivateServer` to receive and send
//! messages between nodes.
//!
//! Communcation between these components is passed through a
//! `data_transformer::DataTransformer` to serialise requests and responses.

#![warn(missing_docs)]

extern crate byteorder;
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate gpgme;
extern crate pnet;
extern crate protobuf;
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_json;
extern crate slog_term;
#[cfg(test)]
#[allow(unused)]
#[macro_use]
extern crate spectral;
extern crate regex;
#[macro_use]
extern crate lazy_static;

pub mod api;
pub mod creators;
pub mod data_transformer;
pub mod error;
pub mod gpg_key;
pub mod message_handler;
pub mod payload_handler;
pub mod server;
pub mod socket_server;
mod versioning;

mod address;
pub use address::Address;
pub use address::LocalAddressParams;

mod key;
pub use key::Key;

mod node;
pub use node::Node;
