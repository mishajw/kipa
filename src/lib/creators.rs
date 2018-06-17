//! Functons for creating various aspects of the kipa_lib.
//!
//! These depend on features and conditional compilation.

use data_transformer::DataTransformer;
use error::*;
use message_handler::MessageHandler;
use node::Node;
use payload_handler::PayloadHandler;
#[allow(unused)]
use server::{Client, LocalClient, LocalServer, Server};

use clap;
use slog;
use slog::Drain;
use slog::Logger;
use slog_async;
use slog_json;
use slog_term;
use std::fs;
#[allow(unused)]
use std::sync::{Arc, Mutex};

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

/// Implementors can be constructed from `clap` arguments
pub trait Creator {
    /// Parameters needed for creating the type
    type Parameters;

    /// Add `clap` arguments to the command line options
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> { vec![] }

    /// Create the type, given `clap` arguments and parameters
    fn create(
        _parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        _log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        Err(InternalError::public(
            "Unselected feature",
            ApiErrorType::Configuration,
        ))
    }
}

impl Creator for DataTransformer {
    type Parameters = ();
    #[cfg(feature = "use-protobuf")]
    fn create(
        _parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        _log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use data_transformer::protobuf::ProtobufDataTransformer;
        Ok(Box::new(ProtobufDataTransformer {}))
    }
}

impl Creator for Client {
    type Parameters = Arc<DataTransformer>;
    #[cfg(feature = "use-tcp")]
    fn create(
        data_transformer: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use server::tcp::TcpGlobalClient;
        Ok(Box::new(TcpGlobalClient::new(data_transformer, log)))
    }
}

impl Creator for Server {
    type Parameters = (Arc<MessageHandler>, Arc<DataTransformer>, Node);
    #[cfg(feature = "use-tcp")]
    fn create(
        parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use server::tcp::TcpGlobalServer;
        let (message_handler, data_transformer, local_node) = parameters;
        Ok(Box::new(TcpGlobalServer::new(
            message_handler,
            data_transformer.clone(),
            local_node,
            log,
        )))
    }
}

impl Creator for LocalServer {
    type Parameters = (Arc<MessageHandler>, Arc<DataTransformer>);

    #[cfg(feature = "use-unix-socket")]
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use server::unix_socket::DEFAULT_UNIX_SOCKET_PATH;
        vec![
            clap::Arg::with_name("socket_path")
                .long("socket-path")
                .short("s")
                .help("Socket to listen for local queries from CLI from")
                .takes_value(true)
                .default_value(DEFAULT_UNIX_SOCKET_PATH),
        ]
    }

    #[cfg(feature = "use-unix-socket")]
    fn create(
        parameters: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use server::unix_socket::UnixSocketLocalServer;
        let (message_handler, data_transformer) = parameters;
        let socket_path = args.value_of("socket_path").unwrap();
        let server = to_internal_result(UnixSocketLocalServer::new(
            message_handler,
            data_transformer,
            String::from(socket_path),
            log,
        ))?;
        Ok(Box::new(server))
    }
}

impl Creator for LocalClient {
    type Parameters = Arc<DataTransformer>;
    // Shares `socket_path` parameter with `LocalServer`
    #[cfg(feature = "use-unix-socket")]
    fn create(
        data_transformer: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use server::unix_socket::UnixSocketLocalClient;
        let socket_path = args.value_of("socket_path").unwrap();
        Ok(Box::new(UnixSocketLocalClient::new(
            data_transformer,
            &String::from(socket_path),
            log,
        )))
    }
}

impl Creator for MessageHandler {
    type Parameters = (Arc<PayloadHandler>, Node, Arc<Client>);
    fn create(
        parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        _log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        let (payload_handler, local_node, client) = parameters;
        Ok(Box::new(MessageHandler::new(
            payload_handler,
            local_node,
            client,
        )))
    }
}

#[cfg(feature = "use-graph")]
use payload_handler::graph::KeySpaceManager;
#[cfg(feature = "use-graph")]
impl Creator for KeySpaceManager {
    type Parameters = Node;
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use payload_handler::graph::DEFAULT_KEY_SPACE_SIZE;
        vec![
            clap::Arg::with_name("key_space_size")
                .long("key-space-size")
                .help("Number of dimensions to use for key space")
                .takes_value(true)
                .default_value(DEFAULT_KEY_SPACE_SIZE),
        ]
    }

    fn create(
        _parameters: Self::Parameters,
        args: &clap::ArgMatches,
        _log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        let key_space_size = args
            .value_of("key_space_size")
            .unwrap()
            .parse::<usize>()
            .map_err(|_| {
                InternalError::public(
                    "Error on parsing key space size",
                    ApiErrorType::Parse,
                )
            })?;
        Ok(Box::new(KeySpaceManager::new(key_space_size)))
    }
}

