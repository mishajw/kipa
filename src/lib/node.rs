//! Holds information on another KIPA node.

use address::Address;
use key::Key;

/// The information on a KIPA node.
#[derive(Clone)]
pub struct Node {
    /// The address of the node used for communicating with it.
    pub address: Address,
    /// The key of the node used for locating it.
    pub key: Key
}

impl Node {
    /// Create a new node with some `Address` and `Key`.
    pub fn new(address: Address, key: Key) -> Self {
        Node {
            address: address, key: key
        }
    }
}

