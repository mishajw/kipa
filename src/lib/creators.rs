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

cfg_if! {
    if #[cfg(feature = "use-protobuf")] {
        use data_transformer::protobuf::ProtobufDataTransformer;

        /// Create a `DataTransformer`
        pub fn create_data_transformer() -> Result<Arc<DataTransformer>> {
            Ok(Arc::new(ProtobufDataTransformer{}))
        }
    } else {
        #[allow(missing_docs)]
        pub fn create_data_transformer() -> Result<Arc<DataTransformer>> {
            Err(ErrorKind::ConfigError(
                "A data transformer feature was not selected".into()).into())
        }
    }
}

cfg_if! {
    if #[cfg(feature = "use-tcp")] {
        use server::tcp::{
            TcpGlobalServer, TcpGlobalClient};

        /// Create a `GlobalSendServer`
        pub fn create_global_client(
                data_transformer: Arc<DataTransformer>,
                local_node: Node,
                log: Logger) -> Result<Arc<Client>> {
            Ok(Arc::new(TcpGlobalClient::new(
                data_transformer,
                local_node,
                log)))
        }

        /// Create a `GlobalRecieveServer`
        pub fn create_global_server(
                message_handler: Arc<MessageHandler>,
                data_transformer: Arc<DataTransformer>,
                local_node: Node,
                log: Logger) -> Result<Arc<Mutex<Server>>> {
            Ok(Arc::new(Mutex::new(TcpGlobalServer::new(
                message_handler,
                data_transformer.clone(),
                local_node,
                log))))
        }
    } else {
        #[allow(missing_docs)]
        pub fn create_global_client(
            _data_transformer: Arc<DataTransformer>,
            _local_node: Node,
            _log: Logger
        ) -> Result<Arc<Client>> {
            Err(ErrorKind::ConfigError(
                "A server feature was not selected".into()).into())
        }

        #[allow(missing_docs)]
        pub fn create_global_server(
            _message_handler: Arc<MessageHandler>,
            _data_transformer: Arc<DataTransformer>,
            _local_node: Node,
            _log: Logger,
        ) -> Result<Arc<Mutex<Server>>> {
            Err(ErrorKind::ConfigError(
                "A server feature was not selected".into()).into())
        }
    }
}

cfg_if! {
    if #[cfg(feature = "use-unix-socket")] {
        use server::unix_socket::{
            UnixSocketLocalServer,
            UnixSocketLocalClient,
            DEFAULT_UNIX_SOCKET_PATH};

        /// Create a `LocalReceiveServer`
        pub fn create_local_server(
                message_handler: Arc<MessageHandler>,
                data_transformer: Arc<DataTransformer>,
                args: &clap::ArgMatches,
                log: Logger) -> Result<Arc<Mutex<Server>>> {
            let socket_path = args.value_of("socket_path")
                .unwrap_or(DEFAULT_UNIX_SOCKET_PATH);
            Ok(Arc::new(Mutex::new(UnixSocketLocalServer::new(
                message_handler,
                data_transformer,
                String::from(socket_path),
                log)?)))
        }

        /// Create a `LocalSendServer`
        pub fn create_local_client(
                data_transformer: Arc<DataTransformer>,
                args: &clap::ArgMatches,
                log: Logger) -> Result<Arc<LocalClient>> {
            let socket_path = args.value_of("socket_path")
                .unwrap_or(DEFAULT_UNIX_SOCKET_PATH);
            Ok(Arc::new(UnixSocketLocalClient::new(
                data_transformer,
                &String::from(socket_path),
                log)))
        }
    } else {
        #[allow(missing_docs)]
        pub fn create_local_server(
                _message_handler: Arc<MessageHandler>,
                _data_transformer: Arc<DataTransformer>,
                _args: &clap::ArgMatches,
                _log: Logger) -> Result<Arc<Mutex<Server>>> {
            Err(ErrorKind::ConfigError(
                "A local server feature was not selected".into()).into())
        }

        #[allow(missing_docs)]
        pub fn create_local_client(
                _data_transformer: Arc<DataTransformer>,
                _args: &clap::ArgMatches,
                _log: Logger) -> Result<Arc<LocalClient>> {
            Err(ErrorKind::ConfigError(
                "A local server feature was not selected".into()).into())
        }
    }
}

/// Create a `MessageHandler`.
pub fn create_message_handler(
    payload_handler: Arc<PayloadHandler>,
    local_node: Node,
) -> Arc<MessageHandler> {
    Arc::new(MessageHandler::new(payload_handler, local_node))
}

cfg_if! {
    if #[cfg(feature = "use-graph")] {
        use payload_handler::graph::{
            GraphPayloadHandler,
            DEFAULT_NEIGHBOURS_SIZE,
            DEFAULT_KEY_SPACE_SIZE};

        /// Create a `PayloadHandler`
        pub fn create_payload_handler(
                local_node: Node,
                client: Arc<Client>,
                args: &clap::ArgMatches,
                log: Logger) -> Result<Arc<PayloadHandler>> {

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
                client,
                neighbours_size,
                key_space_size,
                log)))
        }
    } else if #[cfg(feature = "use-black-hole")] {
        use payload_handler::black_hole::BlackHolePayloadHandler;

        #[allow(missing_docs)]
        pub fn create_payload_handler(
                _local_node: Node,
                _client: Arc<Client>,
                _args: &clap::ArgMatches,
                log: Logger) -> Result<Arc<PayloadHandler>> {
            Ok(Arc::new(BlackHolePayloadHandler::new(log)))
        }
    } else {
        #[allow(missing_docs)]
        pub fn create_payload_handler(
                _local_node: Node,
                _client: Arc<Client>,
                _args: &clap::ArgMatches,
                _log: Logger) -> Result<Arc<PayloadHandler>> {
            Err(ErrorKind::ConfigError(
                "A request handler feature was not selected".into()).into())
        }
    }
}
