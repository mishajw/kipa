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
use kipa_lib::socket_server::DEFAULT_PORT;
use kipa_lib::{Address, Node};

use error_chain::ChainedError;
use std::sync::Arc;

fn main() {
    let log = create_logger("daemon");
    info!(log, "Starting servers");

    let mut creator_args = vec![];
    creator_args.append(&mut DataTransformer::get_clap_args());
    creator_args.append(&mut PayloadHandler::get_clap_args());
    creator_args.append(&mut MessageHandler::get_clap_args());
    creator_args.append(&mut Client::get_clap_args());
    creator_args.append(&mut Server::get_clap_args());
    creator_args.append(&mut LocalServer::get_clap_args());

    let args = clap::App::new("kipa_daemon")
        .arg(
            clap::Arg::with_name("port")
                .long("port")
                .short("p")
                .help("Port exposed for communicating with other nodes")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("key_id")
                .long("key-id")
                .short("k")
                .help("Key read from GPG")
                .takes_value(true)
                .required(true),
        )
        .arg(
            clap::Arg::with_name("interface")
                .long("interface")
                .short("i")
                .help("Interface to operate on")
                .takes_value(true),
        )
        .args(&creator_args)
        .get_matches();

    match run_servers(&args, &log) {
        Ok(()) => println!("Daemon running"),
        Err(InternalError::PublicError(err)) => println!("{}", err.message),
        Err(InternalError::PrivateError(err)) => crit!(
            log, "Error occured when setting up daemon";
            "err_message" => err.display_chain().to_string()),
    }
}

fn run_servers(
    args: &clap::ArgMatches,
    log: &slog::Logger,
) -> InternalResult<()>
{
    let mut gpg_key_handler = GpgKeyHandler::new(log.new(o!("gpg" => true)))?;

    // Create local node
    let port = args
        .value_of("port")
        .unwrap_or(&DEFAULT_PORT.to_string())
        .parse::<u16>()
        .map_err(|_| InternalError::public("Error on parsing port number"))?;
    let interface = args.value_of("interface");
    // Get local key
    let local_key = gpg_key_handler
        .get_key(String::from(args.value_of("key_id").unwrap()))?;
    let local_node = Node::new(
        Address::get_local(
            port,
            interface,
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
    let global_client: Arc<Client> = Client::create(
        data_transformer.clone(),
        args,
        log.new(o!("global_client" => true)),
    )?.into();

    // Set up request handler
    let payload_handler: Arc<PayloadHandler> = PayloadHandler::create(
        local_node.clone(),
        args,
        log.new(o!("request_handler" => true)),
    )?.into();

    let message_handler: Arc<MessageHandler> = MessageHandler::create(
        (payload_handler, local_node.clone(), global_client),
        args,
        log.new(o!("message_handler" => true)),
    )?.into();

    // Set up listening for connections
    let global_server = Server::create(
        (
            message_handler.clone(),
            data_transformer.clone(),
            local_node.clone(),
        ),
        args,
        log.new(o!("global_server" => true)),
    )?;

    // Set up local listening for requests
    let local_server = LocalServer::create(
        (message_handler.clone(), data_transformer.clone()),
        args,
        log.new(o!("local_server" => true)),
    )?;

    let global_server_thread = global_server.start().map_err(|_| {
        InternalError::public("Error on creating global server thread")
    })?;
    let local_server_thread = local_server.start().map_err(|_| {
        InternalError::public("Error on creating local server thread")
    })?;

    global_server_thread
        .join()
        .expect("Error on joining global server thread");
    local_server_thread
        .join()
        .expect("Error on joining local server thread");

    Ok(())
}
