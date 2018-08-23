//! Library containing code for the daemon and command line interface for KIPA
//!
//! For an introduction to KIPA, please see the [README.md]. For an overview on
//! the design of KIPA, please see the [design document].
//!
//! [README.md]: https://github.com/mishajw/kipa
//! [design document]: https://github.com/mishajw/kipa/blob/master/docs/design.md

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
extern crate periodic;
extern crate rand;

pub mod api;
pub mod creators;
pub mod data_transformer;
pub mod error;
pub mod gpg_key;
pub mod key_space;
pub mod message_handler;
pub mod payload_handler;
pub mod server;
mod versioning;

mod address;
pub use address::Address;
pub use address::LocalAddressParams;

mod key;
pub use key::Key;

mod node;
pub use node::Node;
