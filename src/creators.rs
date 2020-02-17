//! Generic construction mechanisms for components of this library
//!
//! These depend on features and conditional compilation in order to bring in
//! the correct implementations.

#![allow(unused)]
#![allow(clippy::type_complexity)]

use api::{Node, SecretKey};
use data_transformer::DataTransformer;
use error::*;
use key_space_manager::KeySpaceManager;
use local_address_params::LocalAddressParams;
use message_handler::{MessageHandlerClient, MessageHandlerLocalClient, MessageHandlerServer};
use payload_handler::PayloadHandler;
use pgp::{GnupgKeyLoader, PgpKeyHandler, SecretLoader};
use server::{Client, LocalClient, LocalServer, Server};
use thread_manager::ThreadManager;
use versioning;

use clap;
use slog;
use slog::{Drain, Fuse};
use slog::{Level, LevelFilter, Logger};
use slog_async;
use slog_json;
use slog_term;
use std::fmt::Debug;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Macro to parse a `clap` argument with appropriate errors
macro_rules! parse_with_err {
    ($value_name:ident, $value_type:ty, $args:ident) => {
        let value_string = $args.value_of(stringify!($value_name)).expect(&format!(
            "Error on getting {} argument",
            stringify!($value_name)
        ));

        let $value_name = value_string.parse::<$value_type>().map_err(|err| {
            InternalError::public_with_error(
                &format!(
                    "Error on parsing parameter {} as {} with value {}",
                    stringify!($value_name),
                    stringify!($value_type),
                    value_string,
                ),
                ApiErrorType::Parse,
                err,
            )
        })?;
    };
}

/// Create a logger, print to `stderr` if creation failed
pub fn get_logger(name: &str, args: &clap::ArgMatches) -> Logger {
    match slog::Logger::create(name.into(), &args, slog::Logger::root(&slog::Discard, o!())) {
        Ok(log) => *log,
        Err(InternalError::PublicError(err, _)) => {
            eprintln!("Error when initializing logging: {}", err.message);
            panic!();
        }
        Err(InternalError::PrivateError(_)) => {
            eprintln!("Error when initializing logging");
            panic!();
        }
    }
}

/// Implementors can be constructed from `clap` arguments
pub trait Creator {
    /// Parameters needed for creating the type
    type Parameters;

    /// Add `clap` arguments to the command line options
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        vec![]
    }

    /// Create the type, given `clap` arguments and parameters
    fn create(
        _parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        _log: Logger,
    ) -> InternalResult<Box<Self>> {
        Err(InternalError::public(
            "Unselected feature",
            ApiErrorType::Configuration,
        ))
    }
}

impl Creator for Logger {
    type Parameters = String;

    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        vec![
            clap::Arg::with_name("write_logs")
                .long("write-logs")
                .help("Whether to write logs to a file")
                .default_value("false")
                .empty_values(false)
                .takes_value(true),
            clap::Arg::with_name("log_directory")
                .long("log-directory")
                .help("Directory to write logs to")
                .default_value("logs")
                .empty_values(false)
                .takes_value(true),
            clap::Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .help("Verbose logging (0=errors, 1=warnings, 2=info, 3=debug, 4=trace)")
                .multiple(true)
                .global(true),
        ]
    }

    fn create(
        name: Self::Parameters,
        args: &clap::ArgMatches,
        _log: Logger,
    ) -> InternalResult<Box<Self>> {
        let verbose_level = args.occurrences_of("verbose");
        let filter_level = match verbose_level {
            0 => Level::Error,
            1 => Level::Warning,
            2 => Level::Info,
            3 => Level::Debug,
            _ => Level::Trace,
        };

        // Create terminal logs.
        parse_with_err!(write_logs, bool, args);
        let stdout = slog_term::TermDecorator::new().build();
        let stdout_drain = slog_term::CompactFormat::new(stdout).build().fuse();
        let stdout_drain = LevelFilter::new(stdout_drain, filter_level).fuse();

        // We need to create this function for DRY with different types of loggers depending on
        // write_logs.
        fn create_logger<T: Drain + Send + 'static>(
            name: String,
            drain: Fuse<T>,
        ) -> InternalResult<Box<Logger>>
        where
            T::Err: Debug,
        {
            let drain = slog_async::Async::new(drain).build().fuse();
            Ok(Box::new(slog::Logger::root(
                Arc::new(drain),
                o!("name" => name, "version" => versioning::get_version()),
            )))
        };

        if !write_logs {
            return create_logger(name, stdout_drain);
        }

        // Create log directory.
        parse_with_err!(log_directory, String, args);
        fs::create_dir_all(&log_directory).map_err(|err| {
            InternalError::public_with_error(
                "Failed to create log directory",
                ApiErrorType::External,
                err,
            )
        })?;

        // Create log file.
        let file_name = &format!("log-{}.json", name);
        let file_directory = Path::new(&log_directory);
        let log_file =
            fs::File::create(file_directory.join(file_name)).expect("Error on creating log file");

        let file_drain = slog_json::Json::new(log_file).add_default_keys().build();
        create_logger(name, slog::Duplicate(file_drain, stdout_drain).fuse())
    }
}

