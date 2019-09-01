//! Request and response types for communication between daemons and CLIs

use api::Node;

/// Request with authenticity and secrecy
pub struct Request {
    /// The request sender's key
    pub sender: Node,
    /// Signature of the decrypted body, signed by sender's private key
    pub body_signature: Vec<u8>,
    /// The contents of the body encrypted with the recipient's public key
    pub encrypted_body: Vec<u8>,
}

impl Request {
    #[allow(missing_docs)]
    pub fn new(
        sender: Node,
        body_signature: Vec<u8>,
        encrypted_body: Vec<u8>,
    ) -> Self {
        Request {
            sender,
            body_signature,
            encrypted_body,
        }
    }
}

/// Response with authenticity and secrecy
pub struct Response {
    /// Signature of the decrypted body, signed by sender's private key
    pub body_signature: Vec<u8>,
    /// The contents of the body encrypted with the recipient's public key
    pub encrypted_body: Vec<u8>,
}

impl Response {
    #[allow(missing_docs)]
    pub fn new(body_signature: Vec<u8>, encrypted_body: Vec<u8>) -> Self {
        Response {
            body_signature,
            encrypted_body,
        }
    }
}
