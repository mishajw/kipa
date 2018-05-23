//! Implementation of `DataTransformer` using protobufs to serialize messages.
//!
//! Activated through the `use-protobuf` feature.
//!
//! Some relevant files:
//! 1) `.proto` file can be found in `resources/proto/proto_api.proto`.
//! 2) `build.rs` file creates the protobuf objects and places them in...
//! 3) `src/lib/data_handler/proto_api.rs` is where the generated protobuf
//! files are placed.

use address::Address;
use api::{
    MessageSender, RequestMessage, RequestPayload, ResponseMessage,
    ResponsePayload,
};
use data_transformer::{proto_api, DataTransformer};
use error::*;
use key::Key;
use node::Node;

use byteorder::{NetworkEndian, ReadBytesExt, WriteBytesExt};
use protobuf::*;
use std::convert::{From, Into};
use std::io::Cursor;

/// The protobuf data transformer type
pub struct ProtobufDataTransformer {}

impl ProtobufDataTransformer {
    /// Create a new protobuf data transformer
    pub fn new() -> Self { ProtobufDataTransformer {} }
}

impl DataTransformer for ProtobufDataTransformer {
    fn request_to_bytes(&self, request: &RequestMessage) -> Result<Vec<u8>> {
        let mut general_request = proto_api::GeneralRequest::new();
        match &request.payload {
            &RequestPayload::QueryRequest(ref key) => {
                let mut query = proto_api::QueryRequest::new();
                query.set_key(key.clone().into());
                general_request.set_query_request(query);
            }
            &RequestPayload::SearchRequest(ref key) => {
                let mut search = proto_api::SearchRequest::new();
                search.set_key(key.clone().into());
                general_request.set_search_request(search);
            }
            &RequestPayload::ConnectRequest(ref node) => {
                let mut connect = proto_api::ConnectRequest::new();
                let kipa_node: Result<proto_api::Node> = node.clone().into();
                connect.set_node(kipa_node?);
                general_request.set_connect_request(connect);
            }
            &RequestPayload::ListNeighboursRequest() => {
                let mut list = proto_api::ListNeighboursRequest::new();
                general_request.set_list_neighbours_request(list);
            }
        };

        let sender: Result<proto_api::MessageSender> =
            request.sender.clone().into();
        general_request.set_sender(sender?);
        general_request.set_id(request.id);

        general_request
            .write_to_bytes()
            .chain_err(|| "Error on write request to bytes")
    }

    fn bytes_to_request(
        &self,
        data: &Vec<u8>,
        sender: Option<Address>,
    ) -> Result<RequestMessage>
    {
        // Parse the request to the protobuf type
        let request: proto_api::GeneralRequest =
            parse_from_bytes(data).chain_err(|| "Error on parsing request")?;

        let payload = if request.has_query_request() {
            let key = request.get_query_request().get_key().clone().into();
            RequestPayload::QueryRequest(key)
        } else if request.has_search_request() {
            let key = request.get_search_request().get_key().clone().into();
            RequestPayload::SearchRequest(key)
        } else if request.has_connect_request() {
            let node: Result<Node> =
                request.get_connect_request().get_node().clone().into();
            RequestPayload::ConnectRequest(node?)
        } else if request.has_list_neighbours_request() {
            RequestPayload::ListNeighboursRequest()
        } else {
            return Err(
                ErrorKind::ParseError("Unrecognized request".into()).into()
            );
        };

        let sender: Result<MessageSender> =
            proto_to_message_sender(request.get_sender().clone(), sender);

        Ok(RequestMessage::new(payload, sender?, request.get_id()))
    }

    fn response_to_bytes(&self, response: &ResponseMessage) -> Result<Vec<u8>> {
        let mut general_response = proto_api::GeneralResponse::new();

        match &response.payload {
            &ResponsePayload::QueryResponse(ref nodes) => {
                let mut query = proto_api::QueryResponse::new();
                let kipa_nodes: Result<Vec<proto_api::Node>> =
                    nodes.iter().map(|n| n.clone().into()).collect();
                query.set_nodes(RepeatedField::from_vec(kipa_nodes?));
                general_response.set_query_response(query);
            }
            &ResponsePayload::SearchResponse(ref node) => {
                let mut search = proto_api::SearchResponse::new();
                match node {
                    &Some(ref node) => {
                        let n: Result<proto_api::Node> = node.clone().into();
                        search.set_node(n?);
                    }
                    &None => {}
                }
                general_response.set_search_response(search);
            }
            &ResponsePayload::ConnectResponse() => general_response
                .set_connect_response(proto_api::ConnectResponse::new()),
            &ResponsePayload::ListNeighboursResponse(ref nodes) => {
                let mut list = proto_api::ListNeighboursResponse::new();
                let kipa_nodes: Result<Vec<proto_api::Node>> =
                    nodes.iter().map(|n| n.clone().into()).collect();
                list.set_nodes(RepeatedField::from_vec(kipa_nodes?));
                general_response.set_list_neighbours_response(list);
            }
        };

        let sender: Result<proto_api::MessageSender> =
            response.sender.clone().into();
        general_response.set_sender(sender?);
        general_response.set_id(response.id);

        general_response
            .write_to_bytes()
            .chain_err(|| "Error on write response to bytes")
    }

