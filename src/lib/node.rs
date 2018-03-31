use address::Address;
use key::Key;

#[derive(Clone)]
pub struct Node {
    pub address: Address,
    pub key: Key
}

impl Node {
    pub fn new(address: Address, key: Key) -> Self {
        Node {
            address: address, key: key
        }
    }
}

