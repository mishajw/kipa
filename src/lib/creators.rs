//! Generic construction mechanisms for components of this library
//!
//! These depend on features and conditional compilation in order to bring in
//! the correct implementations.

use address::LocalAddressParams;
use data_transformer::DataTransformer;
use error::*;
use gpg_key::GpgKeyHandler;
use key_space::KeySpaceManager;
use message_handler::{
    MessageHandlerClient, MessageHandlerLocalClient, MessageHandlerServer,
};
use node::Node;
use payload_handler::PayloadHandler;
#[allow(unused)]
use server::{Client, LocalClient, LocalServer, Server};
use versioning;

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

/// Macro to parse a `clap` argument with appropriate errors
macro_rules! parse_with_err {
    ($value_name:ident, $value_type:ty, $args:ident) => {
        let $value_name = $args
            .value_of(stringify!($value_name))
            .expect(&format!(
                "Error on getting {} argument",
                stringify!($value_name)
            ))
            .parse::<$value_type>()
            .map_err(|err| {
                InternalError::public_with_error(
                    &format!(
                        "Error on parsing {} as {}",
                        stringify!($value_name),
                        stringify!($value_type)
                    ),
                    ApiErrorType::Parse,
                    err,
                )
            })?;
    };
}

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
        o!("name" => name, "version" => versioning::get_version()),
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

impl Creator for LocalAddressParams {
    type Parameters = ();

    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use address::DEFAULT_PORT;
        vec![
            clap::Arg::with_name("port")
                .long("port")
                .short("p")
                .help("Port exposed for communicating with other nodes")
                .default_value(DEFAULT_PORT)
                .takes_value(true),
            clap::Arg::with_name("interface_name")
                .long("interface-name")
                .short("i")
                .help("Interface to operate on")
                .default_value("none")
                .takes_value(true),
            clap::Arg::with_name("force_ipv6")
                .long("force-ipv6")
                .help("Only pick IPv6 addresses to listen on")
                .default_value("false")
                .takes_value(true),
        ]
    }

    fn create(
        _parameters: Self::Parameters,
        args: &clap::ArgMatches,
        _log: Logger,
    ) -> InternalResult<Box<Self>>
    {
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

impl Creator for GpgKeyHandler {
    type Parameters = ();

    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use gpg_key::{
            DEFAULT_OWNED_GNUPG_HOME_DIRECTORY, DEFAULT_SECRET_PATH,
        };
        vec![
            clap::Arg::with_name("owned_gnupg_home_directory")
                .long("gnupg-home-directory")
                .help("Modifiable GnuPG directory to load/delete keys from")
                .takes_value(true)
                .default_value(DEFAULT_OWNED_GNUPG_HOME_DIRECTORY),
            clap::Arg::with_name("secret_path")
                .long("secret-path")
                .help("File containing password for GPG keys")
                .takes_value(true)
                .default_value(DEFAULT_SECRET_PATH),
        ]
    }

    fn create(
        _parameters: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        let owned_gnupg_home_directory =
            args.value_of("owned_gnupg_home_directory").unwrap();
        let secret_path = args.value_of("secret_path").unwrap();
        Ok(Box::new(GpgKeyHandler::new(
            owned_gnupg_home_directory.into(),
            secret_path,
            log,
        )?))
    }
}

impl Creator for Client {
    type Parameters = ();
    #[cfg(feature = "use-tcp")]
    fn create(
        _parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use server::tcp::TcpGlobalClient;
        Ok(Box::new(TcpGlobalClient::new(log)))
    }
}

impl Creator for Server {
    type Parameters = (Arc<MessageHandlerServer>, Node);
    #[cfg(feature = "use-tcp")]
    fn create(
        parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use server::tcp::TcpServer;
        let (message_handler_server, local_node) = parameters;
        Ok(Box::new(TcpServer::new(
            message_handler_server,
            local_node,
            log,
        )))
    }
}

impl Creator for LocalServer {
    type Parameters = Arc<MessageHandlerServer>;

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
        let message_handler_server = parameters;
        let socket_path = args.value_of("socket_path").unwrap();
        let server = to_internal_result(UnixSocketLocalServer::new(
            message_handler_server,
            String::from(socket_path),
            log,
        ))?;
        Ok(Box::new(server))
    }
}

impl Creator for LocalClient {
    type Parameters = ();
    // Shares `socket_path` parameter with `LocalServer`
    #[cfg(feature = "use-unix-socket")]
    fn create(
        _parameters: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
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
        Arc<PayloadHandler>,
        Arc<DataTransformer>,
        Arc<GpgKeyHandler>,
        Node,
    );
    fn create(
        parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        let (payload_handler, data_transformer, gpg_key_handler, local_node) =
            parameters;
        Ok(Box::new(MessageHandlerServer::new(
            payload_handler,
            local_node,
            data_transformer,
            gpg_key_handler,
            log,
        )))
    }
}