#[cfg(feature = "use-graph")]
use payload_handler::graph::NeighboursStore;
#[cfg(feature = "use-graph")]
impl Creator for NeighboursStore {
    type Parameters = (::key::Key, Arc<KeySpaceManager>);
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use payload_handler::graph::{
            DEFAULT_ANGLE_WEIGHTING, DEFAULT_DISTANCE_WEIGHTING,
            DEFAULT_MAX_NUM_NEIGHBOURS,
        };
        vec![
            clap::Arg::with_name("neighbours_size")
                .long("neighbours-size")
                .help("Maximum number of neighbours to store")
                .takes_value(true)
                .default_value(DEFAULT_MAX_NUM_NEIGHBOURS),
            clap::Arg::with_name("distance_weighting")
                .long("distance-weighting")
                .help("Weight of the distance when considering neighbours")
                .takes_value(true)
                .default_value(DEFAULT_DISTANCE_WEIGHTING),
            clap::Arg::with_name("angle_weighting")
                .long("angle-weighting")
                .help("Weight of the angle when considering neighbours")
                .takes_value(true)
                .default_value(DEFAULT_ANGLE_WEIGHTING),
        ]
    }

    fn create(
        parameters: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        let (local_key, key_space_manager) = parameters;

        let neighbours_size = args
            .value_of("neighbours_size")
            .unwrap()
            .parse::<usize>()
            .map_err(|_| {
                InternalError::public(
                    "Error on parsing neighbour size",
                    ApiErrorType::Parse,
                )
            })?;

        let distance_weighting = args
            .value_of("distance_weighting")
            .unwrap()
            .parse::<f32>()
            .map_err(|_| {
                InternalError::public(
                    "Error on parsing distance weighting",
                    ApiErrorType::Parse,
                )
            })?;

        let angle_weighting = args
            .value_of("angle_weighting")
            .unwrap()
            .parse::<f32>()
            .map_err(|_| {
                InternalError::public(
                    "Error on parsing angle weighting",
                    ApiErrorType::Parse,
                )
            })?;

        Ok(Box::new(NeighboursStore::new(
            &local_key,
            neighbours_size,
            distance_weighting,
            angle_weighting,
            key_space_manager,
            log.new(o!("neighbours_store" => true)),
        )))
    }
}

impl Creator for PayloadHandler {
    type Parameters = Node;

    #[cfg(feature = "use-graph")]
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use payload_handler::graph::{
            DEFAULT_CONNECT_SEARCH_BREADTH, DEFAULT_MAX_NUM_SEARCH_THREADS,
            DEFAULT_SEARCH_BREADTH, DEFAULT_SEARCH_TIMEOUT_SEC,
        };

        let mut args = vec![
            clap::Arg::with_name("search_breadth")
                .long("search-breadth")
                .help("Breadth of the search when searching for keys")
                .takes_value(true)
                .default_value(DEFAULT_SEARCH_BREADTH),
            clap::Arg::with_name("connect_search_breadth")
                .long("connect-search-breadth")
                .help("Breadth of the search when connecting to the network")
                .takes_value(true)
                .default_value(DEFAULT_CONNECT_SEARCH_BREADTH),
            clap::Arg::with_name("max_num_search_threads")
                .long("max-num-search-threads")
                .help("Maximum number of threads to spawn when searching")
                .takes_value(true)
                .default_value(DEFAULT_MAX_NUM_SEARCH_THREADS),
            clap::Arg::with_name("search_timeout_sec")
                .long("search-timeout-sec")
                .help("Timeout for querying other node's neighbours")
                .takes_value(true)
                .default_value(DEFAULT_SEARCH_TIMEOUT_SEC),
        ];

        args.append(&mut KeySpaceManager::get_clap_args());
        args.append(&mut NeighboursStore::get_clap_args());
        args
    }

    #[cfg(feature = "use-graph")]
    fn create(
        local_node: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use payload_handler::graph::GraphPayloadHandler;

        let search_breadth = args
            .value_of("search_breadth")
            .unwrap()
            .parse::<usize>()
            .map_err(|_| {
                InternalError::public(
                    "Error on parsing search breadth",
                    ApiErrorType::Parse,
                )
            })?;

        let connect_search_breadth = args
            .value_of("connect_search_breadth")
            .unwrap()
            .parse::<usize>()
            .map_err(|_| {
                InternalError::public(
                    "Error on parsing connect search breadth",
                    ApiErrorType::Parse,
                )
            })?;

        let max_num_search_threads = args
            .value_of("max_num_search_threads")
            .unwrap()
            .parse::<usize>()
            .map_err(|_| {
                InternalError::public(
                    "Error on parsing connect search breadth",
                    ApiErrorType::Parse,
                )
            })?;

        let search_timeout_sec = args
            .value_of("search_timeout_sec")
            .unwrap()
            .parse::<usize>()
            .map_err(|_| {
                InternalError::public(
                    "Error on parsing search timeout",
                    ApiErrorType::Parse,
                )
            })?;

        let key_space_manager: Arc<KeySpaceManager> = KeySpaceManager::create(
            local_node.clone(),
            args,
            log.new(o!("key_space_manager" => true)),
        )?.into();

        let neighbours_store = Arc::new(Mutex::new(
            *(NeighboursStore::create(
                (local_node.key.clone(), key_space_manager.clone()),
                args,
                log.new(o!("neighbours_store" => true)),
            )?),
        ));

        Ok(Box::new(GraphPayloadHandler::new(
            &local_node.key,
            search_breadth,
            connect_search_breadth,
            max_num_search_threads,
            search_timeout_sec,
            key_space_manager,
            neighbours_store,
            log,
        )))
    }

    #[cfg(feature = "use-black-hole")]
    fn create(
        _local_node: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use payload_handler::black_hole::BlackHolePayloadHandler;
        Ok(Box::new(BlackHolePayloadHandler::new(log)))
    }
}
