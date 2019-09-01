//! Request and response types for communication between daemons and CLIs

use api::Node;

/// Request with authenticity and secrecy
pub struct Request {
    /// The request sender's key
    pub sender: Node,
    /// The contents of the body encrypted with the recipient's public key,
    /// and signed with the sender's private key.
    pub encrypted_body: Vec<u8>,
}

impl Request {
    #[allow(missing_docs)]
    pub fn new(sender: Node, encrypted_body: Vec<u8>) -> Self {
        Request {
            sender,
            encrypted_body,
        }
    }
}

/// Response with authenticity and secrecy
pub struct Response {
    /// The contents of the body encrypted with the recipient's public key,
    /// and signed with the sender's private key.
    pub encrypted_body: Vec<u8>,
}

impl Response {
    #[allow(missing_docs)]
    pub fn new(encrypted_body: Vec<u8>) -> Self {
        Response { encrypted_body }
    }
}
