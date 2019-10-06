//! Handle messages sent to a daemon.

// TODO: Ensure we handle errors correctly in this file.

use api::error::ApiErrorType;
use api::request::{Request, Response};
use api::{Address, Node, SecretKey};
use api::{ApiVisibility, RequestBody, RequestPayload, ResponseBody, ResponsePayload};
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

/// Server that receives requests from daemons and CLIs.
pub struct MessageHandlerServer {
    payload_handler: Arc<dyn PayloadHandler>,
    local_secret_key: SecretKey,
    data_transformer: Arc<dyn DataTransformer>,
    pgp_key_handler: Arc<PgpKeyHandler>,
    log: Logger,
}

impl MessageHandlerServer {
    #[allow(missing_docs)]
    pub fn new(
        payload_handler: Arc<dyn PayloadHandler>,
        local_secret_key: SecretKey,
        data_transformer: Arc<dyn DataTransformer>,
        pgp_key_handler: Arc<PgpKeyHandler>,
        log: Logger,
    ) -> Self {
        MessageHandlerServer {
            payload_handler,
            local_secret_key,
            data_transformer,
            pgp_key_handler,
            log,
        }
    }

    /// Receive and handle the bytes of a request message, returning the bytes of response message.
    ///
    /// This function, and all the other `receive`, functions return a `Result` and not an
    /// `InternalResult` because the issues with communication are usually internal errors. Public
    /// errors can occur when computing the result of a query, but this is captured in the payload
    /// `ApiError<_>` value in the encoded bytes.
    pub fn receive_bytes(&self, request_data: &[u8], address: Option<Address>) -> Result<Vec<u8>> {
        remotery_scope!("message_handler_receive_bytes");

        debug!(self.log, "Received bytes"; "from_cli" => address.is_none());

        match address {
            Some(address) => {
                let message = self
                    .data_transformer
                    .decode_request_message(request_data, address)?;
                let response_message = self.receive_request(message)?;
                Ok(self
                    .data_transformer
                    .encode_response_message(response_message)?)
            }
            None => {
                let body = self.data_transformer.decode_request_body(request_data)?;
                let response_body = self.receive_body(body, None)?;
                Ok(self.data_transformer.encode_response_body(response_body)?)
            }
        }
    }

    fn receive_request(&self, request: Request) -> Result<Response> {
        remotery_scope!("message_handler_receive_request");

        debug!(self.log, "Received secure message");

        let decrypted_body_data = self.pgp_key_handler.decrypt_and_sign(
            &request.encrypted_body,
            &request.sender.key,
            &self.local_secret_key,
        )?;
        let body = self
            .data_transformer
            .decode_request_body(&decrypted_body_data)?;
        let response_body = self.receive_body(body, Some(request.sender.clone()))?;
        let response_body_data = self.data_transformer.encode_response_body(response_body)?;
        let encrypted_response_body_data = self.pgp_key_handler.encrypt_and_sign(
            &response_body_data,
            &self.local_secret_key,
            &request.sender.key,
        )?;
        Ok(Response::new(encrypted_response_body_data))
    }

    /// Receive and handle a request message, returning a response message.
    fn receive_body(&self, body: RequestBody, sender: Option<Node>) -> Result<ResponseBody> {
        remotery_scope!("message_handler_receive_body");

        debug!(self.log, "Received request body");

        // Check the visibility of the request is correct
        let visibility = match sender {
            Some(_) => ApiVisibility::Global(),
            None => ApiVisibility::Local(),
        };
        if !body.payload.is_visible(&visibility) {
            return Err(ErrorKind::RequestError("Request is not locally available".into()).into());
        }

        let version_verification_result = api_to_internal_result(versioning::verify_version(
            &versioning::get_version(),
            &body.version,
        ));
        let response_payload_result = version_verification_result
            .and_then(|()| self.payload_handler.receive(&body.payload, sender, body.id));

        Ok(ResponseBody::new(
            to_api_result(response_payload_result, &self.log),
            body.id,
            versioning::get_version(),
        ))
    }
}