impl Creator for LocalAddressParams {
    type Parameters = ();

    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use local_address_params::DEFAULT_PORT;
        vec![
            clap::Arg::with_name("port")
                .long("port")
                .short("p")
                .help("Port exposed for communicating with other nodes")
                .default_value(DEFAULT_PORT)
                .empty_values(false)
                .takes_value(true),
            clap::Arg::with_name("interface_name")
                .long("interface-name")
                .short("i")
                .help("Interface to operate on")
                .default_value("none")
                .empty_values(false)
                .takes_value(true),
            clap::Arg::with_name("force_ipv6")
                .long("force-ipv6")
                .help("Only pick IPv6 addresses to listen on")
                .default_value("false")
                .empty_values(false)
                .takes_value(true),
        ]
    }

    fn create(
        _parameters: Self::Parameters,
        args: &clap::ArgMatches,
        _log: Logger,
    ) -> InternalResult<Box<Self>> {
        parse_with_err!(port, u16, args);
        parse_with_err!(interface_name, String, args);
        let interface_name = if interface_name == "none" {
            None
        } else {
            Some(interface_name)
        };
        parse_with_err!(force_ipv6, bool, args);
        Ok(Box::new(LocalAddressParams::new(
            port,
            interface_name,
            force_ipv6,
        )))
    }
}

impl Creator for dyn DataTransformer {
    type Parameters = ();
    #[cfg(feature = "use-protobuf")]
    fn create(
        _parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        _log: Logger,
    ) -> InternalResult<Box<Self>> {
        use data_transformer::ProtobufDataTransformer;
        Ok(Box::new(ProtobufDataTransformer {}))
    }
}

impl Creator for PgpKeyHandler {
    type Parameters = ();

    fn create(
        _parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        Ok(Box::new(PgpKeyHandler::new(log)))
    }
}

impl Creator for GnupgKeyLoader {
    type Parameters = ();

    fn create(
        _parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        Ok(Box::new(GnupgKeyLoader::new(log)))
    }
}

impl Creator for SecretLoader {
    type Parameters = ();

    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use pgp::DEFAULT_SECRET_PATH;
        vec![clap::Arg::with_name("secret_path")
            .long("secret-path")
            .help("File containing password for GPG keys")
            .empty_values(false)
            .takes_value(true)
            .default_value(DEFAULT_SECRET_PATH)]
    }

    fn create(
        _parameters: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        let secret_path = args.value_of("secret_path").unwrap();
        Ok(Box::new(SecretLoader::new(secret_path.to_string(), log)))
    }
}

impl Creator for dyn Client {
    type Parameters = ();
    #[cfg(feature = "use-tcp")]
    fn create(
        _parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        use server::tcp::TcpGlobalClient;
        Ok(Box::new(TcpGlobalClient::new(log)))
    }
}

impl Creator for dyn Server {
    type Parameters = (Arc<MessageHandlerServer>, Node, Arc<ThreadManager>);
    #[cfg(feature = "use-tcp")]
    fn create(
        parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        use server::tcp::TcpServer;
        let (message_handler_server, local_node, thread_manager) = parameters;
        Ok(Box::new(TcpServer::new(
            message_handler_server,
            local_node,
            thread_manager,
            log,
        )))
    }
}

impl Creator for dyn LocalServer {
    type Parameters = (Arc<MessageHandlerServer>, Arc<ThreadManager>);

    #[cfg(feature = "use-unix-socket")]
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use server::unix_socket::DEFAULT_UNIX_SOCKET_PATH;
        vec![clap::Arg::with_name("socket_path")
            .long("socket-path")
            .short("s")
            .help("Socket to listen for local queries from CLI from")
            .empty_values(false)
            .takes_value(true)
            .default_value(DEFAULT_UNIX_SOCKET_PATH)]
    }

    #[cfg(feature = "use-unix-socket")]
    fn create(
        parameters: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        use server::unix_socket::UnixSocketLocalServer;
        let (message_handler_server, thread_manager) = parameters;
        let socket_path = args.value_of("socket_path").unwrap();
        let server = to_internal_result(UnixSocketLocalServer::new(
            message_handler_server,
            String::from(socket_path),
            thread_manager,
            log,
        ))?;
        Ok(Box::new(server))
    }
}

