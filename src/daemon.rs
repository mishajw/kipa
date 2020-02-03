extern crate clap;
extern crate error_chain;
extern crate kipa_lib;
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use std::sync::Arc;

use error_chain::ChainedError;

use kipa_lib::api::error::ApiErrorType;
use kipa_lib::api::{Address, Node, RequestPayload};
use kipa_lib::creators::*;
use kipa_lib::data_transformer::DataTransformer;
use kipa_lib::error::*;
use kipa_lib::key_space_manager::KeySpaceManager;
use kipa_lib::local_address_params::LocalAddressParams;
use kipa_lib::message_handler::{MessageHandlerClient, MessageHandlerServer};
use kipa_lib::payload_handler::PayloadHandler;
use kipa_lib::pgp::GnupgKeyLoader;
use kipa_lib::pgp::PgpKeyHandler;
use kipa_lib::pgp::SecretLoader;
use kipa_lib::remotery_util;
use kipa_lib::server::{Client, LocalServer, Server};
use kipa_lib::thread_manager::ThreadManager;
use kipa_lib::versioning;

fn main() -> std::result::Result<(), String> {
    let mut creator_args = vec![];
    creator_args.append(&mut slog::Logger::get_clap_args());
    creator_args.append(&mut LocalAddressParams::get_clap_args());
    creator_args.append(&mut DataTransformer::get_clap_args());
    creator_args.append(&mut PayloadHandler::get_clap_args());
    creator_args.append(&mut MessageHandlerServer::get_clap_args());
    creator_args.append(&mut Client::get_clap_args());
    creator_args.append(&mut Server::get_clap_args());
    creator_args.append(&mut LocalServer::get_clap_args());
    creator_args.append(&mut SecretLoader::get_clap_args());
    creator_args.append(&mut KeySpaceManager::get_clap_args());

    let args = clap::App::new("kipa-daemon")
        .version(&*versioning::get_version())
        .arg(
            clap::Arg::with_name("key_id")
                .long("key-id")
                .short("k")
                .help("Key read from GPG")
                .takes_value(true)
                .required(true),
        )
        .arg(
            clap::Arg::with_name("max_num_threads")
                .long("max-num-threads")
                .short("j")
                .help("Maximum number of active threads")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("connect_address")
                .long("connect-address")
                .help("If set, KIPA will connect to the daemon at this address on startup")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("connect_key_id")
                .long("connect-key-id")
                .help("If set, KIPA will connect to a daemon with this key on startup")
                .takes_value(true),
        )
        .args(&creator_args)
        .get_matches();

    let log: slog::Logger = get_logger("daemon", &args);
    info!(
        log, "Starting daemon";
        "args" => ::std::env::args().skip(1).collect::<Vec<_>>().join(" "));
    let _remotery = remotery_util::initialize_remotery(&log);

    match run_servers(&args, &log) {
        Ok(()) => Ok(()),
        Err(InternalError::PublicError(err, priv_err_opt)) => {
            if let Some(priv_err) = priv_err_opt {
                debug!(
                    log, "Error occurred when starting daemon";
                    "err_message" => %priv_err.display_chain());
            }
            Err(err.message)
        }
        Err(InternalError::PrivateError(err)) => {
            crit!(
                log, "Error occurred when starting daemon";
                "err_message" => err.display_chain().to_string());
            Err("Internal error (check logs)".into())
        }
    }
}

