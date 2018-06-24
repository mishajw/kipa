//! Keys used for locating nodes and encryping messages

use std::fmt;

/// The key data belonging to a node
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Key {
    // TODO: Change from String to [u8]
    key_id: String,
    data: Vec<u8>,
}

impl Key {
    #[allow(missing_docs)]
    pub fn new(key_id: String, data: Vec<u8>) -> Self {
        assert!(key_id.len() == 8);
        Key { key_id, data }
    }

    #[allow(missing_docs)]
    pub fn get_data(&self) -> &Vec<u8> { &self.data }

    #[allow(missing_docs)]
    pub fn get_key_id(&self) -> &String { &self.key_id }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Key({})", self.key_id)
    }
}
