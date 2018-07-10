//! Serialize and deserialize requests and responses for API messages over the
//! wire

use address::Address;
use api::{RequestBody, ResponseBody, SecureMessage};
use error::*;

#[cfg(feature = "use-protobuf")]
mod proto_api;
#[cfg(feature = "use-protobuf")]
pub mod protobuf;

/// Implementors must be able to convert `Request`s and `Response`s to and from
/// bytes
pub trait DataTransformer: Send + Sync {
    /// Encode a secure message into bytes
    fn encode_secure_message(&self, message: SecureMessage) -> Result<Vec<u8>>;
    /// Decode a secure message from bytes
    fn decode_secure_message(
        &self,
        data: &[u8],
        sender: Address,
    ) -> Result<SecureMessage>;

    /// Encode a request body into bytes
    fn encode_request_body(&self, body: RequestBody) -> Result<Vec<u8>>;
    /// Decode a request body from bytes
    fn decode_request_body(&self, data: &[u8]) -> Result<RequestBody>;

    /// Encode a response body into bytes
    fn encode_response_body(&self, body: ResponseBody) -> Result<Vec<u8>>;
    /// Decode a response body from bytes
    fn decode_response_body(&self, data: &[u8]) -> Result<ResponseBody>;
}
