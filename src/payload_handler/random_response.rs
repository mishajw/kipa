//! Implement `PayloadHandler` but return random results. Used for testing

use api::{Address, Key, Node};
use api::{RequestPayload, ResponsePayload};
use error::*;
use payload_handler::PayloadHandler;
use rand::{thread_rng, Rng};

use slog::Logger;

/// The request handler that returns nothing
pub struct RandomResponsePayloadHandler {
    log: Logger,
}

impl RandomResponsePayloadHandler {
    /// Create a new black hole request handler
    pub fn new(log: Logger) -> Self {
        RandomResponsePayloadHandler { log: log }
    }

    fn get_random_node() -> Node {
        let mut rng = thread_rng();
        let ip_data: Vec<u8> = (0..4).map(|_| rng.gen()).collect();
        let port: u16 = rng.gen();
        let key_id: String = (0..4).map(|_| format!("{:02X}", rng.gen::<u8>())).collect();
        let key_data: Vec<u8> = (0..1024).map(|_| rng.gen()).collect();

        Node::new(Address::new(ip_data, port), Key::new(key_id, key_data))
    }
}

impl PayloadHandler for RandomResponsePayloadHandler {
    fn receive(
        &self,
        payload: &RequestPayload,
        _sender: Option<Node>,
        _message_id: u32,
    ) -> InternalResult<ResponsePayload> {
        match payload {
            &RequestPayload::QueryRequest(_) => {
                trace!(self.log, "Received query request");
                // Generate a random number of random nodes
                let mut rng = thread_rng();
                Ok(ResponsePayload::QueryResponse(
                    (0..rng.gen_range::<i32>(0, 10))
                        .map(|_| Self::get_random_node())
                        .collect(),
                ))
            }
            &RequestPayload::SearchRequest(..) => {
                trace!(self.log, "Received search request");
                if thread_rng().gen() {
                    Ok(ResponsePayload::SearchResponse(None))
                } else {
                    Ok(ResponsePayload::SearchResponse(Some(
                        Self::get_random_node(),
                    )))
                }
            }
            &RequestPayload::ConnectRequest(_) => {
                trace!(self.log, "Received connect request");
                Ok(ResponsePayload::ConnectResponse())
            }
            &RequestPayload::ListNeighboursRequest() => {
                trace!(self.log, "Received list neighbours request");
                // We don't return random neighbours here, as this call is used
                // primarily for debugging purposes
                Ok(ResponsePayload::ListNeighboursResponse(vec![]))
            }
            &RequestPayload::VerifyRequest() => {
                trace!(self.log, "Received verify request");
                Ok(ResponsePayload::VerifyResponse())
            }
        }
    }
}
