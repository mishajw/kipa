use serde::Serialize;
use serde_json;

use api::{Key, Node, RequestPayload};
use slog::Logger;

/// Logs KIPA event as JSON.
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum LogEvent {
    ReceiveRequest { payload: RequestPayload },
    QueryFailed { node: Node },
    QuerySucceeded { node: Node, neighbours: Vec<Node> },
    SearchFound { node: Node },
    SearchExplored { node: Node },
    SearchError { key: Key },
    SearchNotFound { key: Key },
    SearchVerificationFailed { node: Node },
    SearchSucceeded { node: Node },
}

impl LogEvent {
    /// Logs that a request was received.
    pub fn receive_request(payload: &RequestPayload, log: &Logger) {
        LogEvent::ReceiveRequest {
            payload: payload.clone(),
        }
        .log(log)
    }

    /// Logs that a neighbour query failed.
    pub fn query_failed(node: &Node, log: &Logger) {
        LogEvent::QueryFailed { node: node.clone() }.log(log)
    }

    /// Logs that a neighbour query succeeded.
    pub fn query_succeeded(node: &Node, neighbours: &[Node], log: &Logger) {
        LogEvent::QuerySucceeded {
            node: node.clone(),
            neighbours: neighbours.to_vec(),
        }
        .log(log)
    }

    /// Logs that search found a node.
    pub fn search_found(node: &Node, log: &Logger) {
        LogEvent::SearchFound { node: node.clone() }.log(log)
    }

    /// Logs that search explored a node.
    pub fn search_explored(node: &Node, log: &Logger) {
        LogEvent::SearchExplored { node: node.clone() }.log(log)
    }

    /// Logs that search had an error.
    pub fn search_error(key: &Key, log: &Logger) {
        LogEvent::SearchError { key: key.clone() }.log(log)
    }

    /// Logs that search did not find the node.
    pub fn search_not_found(key: &Key, log: &Logger) {
        LogEvent::SearchNotFound { key: key.clone() }.log(log)
    }

    /// Logs that verifying after a search failed.
    pub fn search_verification_failed(node: &Node, log: &Logger) {
        LogEvent::SearchVerificationFailed { node: node.clone() }.log(log)
    }

    /// Logs that search succeeded.
    pub fn search_succeeded(node: &Node, log: &Logger) {
        LogEvent::SearchSucceeded { node: node.clone() }.log(log)
    }

    fn log(self, log: &Logger) {
        info!(log, "log_event"; "log_event" => serde_json::to_string(&self).unwrap())
    }
}
