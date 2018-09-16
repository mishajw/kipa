extern crate clap;
extern crate error_chain;
extern crate kipa_lib;
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use kipa_lib::creators::*;
use kipa_lib::data_transformer::DataTransformer;
use kipa_lib::error::*;
use kipa_lib::gpg_key::GpgKeyHandler;
use kipa_lib::key_space::KeySpaceManager;
use kipa_lib::message_handler::{MessageHandlerClient, MessageHandlerServer};
use kipa_lib::payload_handler::PayloadHandler;
use kipa_lib::server::{Client, LocalServer, Server};
use kipa_lib::thread_manager::ThreadManager;
use kipa_lib::{Address, LocalAddressParams, Node};

use error_chain::ChainedError;
use std::sync::Arc;

fn main() -> ApiResult<()> {
    let log = create_logger("daemon");
    info!(
        log, "Starting daemon";
        "args" => ::std::env::args().skip(1).collect::<Vec<_>>().join(" "));

    let mut creator_args = vec![];
    creator_args.append(&mut LocalAddressParams::get_clap_args());
    creator_args.append(&mut DataTransformer::get_clap_args());
    creator_args.append(&mut PayloadHandler::get_clap_args());
    creator_args.append(&mut MessageHandlerServer::get_clap_args());
    creator_args.append(&mut Client::get_clap_args());
    creator_args.append(&mut Server::get_clap_args());
    creator_args.append(&mut LocalServer::get_clap_args());
    creator_args.append(&mut GpgKeyHandler::get_clap_args());
    creator_args.append(&mut KeySpaceManager::get_clap_args());

    let args = clap::App::new("kipa_daemon")
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
        .args(&creator_args)
        .get_matches();

    match run_servers(&args, &log) {
        Ok(()) => Ok(()),
        Err(InternalError::PublicError(err, priv_err_opt)) => {
            if let Some(priv_err) = priv_err_opt {
                crit!(
                    log, "Error occured when starting daemon";
                    "err_message" => %priv_err.display_chain());
            }
            println!("Error: {}", err.message);
            Err(err)
        }
        Err(InternalError::PrivateError(err)) => {
            crit!(
                log, "Error occured when starting daemon";
                "err_message" => err.display_chain().to_string());
            Err(ApiError::new(
                "Internal error (check logs)".into(),
                ApiErrorType::Internal,
            ))
        }
    }
}

fn run_servers(
    args: &clap::ArgMatches,
    log: &slog::Logger,
) -> InternalResult<()>
{
    let request_thread_manager = match args
        .value_of("max_num_threads")
        .and_then(|s| s.parse::<usize>().ok())
    {
        Some(max_num_threads) => {
            ThreadManager::from_size("requests".into(), max_num_threads)
        }
        None => ThreadManager::with_default_size("requests".into()),
    };
    let request_thread_manager = Arc::new(request_thread_manager);

    let key_id: String = args.value_of("key_id").unwrap().into();
    let gpg_key_handler: Arc<GpgKeyHandler> =
        GpgKeyHandler::create((), args, log.new(o!("gpg" => true)))?.into();
    gpg_key_handler.copy_user_key(&key_id, true)?;

    // Create local node
    let local_key = gpg_key_handler.get_user_key(key_id)?;
    let local_node = Node::new(
        Address::get_local(
            *LocalAddressParams::create(
                (),
                args,
                log.new(o!("local_address_params" => true)),
            )?,
            log.new(o!("address_creation" => true)),
        )?,
        local_key,
    );

    // Set up transformer for protobufs
    let data_transformer: Arc<DataTransformer> = DataTransformer::create(
        (),
        args,
        log.new(o!("data_transformer" => true)),
    )?.into();

    // Set up out communication
    let client: Arc<Client> =
        Client::create((), args, log.new(o!("client" => true)))?.into();

    // Set up how to handle key spaces
    let key_space_manager: Arc<KeySpaceManager> = KeySpaceManager::create(
        local_node.clone(),
        args,
        log.new(o!("key_space_manager" => true)),
    )?.into();

    let message_handler_client: Arc<MessageHandlerClient> =
        MessageHandlerClient::create(
            (
                local_node.clone(),
                client,
                data_transformer.clone(),
                gpg_key_handler.clone(),
            ),
            args,
            log.new(o!("message_handler_client" => true)),
        )?.into();

    // Set up request handler
    let payload_handler: Arc<PayloadHandler> = PayloadHandler::create(
        (
            local_node.clone(),
            message_handler_client,
            key_space_manager,
        ),
        args,
        log.new(o!("request_handler" => true)),
    )?.into();

    let message_handler_server: Arc<MessageHandlerServer> =
        MessageHandlerServer::create(
            (
                payload_handler,
                data_transformer.clone(),
                gpg_key_handler.clone(),
                local_node.clone(),
            ),
            args,
            log.new(o!("message_handler_server" => true)),
        )?.into();

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

    let server_thread = server.start().map_err(|_| {
        InternalError::public(
            "Error on creating server thread",
            ApiErrorType::Configuration,
        )
    })?;
    let local_server_thread = local_server.start().map_err(|_| {
        InternalError::public(
            "Error on creating local server thread",
            ApiErrorType::Configuration,
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