fn run_servers(args: &clap::ArgMatches, log: &slog::Logger) -> InternalResult<()> {
    let request_thread_manager = match args
        .value_of("max_num_threads")
        .and_then(|s| s.parse::<usize>().ok())
    {
        Some(max_num_threads) => ThreadManager::from_size("requests".into(), max_num_threads),
        None => ThreadManager::with_default_size("requests".into()),
    };
    let request_thread_manager = Arc::new(request_thread_manager);

    let key_id: String = args.value_of("key_id").unwrap().into();
    let secret_loader: SecretLoader =
        *SecretLoader::create((), args, log.new(o!("secret_loader" => true)))?;
    let gnupg_key_loader: GnupgKeyLoader =
        *GnupgKeyLoader::create((), args, log.new(o!("gnupg_key_loader" => true)))?;
    let pgp_key_handler: Arc<PgpKeyHandler> =
        PgpKeyHandler::create((), args, log.new(o!("pgp_key_handler" => true)))?.into();

    // Create local node
    let local_secret_key = gnupg_key_loader.get_local_private_key(key_id, secret_loader)?;
    let local_node = Node::new(
        LocalAddressParams::create((), args, log.new(o!("local_address_params" => true)))?
            .create_address(log.new(o!("address_creation" => true)))?,
        local_secret_key.public().map_err(InternalError::private)?,
    );

    // Set up transformer for protobufs
    let data_transformer: Arc<dyn DataTransformer> =
        DataTransformer::create((), args, log.new(o!("data_transformer" => true)))?.into();

    // Set up out communication
    let client: Arc<dyn Client> = Client::create((), args, log.new(o!("client" => true)))?.into();

    // Set up how to handle key spaces
    let key_space_manager: Arc<KeySpaceManager> = KeySpaceManager::create(
        local_node.clone(),
        args,
        log.new(o!("key_space_manager" => true)),
    )?
    .into();

    let message_handler_client: Arc<MessageHandlerClient> = MessageHandlerClient::create(
        (
            local_node.clone(),
            local_secret_key.clone(),
            client,
            data_transformer.clone(),
            pgp_key_handler.clone(),
        ),
        args,
        log.new(o!("message_handler_client" => true)),
    )?
    .into();

    // Set up request handler
    let payload_handler: Arc<dyn PayloadHandler> = PayloadHandler::create(
        (
            local_node.clone(),
            message_handler_client,
            key_space_manager,
        ),
        args,
        log.new(o!("request_handler" => true)),
    )?
    .into();

    // Now we've set up the payload handler, we can connect to a node at startup.
    match (
        args.value_of("connect_address"),
        args.value_of("connect_key_id"),
    ) {
        (Some(connect_address), Some(connect_key_id)) => {
            let node = Node::new(
                Address::from_string(connect_address)?,
                gnupg_key_loader.get_recipient_public_key(connect_key_id.into())?,
            );
            info!(&log, "Connecting to the initial node"; "node" => %node);
            payload_handler.receive(&RequestPayload::ConnectRequest(node), None, 0)?;
        }
        (Some(_), None) => {
            return Err(InternalError::public(
                "If --connect-address is set, --connect-key-id must be too.",
                ApiErrorType::Configuration,
            ))
        }
        (None, Some(_)) => {
            return Err(InternalError::public(
                "If --connect-key-id is set, --connect-address must be too.",
                ApiErrorType::Configuration,
            ))
        }
        (None, None) => {}
    };

    let message_handler_server: Arc<MessageHandlerServer> = MessageHandlerServer::create(
        (
            payload_handler,
            data_transformer.clone(),
            pgp_key_handler.clone(),
            local_secret_key.clone(),
        ),
        args,
        log.new(o!("message_handler_server" => true)),
    )?
    .into();

    // Set up listening for connections
    let server = Server::create(
        (
            message_handler_server.clone(),
            local_node.clone(),
            request_thread_manager.clone(),
        ),
        args,
        log.new(o!("server" => true)),
    )?;

    // Set up local listening for requests
    let local_server = LocalServer::create(
        (message_handler_server.clone(), request_thread_manager),
        args,
        log.new(o!("local_server" => true)),
    )?;

    let server_thread = server.start()?;
    let local_server_thread = local_server.start().map_err(|err| {
        InternalError::public_with_error(
            "Error on creating local server thread",
            ApiErrorType::Configuration,
            err,
        )
    })?;

    server_thread
        .join()
        .expect("Error on joining server thread");
    local_server_thread
        .join()
        .expect("Error on joining local server thread");

    Ok(())
}
