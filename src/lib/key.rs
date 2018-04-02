//! Keys used for locating nodes and encryping messages.

/// Holds the data for some key implementation.
#[derive(Clone)]
pub struct Key {
    data: Vec<u8>
}

impl Key {
    /// Create a new key for some data.
    pub fn new(data: Vec<u8>) -> Self {
        Key {data: data}
    }

    /// Get the data of some key.
    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }
}

