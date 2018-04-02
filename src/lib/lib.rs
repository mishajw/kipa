//! Library to create and interface with KIPA daemons and other KIPA nodes.
//!
//! Structure for communicating between nodes is a `RequestHandler` that uses
//! `server::PublicServer` and `server::PrivateServer` to receive and send
//! messages between nodes.
//!
//! Communcation between these components is passed through a
//! `data_transformer::DataTransformer` to serialise requests and responses.

#![warn(missing_docs)]

#[macro_use] extern crate cfg_if;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate log;
extern crate byteorder;
extern crate clap;
extern crate gpgme;
extern crate protobuf;

pub mod creators;
pub mod data_transformer;
pub mod error;
pub mod global_server;
pub mod gpg_key;
pub mod local_server;
pub mod request_handler;
pub mod server;

mod address;
pub use address::Address;

mod key;
pub use key::Key;

mod node;
pub use node::Node;

