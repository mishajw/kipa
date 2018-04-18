//! Functons for creating various aspects of the kipa_lib.
//!
//! These depend on features and conditional compilation.

use data_transformer::DataTransformer;
use error::*;
#[allow(unused)]
use server::{Client, LocalClient, Server};
use request_handler::RequestHandler;
use node::Node;

#[allow(unused)]
use std::sync::{Arc, Mutex};
use clap;

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
                local_node: Node) -> Result<Arc<Client>> {
            Ok(Arc::new(TcpGlobalClient::new(data_transformer, local_node)))
        }

        /// Create a `GlobalRecieveServer`
        pub fn create_global_server(
                request_handler: Arc<RequestHandler>,
                data_transformer: Arc<DataTransformer>,
                local_node: Node) -> Result<Arc<Mutex<Server>>> {
            Ok(Arc::new(Mutex::new(TcpGlobalServer::new(
                request_handler, data_transformer.clone(), local_node))))
        }
    } else {
        #[allow(missing_docs)]
        pub fn create_global_client(
                _data_transformer: Arc<DataTransformer>, _local_node: Node) ->
                Result<Arc<Client>> {
            Err(ErrorKind::ConfigError(
                "A server feature was not selected".into()).into())
        }

        #[allow(missing_docs)]
        pub fn create_global_server(
            _request_handler: Arc<RequestHandler>,
            _data_transformer: Arc<DataTransformer>,
            _local_node: Node
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
                request_handler: Arc<RequestHandler>,
                data_transformer: Arc<DataTransformer>,
                args: &clap::ArgMatches) -> Result<Arc<Mutex<Server>>> {
            let socket_path = args.value_of("socket_path")
                .unwrap_or(DEFAULT_UNIX_SOCKET_PATH);
            Ok(Arc::new(Mutex::new(UnixSocketLocalServer::new(
                request_handler,
                data_transformer,
                String::from(socket_path))?)))
        }

        /// Create a `LocalSendServer`
        pub fn create_local_client(
                data_transformer: Arc<DataTransformer>,
                args: &clap::ArgMatches) -> Result<Arc<LocalClient>> {
            let socket_path = args.value_of("socket_path")
                .unwrap_or(DEFAULT_UNIX_SOCKET_PATH);
            Ok(Arc::new(UnixSocketLocalClient::new(
                data_transformer, &String::from(socket_path))))
        }
    } else {
        #[allow(missing_docs)]
        pub fn create_local_server(
                _request_handler: Arc<RequestHandler>,
                _data_transformer: Arc<DataTransformer>,
                _args: &clap::ArgMatches) -> Result<Arc<Mutex<Server>>> {
            Err(ErrorKind::ConfigError(
                "A local server feature was not selected".into()).into())
        }

        #[allow(missing_docs)]
        pub fn create_local_client(
                _data_transformer: Arc<DataTransformer>,
                _args: &clap::ArgMatches) -> Result<Arc<LocalClient>> {
            Err(ErrorKind::ConfigError(
                "A local server feature was not selected".into()).into())
        }
    }
}

cfg_if! {
    if #[cfg(feature = "use-graph")] {
        use request_handler::graph::{
            GraphRequestHandler,
            DEFAULT_NEIGHBOURS_SIZE,
            DEFAULT_KEY_SPACE_SIZE};

        /// Create a `RequestHandler`
        pub fn create_request_handler(
                local_node: Node,
                client: Arc<Client>,
                args: &clap::ArgMatches) -> Result<Arc<RequestHandler>> {

            let neighbours_size = args.value_of("neighbours_size")
                .unwrap_or(&DEFAULT_NEIGHBOURS_SIZE.to_string())
                .parse::<usize>()
                .chain_err(|| "Error on parsing neighbour size")?;

            let key_space_size = args.value_of("key_space_size")
                .unwrap_or(&DEFAULT_KEY_SPACE_SIZE.to_string())
                .parse::<usize>()
                .chain_err(|| "Error on parsing key space size")?;

            Ok(Arc::new(GraphRequestHandler::new(
                local_node.key,
                client,
                neighbours_size,
                key_space_size)))
        }
    } else if #[cfg(feature = "use-black-hole")] {
        use request_handler::black_hole::BlackHoleRequestHandler;

        pub fn create_request_handler(
                _local_node: Node,
                _client: Arc<Client>,
                _args: &clap::ArgMatches) -> Result<Arc<RequestHandler>> {
            Ok(Arc::new(BlackHoleRequestHandler::new()))
        }
    } else {
        #[allow(missing_docs)]
        pub fn create_request_handler(
                _local_node: Node,
                _client: Arc<Client>,
                _args: &clap::ArgMatches) -> Result<Arc<RequestHandler>> {
            Err(ErrorKind::ConfigError(
                "A request handler feature was not selected".into()).into())
        }
    }
}
