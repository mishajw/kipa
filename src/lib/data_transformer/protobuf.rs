//! Implementation of `DataTransformer` using protobufs to serialize messages
//!
//! Activated through the `use-protobuf` feature.
//!
//! Some relevant files:
//! 1) `.proto` file can be found in `resources/proto/proto_api.proto`.
//! 2) `build.rs` file creates the protobuf objects and places them in...
//! 3) `src/lib/data_transformer/proto_api.rs` is where the generated protobuf
//! files are placed.

use address::Address;
use api::{
    ApiError, ApiErrorType, ApiResult, RequestBody, RequestPayload,
    ResponseBody, ResponsePayload, SecureMessage,
};
use data_transformer::{proto_api, DataTransformer};
use error::*;
use key::Key;
use node::Node;

use protobuf::*;
use std::convert::{From, Into};

/// The protobuf data transformer type
#[derive(Default)]
pub struct ProtobufDataTransformer {}

impl DataTransformer for ProtobufDataTransformer {
    fn encode_secure_message(&self, message: SecureMessage) -> Result<Vec<u8>> {
        let mut proto_message = proto_api::SecureMessage::new();
        proto_message.set_sender(message.sender.clone().into());
        proto_message.set_body_signature(message.body_signature);
        proto_message.set_encrypted_body(message.encrypted_body);
        proto_message
            .write_to_bytes()
            .chain_err(|| "Error on write secure message to bytes")
    }

    fn decode_secure_message(
        &self,
        data: &[u8],
        sender: Address,
    ) -> Result<SecureMessage>
    {
        let proto_message: proto_api::SecureMessage = parse_from_bytes(data)
            .chain_err(|| "Error on parsing secure message")?;
        Ok(SecureMessage::new(
            sender_node_to_node(proto_message.get_sender(), sender),
            proto_message.get_body_signature().to_vec(),
            proto_message.get_encrypted_body().to_vec(),
        ))
    }

    fn encode_request_body(&self, body: RequestBody) -> Result<Vec<u8>> {
        let proto_body: proto_api::RequestBody = body.into();
        proto_body
            .write_to_bytes()
            .chain_err(|| "Error on write request body to bytes")
    }

    fn decode_request_body(&self, data: &[u8]) -> Result<RequestBody> {
        let proto_body: proto_api::RequestBody = parse_from_bytes(data)
            .chain_err(|| "Error on parsing request body")?;
        proto_body.into()
    }

    fn encode_response_body(&self, body: ResponseBody) -> Result<Vec<u8>> {
        let proto_body: proto_api::ResponseBody = body.into();
        proto_body
            .write_to_bytes()
            .chain_err(|| "Error on write response body to bytes")
    }

    fn decode_response_body(&self, data: &[u8]) -> Result<ResponseBody> {
        let proto_body: proto_api::ResponseBody = parse_from_bytes(data)
            .chain_err(|| "Error on parsing response body")?;
        proto_body.into()
    }
}

// TODO: Try to remove clones from the `Into<>` impls

impl Into<proto_api::RequestBody> for RequestBody {
    fn into(self) -> proto_api::RequestBody {
        let mut proto_body = proto_api::RequestBody::new();

        match self.payload {
            RequestPayload::QueryRequest(ref key) => {
                let mut query = proto_api::QueryRequest::new();
                query.set_key(key.clone().into());
                proto_body.set_query_request(query);
            }
            RequestPayload::SearchRequest(ref key) => {
                let mut search = proto_api::SearchRequest::new();
                search.set_key(key.clone().into());
                proto_body.set_search_request(search);
            }
            RequestPayload::ConnectRequest(ref node) => {
                let mut connect = proto_api::ConnectRequest::new();
                connect.set_node(node.clone().into());
                proto_body.set_connect_request(connect);
            }
            RequestPayload::ListNeighboursRequest() => {
                let mut list = proto_api::ListNeighboursRequest::new();
                proto_body.set_list_neighbours_request(list);
            }
        };

        proto_body.set_id(self.id);
        proto_body.set_version(self.version);
        proto_body
    }
}