impl Creator for dyn LocalClient {
    type Parameters = ();
    // Shares `socket_path` parameter with `LocalServer`
    #[cfg(feature = "use-unix-socket")]
    fn create(
        _parameters: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        use server::unix_socket::UnixSocketLocalClient;
        let socket_path = args.value_of("socket_path").unwrap();
        Ok(Box::new(UnixSocketLocalClient::new(
            socket_path.into(),
            log,
        )))
    }
}

impl Creator for MessageHandlerServer {
    type Parameters = (
        Arc<dyn PayloadHandler>,
        Arc<dyn DataTransformer>,
        Arc<PgpKeyHandler>,
        SecretKey,
    );
    fn create(
        parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        let (payload_handler, data_transformer, pgp_key_handler, local_secret_key) = parameters;
        Ok(Box::new(MessageHandlerServer::new(
            payload_handler,
            local_secret_key,
            data_transformer,
            pgp_key_handler,
            log,
        )))
    }
}

impl Creator for MessageHandlerClient {
    type Parameters = (
        Node,
        SecretKey,
        Arc<dyn Client>,
        Arc<dyn DataTransformer>,
        Arc<PgpKeyHandler>,
    );
    fn create(
        parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        let (local_node, local_secret_key, client, data_transformer, pgp_key_handler) = parameters;
        Ok(Box::new(MessageHandlerClient::new(
            local_node,
            local_secret_key,
            client,
            data_transformer,
            pgp_key_handler,
            log,
        )))
    }
}

impl Creator for MessageHandlerLocalClient {
    type Parameters = (Arc<dyn LocalClient>, Arc<dyn DataTransformer>);
    fn create(
        parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        let (local_client, data_transformer) = parameters;
        Ok(Box::new(MessageHandlerLocalClient::new(
            local_client,
            data_transformer,
            log,
        )))
    }
}

impl Creator for KeySpaceManager {
    type Parameters = Node;
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use key_space_manager::DEFAULT_KEY_SPACE_SIZE;
        vec![clap::Arg::with_name("key_space_size")
            .long("key-space-size")
            .help("Number of dimensions to use for key space")
            .empty_values(false)
            .takes_value(true)
            .default_value(DEFAULT_KEY_SPACE_SIZE)]
    }

    fn create(
        _parameters: Self::Parameters,
        args: &clap::ArgMatches,
        _log: Logger,
    ) -> InternalResult<Box<Self>> {
        parse_with_err!(key_space_size, usize, args);
        Ok(Box::new(KeySpaceManager::new(key_space_size)))
    }
}

#[cfg(feature = "use-graph")]
use graph::NeighboursStore;
#[cfg(feature = "use-graph")]
impl Creator for NeighboursStore {
    type Parameters = (::api::Key, Arc<KeySpaceManager>, Arc<MessageHandlerClient>);
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use graph::{
            DEFAULT_ANGLE_WEIGHTING, DEFAULT_DISTANCE_WEIGHTING, DEFAULT_MAX_NUM_NEIGHBOURS,
        };
        vec![
            clap::Arg::with_name("neighbours_size")
                .long("neighbours-size")
                .help("Maximum number of neighbours to store")
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_MAX_NUM_NEIGHBOURS),
            clap::Arg::with_name("distance_weighting")
                .long("distance-weighting")
                .help("Weight of the distance when considering neighbours")
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_DISTANCE_WEIGHTING),
            clap::Arg::with_name("angle_weighting")
                .long("angle-weighting")
                .help("Weight of the angle when considering neighbours")
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_ANGLE_WEIGHTING),
        ]
    }

    fn create(
        parameters: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        use api::RequestPayload;
        use std::time::Duration;

        let (local_key, key_space_manager, message_handler_client) = parameters;
        parse_with_err!(neighbours_size, usize, args);
        parse_with_err!(distance_weighting, f32, args);
        parse_with_err!(angle_weighting, f32, args);

        Ok(Box::new(NeighboursStore::new(
            &local_key,
            neighbours_size,
            distance_weighting,
            angle_weighting,
            key_space_manager,
            Arc::new(move |n| {
                message_handler_client
                    .send_request(n, RequestPayload::VerifyRequest(), Duration::from_secs(3))
                    .map(|_| ())
            }),
            log,
        )))
    }
}

impl Creator for dyn PayloadHandler {
    type Parameters = (Node, Arc<MessageHandlerClient>, Arc<KeySpaceManager>);

