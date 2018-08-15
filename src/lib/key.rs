//! Keys used for locating nodes and encryping messages

use std::fmt;

/// The key data belonging to a node
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Key {
    /// Key identifier
    pub key_id: String,
    /// Cryptographic key data
    pub data: Vec<u8>,
}

impl Key {
    #[allow(missing_docs)]
    pub fn new(key_id: String, data: Vec<u8>) -> Self {
        assert_eq!(key_id.len(), 8);
        Key { key_id, data }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Key({})", self.key_id)
    }
}