    fn bytes_to_response(
        &self,
        data: &Vec<u8>,
        sender: Option<Address>,
    ) -> Result<ResponseMessage>
    {
        // Parse the request to the protobuf type
        let response: proto_api::GeneralResponse =
            parse_from_bytes(data).chain_err(|| "Error on parsing response")?;

        let payload = if response.has_query_response() {
            let nodes: Result<Vec<Node>> = response
                .get_query_response()
                .get_nodes()
                .iter()
                .map(|n| n.clone().into())
                .collect();
            ResponsePayload::QueryResponse(nodes?)
        } else if response.has_search_response() {
            if response.get_search_response().has_node() {
                let node: Result<Node> =
                    response.get_search_response().get_node().clone().into();
                ResponsePayload::SearchResponse(Some(node?))
            } else {
                ResponsePayload::SearchResponse(None)
            }
        } else if response.has_connect_response() {
            ResponsePayload::ConnectResponse()
        } else if response.has_list_neighbours_response() {
            let nodes: Result<Vec<Node>> = response
                .get_list_neighbours_response()
                .get_nodes()
                .iter()
                .map(|n| n.clone().into())
                .collect();
            ResponsePayload::ListNeighboursResponse(nodes?)
        } else {
            return Err(
                ErrorKind::ParseError("Unrecognized response".into()).into()
            );
        };

        let sender: Result<MessageSender> =
            proto_to_message_sender(response.get_sender().clone(), sender);

        Ok(ResponseMessage::new(payload, sender?, response.get_id()))
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

impl Into<Result<proto_api::Address>> for Address {
    fn into(self) -> Result<proto_api::Address> {
        let mut cursor = Cursor::new(self.ip_data.clone());
        let ipv4 = cursor
            .read_u32::<NetworkEndian>()
            .chain_err(|| "Error casting address IP data to IPv4")?;
        let mut kipa_address = proto_api::Address::new();
        kipa_address.set_ipv4(ipv4);
        kipa_address.set_port(self.port as u32);
        Ok(kipa_address)
    }
}

impl Into<Result<Address>> for proto_api::Address {
    fn into(self) -> Result<Address> {
        let mut ip_data = vec![];
        ip_data
            .write_u32::<NetworkEndian>(self.get_ipv4())
            .chain_err(|| "Error reading IP data to bytes")?;
        Ok(Address::new(ip_data, self.get_port() as u16))
    }
}

impl Into<Result<proto_api::Node>> for Node {
    fn into(self) -> Result<proto_api::Node> {
        let mut kipa_node = proto_api::Node::new();
        kipa_node.set_key(self.key.clone().into());
        let kipa_address: Result<proto_api::Address> =
            self.address.clone().into();
        kipa_node.set_address(kipa_address?);
        Ok(kipa_node)
    }
}

impl Into<Result<Node>> for proto_api::Node {
    fn into(self) -> Result<Node> {
        let address: Result<Address> = self.get_address().clone().into();
        Ok(Node::new(address?, self.get_key().clone().into()))
    }
}

impl Into<Result<proto_api::MessageSender>> for MessageSender {
    fn into(self) -> Result<proto_api::MessageSender> {
        let mut kipa_sender = proto_api::MessageSender::new();
        match self {
            MessageSender::Node(ref n) => {
                let key = n.key.clone().into();
                let port = n.address.port;
                kipa_sender.set_key(key);
                kipa_sender.set_port(u32::from(port));
            }
            MessageSender::Cli() => {}
        }
        Ok(kipa_sender)
    }
}

/// We can not define this function as a `Into` trait, as we also need the
/// `Address` to create the `MessageSender`
fn proto_to_message_sender(
    message_sender: proto_api::MessageSender,
    address: Option<Address>,
) -> Result<MessageSender>
{
    if address.is_some() {
        assert!(message_sender.has_key());
        assert!(
            message_sender.get_port() > 0 && message_sender.get_port() < 0xFFFF
        );
        let key = message_sender.get_key().clone().into();
        Ok(MessageSender::Node(Node::new(
            Address::new(
                address.unwrap().ip_data,
                message_sender.get_port() as u16,
            ),
            key,
        )))
    } else {
        Ok(MessageSender::Cli())
    }
}
