//! Handle messages sent to a daemon

use address::Address;
use api::{
    MessageSender, RequestMessage, RequestPayload, ResponseMessage,
    ResponsePayload,
};
use data_transformer::DataTransformer;
use error::*;
use node::Node;
use payload_handler::PayloadHandler;
use server::Client;
use versioning;

use std::sync::Arc;
use std::time::Duration;

/// The message handling struct
pub struct MessageHandler {
    payload_handler: Arc<PayloadHandler>,
    data_transformer: Arc<DataTransformer>,
    local_node: Node,
    client: Arc<Client>,
}

impl MessageHandler {
    /// Create a new `MessageHandler` with a `PayloadHandler` to pass payloads
    /// to
    pub fn new(
        payload_handler: Arc<PayloadHandler>,
        data_transformer: Arc<DataTransformer>,
        local_node: Node,
        client: Arc<Client>,
    ) -> Self
    {
        MessageHandler {
            payload_handler,
            data_transformer,
            local_node,
            client,
        }
    }

    /// Receive and handle the bytes of a request message, returning the bytes
    /// of response message
    pub fn receive_bytes(
        &self,
        request_data: &[u8],
        address: Option<Address>,
    ) -> Result<Vec<u8>>
    {
        let request = self
            .data_transformer
            .bytes_to_request(&request_data.to_vec(), address)?;

        let response = self.receive_message(&request)?;

        self.data_transformer.response_to_bytes(&response)
    }

    /// Receive and handle a request message, returning a response message
    pub fn receive_message(
        &self,
        message: &RequestMessage,
    ) -> Result<ResponseMessage>
    {
        let sender = match message.sender {
            MessageSender::Node(ref n) => Some(n),
            MessageSender::Cli() => None,
        };

        let payload_client = Arc::new(PayloadClient::new(
            message.id,
            self.local_node.clone(),
            self.client.clone(),
        ));

        let version_verification_result =
            api_to_internal_result(versioning::verify_version(
                &versioning::get_version(),
                &message.version,
            ));
        let response_payload_result =
            version_verification_result.and_then(|()| {
                self.payload_handler.receive(
                    &message.payload,
                    sender,
                    payload_client,
                    message.id,
                )
            });

        let response = ResponseMessage::new(
            to_api_result(response_payload_result),
            MessageSender::Node(self.local_node.clone()),
            message.id,
            versioning::get_version(),
        );

        Ok(response)
    }
}

/// Client that will take a payload, wrap it in a message, and send to another
/// node
pub struct PayloadClient {
    message_id: u32,
    local_node: Node,
    client: Arc<Client>,
}

impl PayloadClient {
    /// Create a new `PayloadClient` with information needed to create a message
    /// and send it to another node
    pub fn new(
        message_id: u32,
        local_node: Node,
        client: Arc<Client>,
    ) -> PayloadClient
    {
        PayloadClient {
            message_id,
            local_node,
            client,
        }
    }

    /// Send a payload to a node
    pub fn send(
        &self,
        node: &Node,
        payload: RequestPayload,
        timeout: Duration,
    ) -> ResponseResult<ResponsePayload>
    {
        let request_message = RequestMessage::new(
            payload,
            MessageSender::Node(self.local_node.clone()),
            self.message_id,
            versioning::get_version(),
        );

        let response_message = to_internal_result(self.client.send(
            node,
            request_message,
            timeout,
        ))?;

        if response_message.id != self.message_id {
            // TODO: We need to reference `InternalError` here instead of
            // `ResponseError` - seems that when you typedef enums, referencing
            // the instances of the enum still needs to be done through the
            // original enum type. Find a solution to this, and make sure that
            // *all* mentions of `{Public,Private}Error` are to the correct enum
            // type.
            return Err(InternalError::private(ErrorKind::ResponseError(
                format!(
                    "Response had incorrect ID, expected {}, received {}",
                    self.message_id, response_message.id
                ),
            )));
        }

        api_to_internal_result(response_message.payload)
    }
}
