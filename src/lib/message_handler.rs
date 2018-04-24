//! Handle messages received from a server.

use api::{MessageSender, RequestMessage, ResponseMessage};
use error::*;
use node::Node;
use payload_handler::PayloadHandler;

use std::sync::Arc;

/// The message handling struct.
pub struct MessageHandler {
    payload_handler: Arc<PayloadHandler>,
    local_node: Node,
}

impl MessageHandler {
    /// Create a new `MessageHandler` with a `PayloadHandler` to pass payloads
    /// to.
    pub fn new(payload_handler: Arc<PayloadHandler>, local_node: Node) -> Self {
        MessageHandler {
            payload_handler: payload_handler,
            local_node: local_node,
        }
    }

    /// Receive and handle a request message, returning a response message.
    pub fn receive(&self, message: RequestMessage) -> Result<ResponseMessage> {
        let sender = match &message.sender {
            &MessageSender::Node(ref n) => Some(n),
            &MessageSender::Cli() => None,
        };

        let response_payload =
            self.payload_handler.receive(&message.payload, sender)?;

        let response = ResponseMessage::new(
            response_payload,
            MessageSender::Node(self.local_node.clone()),
        );

        Ok(response)
    }
}