impl Creator for MessageHandlerClient {
    type Parameters =
        (Node, Arc<Client>, Arc<DataTransformer>, Arc<GpgKeyHandler>);
    fn create(
        parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        let (local_node, client, data_transformer, gpg_key_handler) =
            parameters;
        Ok(Box::new(MessageHandlerClient::new(
            local_node,
            client,
            data_transformer,
            gpg_key_handler,
            log,
        )))
    }
}

impl Creator for MessageHandlerLocalClient {
    type Parameters = (Arc<LocalClient>, Arc<DataTransformer>);
    fn create(
        parameters: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
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
        use key_space::DEFAULT_KEY_SPACE_SIZE;
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
        parse_with_err!(key_space_size, usize, args);
        Ok(Box::new(KeySpaceManager::new(key_space_size)))
    }
}

#[cfg(feature = "use-graph")]
use payload_handler::graph::NeighboursStore;
#[cfg(feature = "use-graph")]
impl Creator for NeighboursStore {
    type Parameters =
        (::key::Key, Arc<KeySpaceManager>, Arc<MessageHandlerClient>);
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
                    .send(
                        n,
                        RequestPayload::VerifyRequest(),
                        Duration::from_secs(3),
                    )
                    .map(|_| ())
            }),
            log.new(o!("neighbours_store" => true)),
        )))
    }
}

impl Creator for PayloadHandler {
    type Parameters = (Node, Arc<MessageHandlerClient>);

    #[cfg(feature = "use-graph")]
    fn get_clap_args<'a, 'b>() -> Vec<clap::Arg<'a, 'b>> {
        use payload_handler::graph::neighbour_gc::{
            DEFAULT_FREQUENCY_SEC, DEFAULT_NUM_RETRIES,
            DEFAULT_RETRY_FREQUENCY_SEC,
        };
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
            clap::Arg::with_name("neighbour_gc_frequency_sec")
                .long("neighbour-gc-frequency-sec")
                .help(
                    "How often to check if neighbours are still responding to \
                     requests",
                )
                .takes_value(true)
                .default_value(DEFAULT_FREQUENCY_SEC),
            clap::Arg::with_name("neighbour_gc_num_retries")
                .long("neighbour-gc-num-retries")
                .help(
                    "Number of retries to attempt before regarding a \
                     neighbour as unresponsive",
                )
                .takes_value(true)
                .default_value(DEFAULT_NUM_RETRIES),
            clap::Arg::with_name("neighbour_gc_retry_frequency_sec")
                .long("neighbour-gc-retry-frequency-sec")
                .help("Time to wait between checking if a neighbour is alive")
                .takes_value(true)
                .default_value(DEFAULT_RETRY_FREQUENCY_SEC),
        ];

        args.append(&mut KeySpaceManager::get_clap_args());
        args.append(&mut NeighboursStore::get_clap_args());
        args
    }

    #[cfg(feature = "use-graph")]
    fn create(
        parameters: Self::Parameters,
        args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use payload_handler::graph::{neighbour_gc, GraphPayloadHandler};
        use std::time::Duration;

        let (local_node, message_handler_client) = parameters;

        parse_with_err!(search_breadth, usize, args);
        parse_with_err!(connect_search_breadth, usize, args);
        parse_with_err!(max_num_search_threads, usize, args);
        parse_with_err!(search_timeout_sec, usize, args);
        parse_with_err!(neighbour_gc_frequency_sec, u64, args);
        parse_with_err!(neighbour_gc_num_retries, u32, args);
        parse_with_err!(neighbour_gc_retry_frequency_sec, u64, args);

        let key_space_manager: Arc<KeySpaceManager> = KeySpaceManager::create(
            local_node.clone(),
            args,
            log.new(o!("key_space_manager" => true)),
        )?.into();

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

        neighbour_gc::start_gc(
            neighbours_store.clone(),
            message_handler_client.clone(),
            Duration::from_secs(neighbour_gc_frequency_sec),
            neighbour_gc_num_retries,
            Duration::from_secs(neighbour_gc_retry_frequency_sec),
            log.new(o!("neighbour_gc" => true)),
        );

        Ok(Box::new(GraphPayloadHandler::new(
            &local_node.key,
            search_breadth,
            connect_search_breadth,
            max_num_search_threads,
            search_timeout_sec,
            message_handler_client,
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

    #[cfg(feature = "use-random-response")]
    fn create(
        _local_node: Self::Parameters,
        _args: &clap::ArgMatches,
        log: Logger,
    ) -> InternalResult<Box<Self>>
    {
        use payload_handler::random_response::RandomResponsePayloadHandler;
        Ok(Box::new(RandomResponsePayloadHandler::new(log)))
    }
}
