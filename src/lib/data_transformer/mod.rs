use error::*;
use request_handler::{Request, Response};

#[cfg(feature = "use-protobuf")]
pub mod protobuf;
#[cfg(feature = "use-protobuf")]
mod proto_api;

pub trait DataTransformer: Send + Sync {
    fn request_to_bytes(&self, request: &Request) -> Result<Vec<u8>>;
    fn bytes_to_request(&self, data: &Vec<u8>) -> Result<Request>;

    fn response_to_bytes(&self, response: &Response) -> Result<Vec<u8>>;
    fn bytes_to_response(&self, data: &Vec<u8>) -> Result<Response>;
}

