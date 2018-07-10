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
use kipa_lib::message_handler::MessageHandler;
use kipa_lib::payload_handler::PayloadHandler;
use kipa_lib::server::{Client, LocalServer, Server};
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
    creator_args.append(&mut MessageHandler::get_clap_args());
    creator_args.append(&mut Client::get_clap_args());
    creator_args.append(&mut Server::get_clap_args());
    creator_args.append(&mut LocalServer::get_clap_args());
    creator_args.append(&mut GpgKeyHandler::get_clap_args());

    let args = clap::App::new("kipa_daemon")
        .arg(
            clap::Arg::with_name("key_id")
                .long("key-id")
                .short("k")
                .help("Key read from GPG")
                .takes_value(true)
                .required(true),
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
    let gpg_key_handler: Arc<GpgKeyHandler> =
        GpgKeyHandler::create((), args, log.new(o!("gpg" => true)))?.into();

    // Create local node
    let local_key =
        gpg_key_handler.get_key(args.value_of("key_id").unwrap().into())?;
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
    let global_client: Arc<Client> =
        Client::create((), args, log.new(o!("global_client" => true)))?.into();

    // Set up request handler
    let payload_handler: Arc<PayloadHandler> = PayloadHandler::create(
        local_node.clone(),
        args,
        log.new(o!("request_handler" => true)),
    )?.into();

    let message_handler: Arc<MessageHandler> = MessageHandler::create(
        (
            payload_handler,
            data_transformer.clone(),
            gpg_key_handler.clone(),
            local_node.clone(),
            global_client,
        ),
        args,
        log.new(o!("message_handler" => true)),
    )?.into();

    // Set up listening for connections
    let global_server = Server::create(
        (message_handler.clone(), local_node.clone()),
        args,
        log.new(o!("global_server" => true)),
    )?;

    // Set up local listening for requests
    let local_server = LocalServer::create(
        message_handler.clone(),
        args,
        log.new(o!("local_server" => true)),
    )?;

    let global_server_thread = global_server.start().map_err(|_| {
        InternalError::public(
            "Error on creating global server thread",
            ApiErrorType::Configuration,
        )
    })?;
    let local_server_thread = local_server.start().map_err(|_| {
        InternalError::public(
            "Error on creating local server thread",
            ApiErrorType::Configuration,
        )
    })?;

    global_server_thread
        .join()
        .expect("Error on joining global server thread");
    local_server_thread
        .join()
        .expect("Error on joining local server thread");

    Ok(())
}