impl Into<Result<RequestBody>> for proto_api::RequestBody {
    fn into(self) -> Result<RequestBody> {
        let payload = if self.has_query_request() {
            let key = self.get_query_request().get_key().clone().into();
            RequestPayload::QueryRequest(key)
        } else if self.has_search_request() {
            let key = self.get_search_request().get_key().clone().into();
            RequestPayload::SearchRequest(key)
        } else if self.has_connect_request() {
            RequestPayload::ConnectRequest(
                self.get_connect_request().get_node().clone().into(),
            )
        } else if self.has_list_neighbours_request() {
            RequestPayload::ListNeighboursRequest()
        } else {
            return Err(
                ErrorKind::RequestError("Unrecognized request".into()).into()
            );
        };

        Ok(RequestBody::new(
            payload,
            self.get_id(),
            self.get_version().into(),
        ))
    }
}

impl Into<proto_api::ResponseBody> for ResponseBody {
    fn into(self) -> proto_api::ResponseBody {
        let mut proto_body = proto_api::ResponseBody::new();

        match self.payload {
            Ok(ResponsePayload::QueryResponse(ref nodes)) => {
                let mut query = proto_api::QueryResponse::new();
                query.set_nodes(RepeatedField::from_vec(
                    nodes.iter().map(|n| n.clone().into()).collect(),
                ));
                proto_body.set_query_response(query);
            }
            Ok(ResponsePayload::SearchResponse(ref node)) => {
                let mut search = proto_api::SearchResponse::new();
                if let Some(node) = node {
                    search.set_node(node.clone().into());
                }
                proto_body.set_search_response(search);
            }
            Ok(ResponsePayload::ConnectResponse()) => proto_body
                .set_connect_response(proto_api::ConnectResponse::new()),
            Ok(ResponsePayload::ListNeighboursResponse(ref nodes)) => {
                let mut list = proto_api::ListNeighboursResponse::new();
                let kipa_nodes: Vec<proto_api::Node> =
                    nodes.iter().map(|n| n.clone().into()).collect();
                list.set_nodes(RepeatedField::from_vec(kipa_nodes));
                proto_body.set_list_neighbours_response(list);
            }
            Err(api_error) => {
                let proto_error = api_error.clone().into();
                proto_body.set_api_error(proto_error);
            }
        };

        proto_body.set_id(self.id);
        proto_body.set_version(self.version.clone());
        proto_body
    }
}

impl Into<Result<ResponseBody>> for proto_api::ResponseBody {
    fn into(self) -> Result<ResponseBody> {
        let payload: ApiResult<ResponsePayload> = if self.has_query_response() {
            let nodes: Vec<Node> = self
                .get_query_response()
                .get_nodes()
                .iter()
                .map(|n| n.clone().into())
                .collect();
            Ok(ResponsePayload::QueryResponse(nodes))
        } else if self.has_search_response() {
            if self.get_search_response().has_node() {
                let node: Node =
                    self.get_search_response().get_node().clone().into();
                Ok(ResponsePayload::SearchResponse(Some(node)))
            } else {
                Ok(ResponsePayload::SearchResponse(None))
            }
        } else if self.has_connect_response() {
            Ok(ResponsePayload::ConnectResponse())
        } else if self.has_list_neighbours_response() {
            let nodes: Vec<Node> = self
                .get_list_neighbours_response()
                .get_nodes()
                .iter()
                .map(|n| n.clone().into())
                .collect();
            Ok(ResponsePayload::ListNeighboursResponse(nodes))
        } else if self.has_api_error() {
            Err(self.get_api_error().clone().into())
        } else {
            // This return is scoped to the function, not to the payload
            return Err(
                ErrorKind::ParseError("Unrecognized response".into()).into()
            );
        };

        Ok(ResponseBody::new(
            payload,
            self.get_id(),
            self.get_version().into(),
        ))
    }
}

