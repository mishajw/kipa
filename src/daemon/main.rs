#[macro_use] extern crate log;
extern crate clap;
extern crate error_chain;
extern crate kipa_lib;
extern crate simple_logger;

use kipa_lib::creators::*;
use kipa_lib::error::*;
use kipa_lib::gpg_key::GpgKeyHandler;

use error_chain::ChainedError;

fn main() {
    simple_logger::init().unwrap();
    info!("Starting servers");

    let args = clap::App::new("kipa_daemon")
        .arg(clap::Arg::with_name("port")
             .long("port")
             .short("p")
             .help("Port exposed for communicating with other nodes")
             .takes_value(true))
        .arg(clap::Arg::with_name("socket_path")
             .long("socket-path")
             .short("s")
             .help("Socket to listen for local queries from CLI from")
             .takes_value(true))
        .arg(clap::Arg::with_name("key_id")
             .long("key-id")
             .short("k")
             .help("Key read from GPG")
             .takes_value(true)
             .required(true))
        .arg(clap::Arg::with_name("initial_node_key_id")
             .long("initial-node-key-id")
             .help("Key ID of the initial node to connect to")
             .takes_value(true)
             .required(true))
        .arg(clap::Arg::with_name("initial_node_address")
             .long("initial-node-address")
             .help("Address of the initial node to connect to")
             .takes_value(true)
             .required(true))
        .get_matches();

    if let Err(err) = run_servers(&args) {
        println!("{}", err.display_chain().to_string());
    }
}

fn run_servers(args: &clap::ArgMatches) -> Result<()> {
    let mut gpg_key_handler = GpgKeyHandler::new()?;

    // Set up transformer for protobufs
    let data_transformer = create_data_transformer()?;

    // Set up out communication
    let remote_server = create_global_send_server(data_transformer.clone())?;

    // Set up request handler
    let request_handler = create_request_handler(
        &mut gpg_key_handler, remote_server, args)?;

    // Set up listening for connections
    let mut global_server = create_global_receive_server(
        request_handler.clone(), data_transformer.clone(), args)?;

    // Set up local listening for requests
    #[allow(unused)]
    let mut local_server = create_local_receive_server(
        request_handler.clone(), data_transformer.clone(), args)?;

    // Wait for the public server to finish
    global_server.join()?;
    local_server.join()?;

    Ok(())
}