/// Client that sends requests to daemons from another daemon.
pub struct MessageHandlerClient {
    local_node: Node,
    local_secret_key: SecretKey,
    client: Arc<dyn Client>,
    data_transformer: Arc<dyn DataTransformer>,
    pgp_key_handler: Arc<PgpKeyHandler>,
    log: Logger,
}

impl MessageHandlerClient {
    #[allow(missing_docs)]
    pub fn new(
        local_node: Node,
        local_secret_key: SecretKey,
        client: Arc<dyn Client>,
        data_transformer: Arc<dyn DataTransformer>,
        pgp_key_handler: Arc<PgpKeyHandler>,
        log: Logger,
    ) -> MessageHandlerClient {
        MessageHandlerClient {
            local_node,
            local_secret_key,
            client,
            data_transformer,
            pgp_key_handler,
            log,
        }
    }

    /// Send a payload to a node.
    pub fn send_request(
        &self,
        node: &Node,
        payload: RequestPayload,
        timeout: Duration,
    ) -> InternalResult<ResponsePayload> {
        remotery_scope!("message_handler_client_send_request");

        let message_id: u32 = thread_rng().gen();
        debug!(
            self.log, "Sending private request"; "message_id" => message_id);
        let body = RequestBody::new(payload, message_id, versioning::get_version());

        let body_data = to_internal_result(self.data_transformer.encode_request_body(body))?;
        let encrypted_body_data = to_internal_result(self.pgp_key_handler.encrypt_and_sign(
            &body_data,
            &self.local_secret_key,
            &node.key,
        ))?;

        let message = Request::new(self.local_node.clone(), encrypted_body_data);
        let message_data =
            to_internal_result(self.data_transformer.encode_request_message(message))?;

        let response_message_data =
            to_internal_result(self.client.send(node, &message_data, timeout))?;
        let response_message = to_internal_result(
            self.data_transformer
                .decode_response_message(&response_message_data, node.address.clone()),
        )?;

        let response_body_data = self
            .pgp_key_handler
            .decrypt_and_sign(
                &response_message.encrypted_body,
                &node.key,
                &self.local_secret_key,
            )
            .map_err(|err| {
                InternalError::public_with_error(
                    "Failed to decrypt message",
                    ApiErrorType::Parse,
                    err,
                )
            })?;

        let response_body = to_internal_result(
            self.data_transformer
                .decode_response_body(&response_body_data),
        )?;

        if response_body.id != message_id {
            return Err(InternalError::private(ErrorKind::ResponseError(format!(
                "Response had incorrect ID, expected {}, received {}",
                message_id, response_body.id
            ))));
        }

        api_to_internal_result(response_body.payload)
    }
}

/// Client that sends requests to daemons from a CLI.
pub struct MessageHandlerLocalClient {
    local_client: Arc<dyn LocalClient>,
    data_transformer: Arc<dyn DataTransformer>,
    log: Logger,
}

impl MessageHandlerLocalClient {
    #[allow(missing_docs)]
    pub fn new(
        local_client: Arc<dyn LocalClient>,
        data_transformer: Arc<dyn DataTransformer>,
        log: Logger,
    ) -> Self {
        MessageHandlerLocalClient {
            local_client,
            data_transformer,
            log,
        }
    }

    /// Send a payload to a node.
    pub fn send(&self, payload: RequestPayload) -> InternalResult<ResponsePayload> {
        remotery_scope!("message_handler_local_client_send");

        let message_id: u32 = thread_rng().gen();
        debug!(
            self.log, "Created message identifier"; "message_id" => message_id);
        let body = RequestBody::new(payload, message_id, versioning::get_version());
        let request_data = to_internal_result(self.data_transformer.encode_request_body(body))?;
        let response_data = self.local_client.send(&request_data).map_err(|_| {
            InternalError::public(
                "Error on connecting to daemon - is it running?",
                ApiErrorType::Configuration,
            )
        })?;
        let response_message =
            to_internal_result(self.data_transformer.decode_response_body(&response_data))?;

        // Verify return message identifier
        if response_message.id != message_id {
            return Err(InternalError::private(ErrorKind::ResponseError(format!(
                "Incorrect message ID in response: expected {}, got {}",
                message_id, response_message.id
            ))));
        }

        api_to_internal_result(response_message.payload)
    }
}
