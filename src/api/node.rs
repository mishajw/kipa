use serde::Serialize;
use std::fmt;

use api::{Address, Key};

/// A node (i.e. user) in the network.
#[derive(Clone, Eq, PartialEq, Hash, Serialize)]
pub struct Node {
    /// The address of the node.
    ///
    /// Used for communicating with it.
    pub address: Address,
    /// The public key of the node.
    ///
    /// Used for deriving location in key space and securing communication.
    pub key: Key,
}

impl Node {
    #[allow(missing_docs)]
    pub fn new(address: Address, key: Key) -> Self {
        Node { address, key }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Node({}, {})", self.address, self.key)
    }
}