    #[cfg(feature = "use-graph")]
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use graph::neighbour_gc::{DEFAULT_ENABLED, DEFAULT_FREQUENCY_SEC, DEFAULT_NUM_RETRIES};
        use graph::{
            DEFAULT_CONNECT_SEARCH_BREADTH, DEFAULT_MAX_NUM_SEARCH_THREADS, DEFAULT_SEARCH_BREADTH,
            DEFAULT_SEARCH_THREAD_POOL_SIZE, DEFAULT_SEARCH_TIMEOUT_SEC,
        };

        let mut args = vec![
            clap::Arg::with_name("search_breadth")
                .long("search-breadth")
                .help("Breadth of the search when searching for keys")
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_SEARCH_BREADTH),
            clap::Arg::with_name("connect_search_breadth")
                .long("connect-search-breadth")
                .help("Breadth of the search when connecting to the network")
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_CONNECT_SEARCH_BREADTH),
            clap::Arg::with_name("max_num_search_threads")
                .long("max-num-search-threads")
                .help("Maximum number of threads to spawn when searching")
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_MAX_NUM_SEARCH_THREADS),
            clap::Arg::with_name("search_timeout_sec")
                .long("search-timeout-sec")
                .help("Timeout for querying other node's neighbours")
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_SEARCH_TIMEOUT_SEC),
            clap::Arg::with_name("neighbour_gc_frequency_sec")
                .long("neighbour-gc-frequency-sec")
                .help(
                    "How often to check if neighbours are still responding to \
                     requests",
                )
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_FREQUENCY_SEC),
            clap::Arg::with_name("neighbour_gc_num_retries")
                .long("neighbour-gc-num-retries")
                .help(
                    "Number of retries to attempt before regarding a \
                     neighbour as unresponsive",
                )
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_NUM_RETRIES),
            clap::Arg::with_name("neighbour_gc_enabled")
                .long("neighbour-gc-enabled")
                .help("Enable garbage collection of unresponsive neighbours")
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_ENABLED),
            clap::Arg::with_name("search_thread_pool_size")
                .long("search-thread-pool-size")
                .help(
                    "Thread pool size shared between search operations for \
                     spawning querying threads",
                )
                .empty_values(false)
                .takes_value(true)
                .default_value(DEFAULT_SEARCH_THREAD_POOL_SIZE),
        ];

        args.append(&mut NeighboursStore::get_clap_args());
        args
    }

    #[cfg(feature = "use-graph")]
    fn create(
        parameters: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        use graph::neighbour_gc::NeighbourGc;
        use graph::{GraphParams, GraphPayloadHandler};
        use std::time::Duration;

        let (local_node, message_handler_client, key_space_manager) = parameters;

        parse_with_err!(search_breadth, usize, args);
        parse_with_err!(connect_search_breadth, usize, args);
        parse_with_err!(max_num_search_threads, usize, args);
        parse_with_err!(search_timeout_sec, usize, args);
        parse_with_err!(neighbour_gc_frequency_sec, u64, args);
        parse_with_err!(neighbour_gc_num_retries, u32, args);
        parse_with_err!(neighbour_gc_enabled, bool, args);
        parse_with_err!(search_thread_pool_size, usize, args);

        let neighbours_store = Arc::new(
            *(NeighboursStore::create(
                (
                    local_node.key.clone(),
                    key_space_manager.clone(),
                    message_handler_client.clone(),
                ),
                args,
                log.new(o!("neighbours_store" => true)),
            )?),
        );

        if neighbour_gc_enabled {
            NeighbourGc::new(
                neighbours_store.clone(),
                message_handler_client.clone(),
                neighbour_gc_num_retries,
                log.new(o!("neighbour_gc" => true)),
            )
            .start(Duration::from_secs(neighbour_gc_frequency_sec));
        }

        Ok(Box::new(GraphPayloadHandler::new(
            local_node,
            message_handler_client,
            key_space_manager,
            neighbours_store,
            search_thread_pool_size,
            GraphParams {
                search_breadth,
                connect_search_breadth,
                max_num_search_threads,
                search_timeout: Duration::from_secs(search_timeout_sec as u64),
            },
            log,
        )))
    }

    #[cfg(feature = "use-black-hole")]
    fn create(
        _local_node: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        use payload_handler::black_hole::BlackHolePayloadHandler;
        Ok(Box::new(BlackHolePayloadHandler::new(log)))
    }

    #[cfg(feature = "use-random-response")]
    fn create(
        _local_node: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>> {
        use payload_handler::random_response::RandomResponsePayloadHandler;
        Ok(Box::new(RandomResponsePayloadHandler::new(log)))
    }
}