impl Into<proto_api::Key> for Key {
    fn into(self) -> proto_api::Key {
        let mut kipa_key = proto_api::Key::new();
        kipa_key.set_key_id(self.get_key_id().clone());
        kipa_key.set_data(self.get_data().clone());
        kipa_key
    }
}

impl From<proto_api::Key> for Key {
    fn from(kipa_key: proto_api::Key) -> Key {
        Key::new(kipa_key.get_key_id().into(), kipa_key.data.clone())
    }
}

impl Into<proto_api::Address> for Address {
    fn into(self) -> proto_api::Address {
        let mut kipa_address = proto_api::Address::new();
        kipa_address.set_ip_data(self.ip_data);
        kipa_address.set_port(u32::from(self.port));
        kipa_address
    }
}

impl Into<Address> for proto_api::Address {
    fn into(self) -> Address {
        assert!(self.get_port() > 0 && self.get_port() < 0xFFFF);
        Address::new(self.get_ip_data().to_vec(), self.get_port() as u16)
    }
}

impl Into<proto_api::Node> for Node {
    fn into(self) -> proto_api::Node {
        let mut kipa_node = proto_api::Node::new();
        kipa_node.set_key(self.key.clone().into());
        kipa_node.set_address(self.address.clone().into());
        kipa_node
    }
}

impl Into<Node> for proto_api::Node {
    fn into(self) -> Node {
        Node::new(
            self.get_address().clone().into(),
            self.get_key().clone().into(),
        )
    }
}

impl Into<proto_api::SenderNode> for Node {
    fn into(self) -> proto_api::SenderNode {
        let mut proto_node = proto_api::SenderNode::new();
        proto_node.set_key(self.key.into());
        proto_node.set_port(u32::from(self.address.port));
        proto_node
    }
}

/// We can not define this function as a `Into` trait, as we also need the
/// `Address` to create the `SenderNode`
fn sender_node_to_node(
    sender_node: &proto_api::SenderNode,
    address: Address,
) -> Node
{
    assert!(sender_node.has_key());
    assert!(sender_node.get_port() > 0 && sender_node.get_port() < 0xFFFF);
    let key = sender_node.get_key().clone().into();
    Node::new(
        Address::new(address.ip_data, sender_node.get_port() as u16),
        key,
    )
}

impl Into<ApiErrorType> for proto_api::ApiErrorType {
    fn into(self) -> ApiErrorType {
        match self {
            proto_api::ApiErrorType::NoError => ApiErrorType::NoError,
            proto_api::ApiErrorType::Parse => ApiErrorType::Parse,
            proto_api::ApiErrorType::Configuration => {
                ApiErrorType::Configuration
            }
            proto_api::ApiErrorType::Internal => ApiErrorType::Internal,
            proto_api::ApiErrorType::External => ApiErrorType::External,
        }
    }
}

impl Into<proto_api::ApiErrorType> for ApiErrorType {
    fn into(self) -> proto_api::ApiErrorType {
        match self {
            ApiErrorType::NoError => proto_api::ApiErrorType::NoError,
            ApiErrorType::Parse => proto_api::ApiErrorType::Parse,
            ApiErrorType::Configuration => {
                proto_api::ApiErrorType::Configuration
            }
            ApiErrorType::Internal => proto_api::ApiErrorType::Internal,
            ApiErrorType::External => proto_api::ApiErrorType::External,
        }
    }
}

impl Into<ApiError> for proto_api::ApiError {
    fn into(self) -> ApiError {
        ApiError::new(self.get_msg().to_string(), self.get_error_type().into())
    }
}

impl Into<proto_api::ApiError> for ApiError {
    fn into(self) -> proto_api::ApiError {
        let mut api_error = proto_api::ApiError::new();
        api_error.set_msg(self.message);
        api_error
    }
}
