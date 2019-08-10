//! Handle messages sent to a daemon

use api::error::ApiErrorType;
use api::request::{
    FastRequest, FastResponse, MessageMode, PrivateRequest, PrivateResponse,
    RequestMessage, ResponseMessage,
};
use api::{Address, Node};
use api::{
    ApiVisibility, RequestBody, RequestPayload, ResponseBody, ResponsePayload,
};
use data_transformer::DataTransformer;
use error::*;
use payload_handler::PayloadHandler;
use pgp::PgpKeyHandler;
use server::{Client, LocalClient};
use versioning;

use rand::{thread_rng, Rng};
use slog::Logger;
use std::sync::Arc;
use std::time::Duration;

/// Handles messages incoming from external sources (i.e. another daemon, CLI)
pub struct MessageHandlerServer {
    payload_handler: Arc<PayloadHandler>,
    local_node: Node,
    data_transformer: Arc<DataTransformer>,
    pgp_key_handler: Arc<PgpKeyHandler>,
    log: Logger,
}

impl MessageHandlerServer {
    #[allow(missing_docs)]
    pub fn new(
        payload_handler: Arc<PayloadHandler>,
        local_node: Node,
        data_transformer: Arc<DataTransformer>,
        pgp_key_handler: Arc<PgpKeyHandler>,
        log: Logger,
    ) -> Self {
        MessageHandlerServer {
            payload_handler,
            local_node,
            data_transformer,
            pgp_key_handler,
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
    ) -> Result<Vec<u8>> {
        remotery_scope!("message_handler_receive_bytes");

        debug!(self.log, "Received bytes"; "from_cli" => address.is_none());

        match address {
            Some(address) => {
                let message = self
                    .data_transformer
                    .decode_request_message(request_data, address)?;
                let response_message = self.receive_request_message(message)?;
                Ok(self
                    .data_transformer
                    .encode_response_message(response_message)?)
            }
            None => {
                let body =
                    self.data_transformer.decode_request_body(request_data)?;
                let response_body = self.receive_body(body, None)?;
                Ok(self.data_transformer.encode_response_body(response_body)?)
            }
        }
    }

    fn receive_request_message(
        &self,
        secure_request: RequestMessage,
    ) -> Result<ResponseMessage> {
        remotery_scope!("message_handler_receive_request_message");

        Ok(match secure_request {
            RequestMessage::Private(private) => {
                ResponseMessage::Private(self.receive_private_message(private)?)
            }
            RequestMessage::Fast(fast) => {
                ResponseMessage::Fast(self.receive_fast_message(fast)?)
            }
        })
    }

    fn receive_private_message(
        &self,
        private_request: PrivateRequest,
    ) -> Result<PrivateResponse> {
        remotery_scope!("message_handler_receive_private_message");

        debug!(self.log, "Received secure message");

        let decrypted_body_data = self.pgp_key_handler.decrypt(
            &private_request.encrypted_body,
            &private_request.sender.key,
            &self.local_node.key,
        )?;
        let body = self
            .data_transformer
            .decode_request_body(&decrypted_body_data)?;
        let response_body =
            self.receive_body(body, Some(private_request.sender.clone()))?;
        let response_body_data =
            self.data_transformer.encode_response_body(response_body)?;
        let encrypted_response_body_data = self.pgp_key_handler.encrypt(
            &response_body_data,
            &self.local_node.key,
            &private_request.sender.key,
        )?;
        let signed_response_body_data = self
            .pgp_key_handler
            .sign(&response_body_data, &self.local_node.key)?;
        Ok(PrivateResponse::new(
            signed_response_body_data,
            encrypted_response_body_data,
        ))
    }

    fn receive_fast_message(
        &self,
        fast_request: FastRequest,
    ) -> Result<FastResponse> {
        remotery_scope!("message_handler_receive_fast_message");

        debug!(self.log, "Received secure message");

        let body = self
            .data_transformer
            .decode_request_body(&fast_request.body)?;

        let response_body =
            self.receive_body(body, Some(fast_request.sender))?;
        let response_body_data =
            self.data_transformer.encode_response_body(response_body)?;
        let signed_response_body_data = self
            .pgp_key_handler
            .sign(&response_body_data, &self.local_node.key)?;
        Ok(FastResponse::new(
            response_body_data,
            signed_response_body_data,
        ))
    }

    /// Receive and handle a request message, returning a response message
    fn receive_body(
        &self,
        body: RequestBody,
        sender: Option<Node>,
    ) -> Result<ResponseBody> {
        remotery_scope!("message_handler_receive_body");

        debug!(self.log, "Received request body");

        // Check the visibility of the request is correct
        let visibility = match sender {
            Some(_) => ApiVisibility::Global(),
            None => ApiVisibility::Local(),
        };
        if !body.payload.is_visible(&visibility) {
            return Err(ErrorKind::RequestError(
                "Request is not locally available".into(),
            )
            .into());
        }

        let version_verification_result =
            api_to_internal_result(versioning::verify_version(
                &versioning::get_version(),
                &body.version,
            ));
        let response_payload_result =
            version_verification_result.and_then(|()| {
                self.payload_handler.receive(&body.payload, sender, body.id)
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
pub struct MessageHandlerClient {
    local_node: Node,
    client: Arc<Client>,
    data_transformer: Arc<DataTransformer>,
    pgp_key_handler: Arc<PgpKeyHandler>,
    log: Logger,
}

impl MessageHandlerClient {
    #[allow(missing_docs)]
    pub fn new(
        local_node: Node,
        client: Arc<Client>,
        data_transformer: Arc<DataTransformer>,
        pgp_key_handler: Arc<PgpKeyHandler>,
        log: Logger,
    ) -> MessageHandlerClient {
        MessageHandlerClient {
            local_node,
            client,
            data_transformer,
            pgp_key_handler,
            log,
        }
    }

    /// Send a message in a specified message mode
    pub fn send_message(
        &self,
        node: &Node,
        payload: RequestPayload,
        timeout: Duration,
        mode: &MessageMode,
    ) -> ResponseResult<ResponsePayload> {
        remotery_scope!("message_handler_client_send_message");

        match mode {
            MessageMode::Fast() => {
                self.send_fast_message(node, payload, timeout)
            }
            MessageMode::Private() => {
                self.send_private_message(node, payload, timeout)
            }
        }
    }

    /// Send a payload to a node in private mode
    pub fn send_private_message(
        &self,
        node: &Node,
        payload: RequestPayload,
        timeout: Duration,
    ) -> ResponseResult<ResponsePayload> {
        remotery_scope!("message_handler_client_send_private_message");

        let message_id: u32 = thread_rng().gen();
        debug!(
            self.log, "Sending private request"; "message_id" => message_id);
        let body =
            RequestBody::new(payload, message_id, versioning::get_version());

        let body_data = to_internal_result(
            self.data_transformer.encode_request_body(body),
        )?;
        let encrypted_body_data =
            to_internal_result(self.pgp_key_handler.encrypt(
                &body_data,
                &self.local_node.key,
                &node.key,
            ))?;
        let signed_body_data = to_internal_result(
            self.pgp_key_handler.sign(&body_data, &self.local_node.key),
        )?;

        let message = PrivateRequest::new(
            self.local_node.clone(),
            signed_body_data,
            encrypted_body_data,
        );
        let message_data = to_internal_result(
            self.data_transformer
                .encode_request_message(RequestMessage::Private(message)),
        )?;

        let response_message_data =
            to_internal_result(self.client.send(node, &message_data, timeout))?;
        let response_message =
            to_internal_result(self.data_transformer.decode_response_message(
                &response_message_data,
                node.address.clone(),
            ))?;

        let private_response = match response_message {
            ResponseMessage::Private(private) => private,
            ResponseMessage::Fast(_) => {
                return Err(InternalError::private(ErrorKind::ResponseError(
                    "Expected private mode response, found fast mode response"
                        .into(),
                )));
            }
        };

        let response_body_data = self
            .pgp_key_handler
            .decrypt(
                &private_response.encrypted_body,
                &node.key,
                &self.local_node.key,
            )
            .map_err(|err| {
                InternalError::public_with_error(
                    "Failed to decrypt message",
                    ApiErrorType::Parse,
                    err,
                )
            })?;

        self.pgp_key_handler
            .verify(
                &response_body_data,
                &private_response.body_signature,
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

        if response_body.id != message_id {
            // TODO: We need to reference `InternalError` here instead of
            // `ResponseError` - seems that when you typedef enums, referencing
            // the instances of the enum still needs to be done through the
            // original enum type. Find a solution to this, and make sure that
            // *all* mentions of `{Public,Private}Error` are to the correct enum
            // type.
            return Err(InternalError::private(ErrorKind::ResponseError(
                format!(
                    "Response had incorrect ID, expected {}, received {}",
                    message_id, response_body.id
                ),
            )));
        }

        api_to_internal_result(response_body.payload)
    }

    /// Send a payload to a node in fast mode
    pub fn send_fast_message(
        &self,
        node: &Node,
        payload: RequestPayload,
        timeout: Duration,
    ) -> ResponseResult<ResponsePayload> {
        remotery_scope!("message_handler_client_send_fast_message");

        let message_id: u32 = thread_rng().gen();
        debug!(
            self.log, "Sending fast request"; "message_id" => message_id);
        let body =
            RequestBody::new(payload, message_id, versioning::get_version());

        let body_data = to_internal_result(
            self.data_transformer.encode_request_body(body),
        )?;

        let message = FastRequest::new(body_data, self.local_node.clone());
        let message_data = to_internal_result(
            self.data_transformer
                .encode_request_message(RequestMessage::Fast(message)),
        )?;

        let response_message_data =
            to_internal_result(self.client.send(node, &message_data, timeout))?;
        let response_message =
            to_internal_result(self.data_transformer.decode_response_message(
                &response_message_data,
                node.address.clone(),
            ))?;

        let fast_response = match response_message {
            ResponseMessage::Fast(fast) => fast,
            ResponseMessage::Private(_) => {
                return Err(InternalError::private(ErrorKind::ResponseError(
                    "Expected fast mode response, found private mode response"
                        .into(),
                )));
            }
        };

        self.pgp_key_handler
            .verify(
                &fast_response.body,
                &fast_response.body_signature,
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
                .decode_response_body(&fast_response.body),
        )?;

        if response_body.id != message_id {
            // TODO: We need to reference `InternalError` here instead of
            // `ResponseError` - seems that when you typedef enums, referencing
            // the instances of the enum still needs to be done through the
            // original enum type. Find a solution to this, and make sure that
            // *all* mentions of `{Public,Private}Error` are to the correct enum
            // type.
            return Err(InternalError::private(ErrorKind::ResponseError(
                format!(
                    "Response had incorrect ID, expected {}, received {}",
                    message_id, response_body.id
                ),
            )));
        }

        api_to_internal_result(response_body.payload)
    }
}

/// Client that will take a payload, wrap it in a message, and send to a local
/// daemon node
pub struct MessageHandlerLocalClient {
    local_client: Arc<LocalClient>,
    data_transformer: Arc<DataTransformer>,
    log: Logger,
}

impl MessageHandlerLocalClient {
    #[allow(missing_docs)]
    pub fn new(
        local_client: Arc<LocalClient>,
        data_transformer: Arc<DataTransformer>,
        log: Logger,
    ) -> Self {
        MessageHandlerLocalClient {
            local_client,
            data_transformer,
            log,
        }
    }

    /// Send a payload to a node
    pub fn send(
        &self,
        payload: RequestPayload,
    ) -> ResponseResult<ResponsePayload> {
        remotery_scope!("message_handler_local_client_send");

        let message_id: u32 = thread_rng().gen();
        debug!(
            self.log, "Created message identifier"; "message_id" => message_id);
        let body =
            RequestBody::new(payload, message_id, versioning::get_version());
        let request_data = to_internal_result(
            self.data_transformer.encode_request_body(body),
        )?;
        let response_data =
            self.local_client.send(&request_data).map_err(|_| {
                InternalError::public(
                    "Error on connecting to daemon - is it running?",
                    ApiErrorType::Configuration,
                )
            })?;
        let response_message = to_internal_result(
            self.data_transformer.decode_response_body(&response_data),
        )?;

        // Verify return message identifier
        if response_message.id != message_id {
            return Err(InternalError::private(ErrorKind::ResponseError(
                format!(
                    "Incorrect message ID in resposonse: expected {}, got {}",
                    message_id, response_message.id
                ),
            )));
        }

        api_to_internal_result(response_message.payload)
    }
}
