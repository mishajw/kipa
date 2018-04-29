//! Functons for creating various aspects of the kipa_lib.
//!
//! These depend on features and conditional compilation.

use data_transformer::DataTransformer;
use error::*;
#[allow(unused)]
use server::{Client, LocalClient, Server};
use payload_handler::PayloadHandler;
use message_handler::MessageHandler;
use node::Node;

#[allow(unused)]
use std::sync::{Arc, Mutex};
use clap;
use slog;
use slog::Logger;
use slog_term;
use slog_async;
use slog_json;
use slog::Drain;
use std::fs;

/// Create the root logger for the project
pub fn create_logger(name: &'static str) -> Logger {
    let log_file = fs::File::create(format!("log-{}.json", name))
        .expect("Error on creating log file");

    let decorator = slog_term::TermDecorator::new().build();
    let json_drain = slog_json::Json::new(log_file).add_default_keys().build();
    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    let drain = slog::Duplicate(json_drain, drain).fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(
        Arc::new(drain),
        o!("name" => name, "version" => "0.1.0"),
    )
}

#[cfg(use_protobuf)]
/// Create a `DataTransformer`
pub fn create_data_transformer() -> Result<Arc<DataTransformer>> {
    use data_transformer::protobuf::ProtobufDataTransformer;
    Ok(Arc::new(ProtobufDataTransformer {}))
}

#[cfg(no_data_handler)]
#[allow(missing_docs)]
pub fn create_data_transformer() -> Result<Arc<DataTransformer>> {
    Err(ErrorKind::ConfigError(
        "A data transformer feature was not selected".into(),
    ).into())
}

#[cfg(use_tcp)]
/// Create a `GlobalSendServer`
pub fn create_global_client(
    data_transformer: Arc<DataTransformer>,
    log: Logger,
) -> Result<Arc<Client>> {
    use server::tcp::TcpGlobalClient;
    Ok(Arc::new(TcpGlobalClient::new(data_transformer, log)))
}

#[cfg(use_tcp)]
/// Create a `GlobalRecieveServer`
pub fn create_global_server(
    message_handler: Arc<MessageHandler>,
    data_transformer: Arc<DataTransformer>,
    local_node: Node,
    log: Logger,
) -> Result<Arc<Mutex<Server>>> {
    use server::tcp::TcpGlobalServer;
    Ok(Arc::new(Mutex::new(TcpGlobalServer::new(
        message_handler,
        data_transformer.clone(),
        local_node,
        log,
    ))))
}

#[cfg(no_global_server)]
#[allow(missing_docs)]
pub fn create_global_client(
    _data_transformer: Arc<DataTransformer>,
    _log: Logger,
) -> Result<Arc<Client>> {
    Err(
        ErrorKind::ConfigError("A server feature was not selected".into())
            .into(),
    )
}

#[cfg(no_global_server)]
#[allow(missing_docs)]
pub fn create_global_server(
    _message_handler: Arc<MessageHandler>,
    _data_transformer: Arc<DataTransformer>,
    _local_node: Node,
    _log: Logger,
) -> Result<Arc<Mutex<Server>>> {
    Err(
        ErrorKind::ConfigError("A server feature was not selected".into())
            .into(),
    )
}

#[cfg(use_unix_socket)]
/// Create a `LocalReceiveServer`
pub fn create_local_server(
    message_handler: Arc<MessageHandler>,
    data_transformer: Arc<DataTransformer>,
    args: &clap::ArgMatches,
    log: Logger,
) -> Result<Arc<Mutex<Server>>> {
    use server::unix_socket::{UnixSocketLocalServer, DEFAULT_UNIX_SOCKET_PATH};
    let socket_path = args.value_of("socket_path")
        .unwrap_or(DEFAULT_UNIX_SOCKET_PATH);
    Ok(Arc::new(Mutex::new(UnixSocketLocalServer::new(
        message_handler,
        data_transformer,
        String::from(socket_path),
        log,
    )?)))
}

#[cfg(use_unix_socket)]
/// Create a `LocalSendServer`
pub fn create_local_client(
    data_transformer: Arc<DataTransformer>,
    args: &clap::ArgMatches,
    log: Logger,
) -> Result<Arc<LocalClient>> {
    use server::unix_socket::{UnixSocketLocalClient, DEFAULT_UNIX_SOCKET_PATH};

    let socket_path = args.value_of("socket_path")
        .unwrap_or(DEFAULT_UNIX_SOCKET_PATH);
    Ok(Arc::new(UnixSocketLocalClient::new(
        data_transformer,
        &String::from(socket_path),
        log,
    )))
}

#[cfg(no_local_server)]
#[allow(missing_docs)]
pub fn create_local_server(
    _message_handler: Arc<MessageHandler>,
    _data_transformer: Arc<DataTransformer>,
    _args: &clap::ArgMatches,
    _log: Logger,
) -> Result<Arc<Mutex<Server>>> {
    Err(ErrorKind::ConfigError(
        "A local server feature was not selected".into(),
    ).into())
}

#[cfg(no_local_server)]
#[allow(missing_docs)]
pub fn create_local_client(
    _data_transformer: Arc<DataTransformer>,
    _args: &clap::ArgMatches,
    _log: Logger,
) -> Result<Arc<LocalClient>> {
    Err(ErrorKind::ConfigError(
        "A local server feature was not selected".into(),
    ).into())
}

/// Create a `MessageHandler`.
pub fn create_message_handler(
    payload_handler: Arc<PayloadHandler>,
    client: Arc<Client>,
    local_node: Node,
) -> Arc<MessageHandler> {
    Arc::new(MessageHandler::new(payload_handler, local_node, client))
}

#[cfg(use_graph)]
/// Create a `PayloadHandler`
pub fn create_payload_handler(
    local_node: Node,
    args: &clap::ArgMatches,
    log: Logger,
) -> Result<Arc<PayloadHandler>> {
    use payload_handler::graph::{GraphPayloadHandler, DEFAULT_KEY_SPACE_SIZE,
                                 DEFAULT_NEIGHBOURS_SIZE};

    let neighbours_size = args.value_of("neighbours_size")
        .unwrap_or(&DEFAULT_NEIGHBOURS_SIZE.to_string())
        .parse::<usize>()
        .chain_err(|| "Error on parsing neighbour size")?;

    let key_space_size = args.value_of("key_space_size")
        .unwrap_or(&DEFAULT_KEY_SPACE_SIZE.to_string())
        .parse::<usize>()
        .chain_err(|| "Error on parsing key space size")?;

    Ok(Arc::new(GraphPayloadHandler::new(
        local_node.key,
        neighbours_size,
        key_space_size,
        log,
    )))
}

#[cfg(use_black_hole)]
#[allow(missing_docs)]
pub fn create_payload_handler(
    _local_node: Node,
    _args: &clap::ArgMatches,
    log: Logger,
) -> Result<Arc<PayloadHandler>> {
    use payload_handler::black_hole::BlackHolePayloadHandler;
    Ok(Arc::new(BlackHolePayloadHandler::new(log)))
}

#[cfg(no_payload_handler)]
#[allow(missing_docs)]
pub fn create_payload_handler(
    _local_node: Node,
    _args: &clap::ArgMatches,
    _log: Logger,
) -> Result<Arc<PayloadHandler>> {
    Err(ErrorKind::ConfigError(
        "A request handler feature was not selected".into(),
    ).into())
}
