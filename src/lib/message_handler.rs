//! Handle messages received from a server.

use api::{MessageSender, RequestMessage, RequestPayload, ResponseMessage,
          ResponsePayload};
use server::Client;
use error::*;
use node::Node;
use payload_handler::PayloadHandler;

use std::sync::Arc;

/// The message handling struct.
pub struct MessageHandler {
    payload_handler: Arc<PayloadHandler>,
    local_node: Node,
    client: Arc<Client>,
}

impl MessageHandler {
    /// Create a new `MessageHandler` with a `PayloadHandler` to pass payloads
    /// to.
    pub fn new(
        payload_handler: Arc<PayloadHandler>,
        local_node: Node,
        client: Arc<Client>,
    ) -> Self {
        MessageHandler {
            payload_handler: payload_handler,
            local_node: local_node,
            client: client,
        }
    }

    /// Receive and handle a request message, returning a response message.
    pub fn receive(&self, message: RequestMessage) -> Result<ResponseMessage> {
        let sender = match &message.sender {
            &MessageSender::Node(ref n) => Some(n),
            &MessageSender::Cli() => None,
        };

        let payload_client = Arc::new(PayloadClient::new(
            message.id,
            self.local_node.clone(),
            self.client.clone(),
        ));

        let response_payload = self.payload_handler.receive(
            &message.payload,
            sender,
            payload_client,
            message.id,
        )?;

        let response = ResponseMessage::new(
            response_payload,
            MessageSender::Node(self.local_node.clone()),
            message.id,
        );

        Ok(response)
    }
}

/// Client that will take a payload, wrap it in a message, and send to another
/// node.
pub struct PayloadClient {
    message_id: u32,
    local_node: Node,
    client: Arc<Client>,
}

impl PayloadClient {
    /// Create a new `PayloadClient` with information needed to create a message
    /// and send it to another node.
    pub fn new(
        message_id: u32,
        local_node: Node,
        client: Arc<Client>,
    ) -> PayloadClient {
        PayloadClient {
            message_id: message_id,
            local_node: local_node,
            client: client,
        }
    }

    /// Send a payload to a node.
    pub fn send(
        &self,
        node: &Node,
        payload: RequestPayload,
    ) -> Result<ResponsePayload> {
        let request_message = RequestMessage::new(
            payload,
            MessageSender::Node(self.local_node.clone()),
            self.message_id,
        );

        let response_message = self.client.send(node, request_message)?;

        if response_message.id != self.message_id {
            return Err(ErrorKind::ResponseError(format!(
                "Response had incorrect ID, expected {}, received {}",
                self.message_id, response_message.id
            )).into());
        }

        Ok(response_message.payload)
    }
}
