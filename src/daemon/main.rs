extern crate clap;
extern crate error_chain;
extern crate kipa_lib;
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use kipa_lib::creators::*;
use kipa_lib::error::*;
use kipa_lib::gpg_key::GpgKeyHandler;
use kipa_lib::socket_server::DEFAULT_PORT;
use kipa_lib::{Address, Node};

use error_chain::ChainedError;
use std::thread;

fn main() {
    let log = create_logger("daemon");
    info!(log, "Starting servers");

    let args = clap::App::new("kipa_daemon")
        .arg(
            clap::Arg::with_name("port")
                .long("port")
                .short("p")
                .help("Port exposed for communicating with other nodes")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("socket_path")
                .long("socket-path")
                .short("s")
                .help("Socket to listen for local queries from CLI from")
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
        .get_matches();

    if let Err(err) = run_servers(&args, &log) {
        println!("{}", err.display_chain().to_string());
    }
}

fn run_servers(args: &clap::ArgMatches, log: &slog::Logger) -> Result<()> {
    let mut gpg_key_handler = GpgKeyHandler::new(log.new(o!("gpg" => true)))?;

    // Create local node
    let port = args.value_of("port")
        .unwrap_or(&DEFAULT_PORT.to_string())
        .parse::<u16>()
        .chain_err(|| "")?;
    let interface = args.value_of("interface");
    // Get local key
    let local_key = gpg_key_handler
        .get_key(String::from(args.value_of("key_id").unwrap()))?;
    let local_node = Node::new(Address::get_local(port, interface)?, local_key);

    // Set up transformer for protobufs
    let data_transformer = create_data_transformer()?;

    // Set up out communication
    let global_client = create_global_client(
        data_transformer.clone(),
        log.new(o!("global-client" => true)),
    )?;

    // Set up request handler
    let payload_handler = create_payload_handler(
        local_node.clone(),
        args,
        log.new(o!("request-handler" => true)),
    )?;

    let message_handler = create_message_handler(
        payload_handler,
        global_client,
        local_node.clone(),
    );

    // Set up listening for connections
    let global_server = create_global_server(
        message_handler.clone(),
        data_transformer.clone(),
        local_node.clone(),
        log.new(o!("global-server" => true)),
    )?;

    // Set up local listening for requests
    let local_server = create_local_server(
        message_handler.clone(),
        data_transformer.clone(),
        args,
        log.new(o!("local-server" => true)),
    )?;

    let global_server_thread = thread::spawn(move || {
        global_server
            .lock()
            .unwrap()
            .start()
            .expect("Error on creating global server thread");
    });
    let local_server_thread = thread::spawn(move || {
        local_server
            .lock()
            .unwrap()
            .start()
            .expect("Error on creating local server thread");
    });

    global_server_thread
        .join()
        .expect("Error on joining global server thread");
    local_server_thread
        .join()
        .expect("Error on joining local server thread");

    Ok(())
}
