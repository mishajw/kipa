//! Handle messages sent to a daemon

use address::Address;
use api::{
    ApiVisibility, RequestBody, RequestPayload, ResponseBody, ResponsePayload,
    SecureMessage,
};
use data_transformer::DataTransformer;
use error::*;
use gpg_key::GpgKeyHandler;
use node::Node;
use payload_handler::PayloadHandler;
use server::Client;
use versioning;

use slog::Logger;
use std::sync::Arc;
use std::time::Duration;

/// The message handling struct
pub struct MessageHandler {
    payload_handler: Arc<PayloadHandler>,
    data_transformer: Arc<DataTransformer>,
    gpg_key_handler: Arc<GpgKeyHandler>,
    local_node: Node,
    client: Arc<Client>,
    log: Logger,
}

impl MessageHandler {
    /// Create a new `MessageHandler` with a `PayloadHandler` to pass payloads
    /// to
    pub fn new(
        payload_handler: Arc<PayloadHandler>,
        data_transformer: Arc<DataTransformer>,
        gpg_key_handler: Arc<GpgKeyHandler>,
        local_node: Node,
        client: Arc<Client>,
        log: Logger,
    ) -> Self
    {
        MessageHandler {
            payload_handler,
            data_transformer,
            gpg_key_handler,
            local_node,
            client,
            log,
        }
    }

    /// Receive and handle the bytes of a request message, returning the bytes
    /// of response message
    ///
    /// This function, and all the other `receive` functions return a `Result`
    /// and not an `InternalResult` because the issues with communication are
    /// usually internal errors. Public errors can occur when computing
    /// the result of a query, but this is captured in the payload `ApiError<_>`
    /// value.
    pub fn receive_bytes(
        &self,
        request_data: &[u8],
        address: Option<Address>,
    ) -> Result<Vec<u8>>
    {
        debug!(self.log, "Received bytes"; "from_cli" => address.is_none());

        match address {
            Some(address) => {
                let message = self
                    .data_transformer
                    .decode_secure_message(request_data, address)?;
                let response_message = self.receive_secure_message(message)?;
                Ok(self
                    .data_transformer
                    .encode_secure_message(response_message)?)
            }
            None => {
                let body =
                    self.data_transformer.decode_request_body(request_data)?;
                let response_body = self.receive_body(body, None)?;
                Ok(self.data_transformer.encode_response_body(response_body)?)
            }
        }
    }

    fn receive_secure_message(
        &self,
        secure_message: SecureMessage,
    ) -> Result<SecureMessage>
    {
        debug!(self.log, "Received secure message");

        let decrypted_body_data = self
            .gpg_key_handler
            .decrypt(&secure_message.encrypted_body, &self.local_node.key)?;
        self.gpg_key_handler.verify(
            &decrypted_body_data,
            &secure_message.body_signature,
            &secure_message.sender.key,
        )?;
        let body = self
            .data_transformer
            .decode_request_body(&decrypted_body_data)?;
        let response_body =
            self.receive_body(body, Some(&secure_message.sender))?;
        let response_body_data =
            self.data_transformer.encode_response_body(response_body)?;
        let encrypted_response_body_data = self
            .gpg_key_handler
            .encrypt(&response_body_data, &secure_message.sender.key)?;
        let signed_response_body_data = self
            .gpg_key_handler
            .sign(&response_body_data, &self.local_node.key)?;
        Ok(SecureMessage::new(
            self.local_node.clone(),
            signed_response_body_data,
            encrypted_response_body_data,
        ))
    }

    /// Receive and handle a request message, returning a response message
    fn receive_body(
        &self,
        body: RequestBody,
        sender: Option<&Node>,
    ) -> Result<ResponseBody>
    {
        debug!(self.log, "Received request body");

        // Check the visibility of the request is correct
        let visibility = match sender {
            Some(_) => ApiVisibility::Global(),
            None => ApiVisibility::Local(),
        };
        if !body.payload.is_visible(&visibility) {
            return Err(ErrorKind::RequestError(
                "Request is not locally available".into(),
            ).into());
        }

        let payload_client = Arc::new(PayloadClient::new(
            body.id,
            self.local_node.clone(),
            self.client.clone(),
            self.data_transformer.clone(),
            self.gpg_key_handler.clone(),
        ));

        let version_verification_result =
            api_to_internal_result(versioning::verify_version(
                &versioning::get_version(),
                &body.version,
            ));
        let response_payload_result =
            version_verification_result.and_then(|()| {
                self.payload_handler.receive(
                    &body.payload,
                    sender,
                    payload_client,
                    body.id,
                )
            });

        Ok(ResponseBody::new(
            to_api_result(response_payload_result, &self.log),
            body.id,
            versioning::get_version(),
        ))
    }
}

/// Client that will take a payload, wrap it in a message, and send to another
/// node
pub struct PayloadClient {
    message_id: u32,
    local_node: Node,
    client: Arc<Client>,
    data_transformer: Arc<DataTransformer>,
    gpg_key_handler: Arc<GpgKeyHandler>,
}

impl PayloadClient {
    /// Create a new `PayloadClient` with information needed to create a message
    /// and send it to another node
    pub fn new(
        message_id: u32,
        local_node: Node,
        client: Arc<Client>,
        data_transformer: Arc<DataTransformer>,
        gpg_key_handler: Arc<GpgKeyHandler>,
    ) -> PayloadClient
    {
        PayloadClient {
            message_id,
            local_node,
            client,
            data_transformer,
            gpg_key_handler,
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
        let body = RequestBody::new(
            payload,
            self.message_id,
            versioning::get_version(),
        );

        let body_data = to_internal_result(
            self.data_transformer.encode_request_body(body),
        )?;
        let encrypted_body_data = to_internal_result(
            self.gpg_key_handler.encrypt(&body_data, &node.key),
        )?;
        let signed_body_data = to_internal_result(
            self.gpg_key_handler.sign(&body_data, &self.local_node.key),
        )?;

        let message = SecureMessage::new(
            self.local_node.clone(),
            signed_body_data,
            encrypted_body_data,
        );
        let message_data = to_internal_result(
            self.data_transformer.encode_secure_message(message),
        )?;

        let response_message_data =
            to_internal_result(self.client.send(node, &message_data, timeout))?;
        let response_message =
            to_internal_result(self.data_transformer.decode_secure_message(
                &response_message_data,
                node.address.clone(),
            ))?;

        let response_body_data = self
            .gpg_key_handler
            .decrypt(&response_message.encrypted_body, &self.local_node.key)
            .map_err(|err| {
                InternalError::public_with_error(
                    "Failed to decrypt message",
                    ApiErrorType::Parse,
                    err,
                )
            })?;

        self.gpg_key_handler
            .verify(
                &response_body_data,
                &response_message.body_signature,
                &node.key,
            )
            .map_err(|err| {
                InternalError::public_with_error(
                    "Invalid signature",
                    ApiErrorType::Parse,
                    err,
                )
            })?;

        let response_body = to_internal_result(
            self.data_transformer
                .decode_response_body(&response_body_data),
        )?;

        if response_body.id != self.message_id {
            // TODO: We need to reference `InternalError` here instead of
            // `ResponseError` - seems that when you typedef enums, referencing
            // the instances of the enum still needs to be done through the
            // original enum type. Find a solution to this, and make sure that
            // *all* mentions of `{Public,Private}Error` are to the correct enum
            // type.
            return Err(InternalError::private(ErrorKind::ResponseError(
                format!(
                    "Response had incorrect ID, expected {}, received {}",
                    self.message_id, response_body.id
                ),
            )));
        }

        api_to_internal_result(response_body.payload)
    }
}
