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
extern crate num_cpus;
extern crate periodic;
extern crate rand;
#[cfg(feature = "use-remotery")]
extern crate remotery;
extern crate threadpool;

// Defines globally used macros, must be first
pub mod remotery_util;

// Main API and building blocks
pub mod api;

// Code for sending/receiving messages
pub mod data_transformer;
pub mod gpg_key;
pub mod key_space_manager;
pub mod message_handler;
pub mod payload_handler;
pub mod server;

// Utilities
pub mod creators;
pub mod local_address_params;
pub mod thread_manager;
mod versioning;

pub mod error;
