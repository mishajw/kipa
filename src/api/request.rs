//! Request and response types for communication between daemons and CLIs

use api::Node;

use std::fmt;

/// Request with authenticity and secrecy
pub struct PrivateRequest {
    /// The request sender's key
    pub sender: Node,
    /// Signature of the decrypted body, signed by sender's private key
    pub body_signature: Vec<u8>,
    /// The contents of the body encrypted with the recipient's public key
    pub encrypted_body: Vec<u8>,
}

impl PrivateRequest {
    #[allow(missing_docs)]
    pub fn new(
        sender: Node,
        body_signature: Vec<u8>,
        encrypted_body: Vec<u8>,
    ) -> Self
    {
        PrivateRequest {
            sender,
            body_signature,
            encrypted_body,
        }
    }
}

/// Response with authenticity and secrecy
pub struct PrivateResponse {
    /// Signature of the decrypted body, signed by sender's private key
    pub body_signature: Vec<u8>,
    /// The contents of the body encrypted with the recipient's public key
    pub encrypted_body: Vec<u8>,
}

impl PrivateResponse {
    #[allow(missing_docs)]
    pub fn new(body_signature: Vec<u8>, encrypted_body: Vec<u8>) -> Self {
        PrivateResponse {
            body_signature,
            encrypted_body,
        }
    }
}

/// Request with no authenticity or secrecy
pub struct FastRequest {
    /// Encoded data of the body
    pub body: Vec<u8>,
    /// Address of the sender
    pub sender: Node,
}

impl FastRequest {
    #[allow(missing_docs)]
    pub fn new(body: Vec<u8>, sender: Node) -> Self {
        FastRequest { body, sender }
    }
}

/// Response with authenticity but no secrecy
pub struct FastResponse {
    /// Encoded data of the body
    pub body: Vec<u8>,
    /// Signature of the body, signed with the sender's private key
    pub body_signature: Vec<u8>,
}

impl FastResponse {
    #[allow(missing_docs)]
    pub fn new(body: Vec<u8>, body_signature: Vec<u8>) -> Self {
        FastResponse {
            body,
            body_signature,
        }
    }
}

/// Request message in either mode
pub enum RequestMessage {
    #[allow(missing_docs)]
    Private(PrivateRequest),
    #[allow(missing_docs)]
    Fast(FastRequest),
}

/// Request message in either mode
pub enum ResponseMessage {
    #[allow(missing_docs)]
    Private(PrivateResponse),
    #[allow(missing_docs)]
    Fast(FastResponse),
}

/// Different modes of search
///
/// See the [design document] for details.
///
/// [design document]: https://github.com/mishajw/kipa/blob/master/docs/design.md#messaging-protocol
#[derive(Clone)]
pub enum MessageMode {
    /// Fast mode, with no encryption
    Fast(),
    /// Private mode, with encryption
    Private(),
}

impl fmt::Display for MessageMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessageMode::Fast() => write!(f, "MessageMode::Fast"),
            MessageMode::Private() => write!(f, "MessageMode::Private"),
        }
    }
}
