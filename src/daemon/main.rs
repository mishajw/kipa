#[macro_use] extern crate log;
#[macro_use] extern crate cfg_if;
extern crate clap;
extern crate error_chain;
extern crate kipa_lib;
extern crate simple_logger;

use kipa_lib::Address;
use kipa_lib::Node;
use kipa_lib::data_transformer::DataTransformer;
use kipa_lib::error::*;
use kipa_lib::gpg_key::GpgKeyHandler;
use kipa_lib::request_handler::RequestHandler;
use kipa_lib::server::{ReceiveServer, SendServer};

use error_chain::ChainedError;
use std::sync::Arc;

fn main() {
    simple_logger::init().unwrap();
    error!("Starting servers");

    let args = clap::App::new("kipa_daemon")
        .arg(clap::Arg::with_name("port")
             .long("port")
             .short("p")
             .help("Port exposed for communicating with other nodes")
             .takes_value(true)
             .required(true))
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
    let remote_server = create_send_server(data_transformer.clone())?;

    // Set up request handler
    let request_handler = create_request_handler(
        &mut gpg_key_handler, remote_server, args)?;

    // Set up listening for connections
    let mut public_server = create_receive_server(
        request_handler.clone(), data_transformer.clone(), args)?;

    // Wait for the public server to finish
    public_server.join()?;

    Ok(())
}

// Create data transformer functions
cfg_if! {
    if #[cfg(feature = "use-protobuf")] {
        use kipa_lib::data_transformer::protobuf::ProtobufDataTransformer;
        fn create_data_transformer() -> Result<Arc<DataTransformer>> {
            Ok(Arc::new(ProtobufDataTransformer{}))
        }
    } else {
        fn create_data_transformer() -> Result<Arc<DataTransformer>> {
            Err(ErrorKind::ConfigError(
                "A data transformer feature was not selected".into()).into())
        }
    }
}

// Create server functions
cfg_if! {
    if #[cfg(feature = "use-tcp")] {
        use kipa_lib::server::tcp::{TcpReceiveServer, TcpSendServer};
        fn create_send_server(
                data_transformer: Arc<DataTransformer>) ->
                Result<Box<SendServer>> {
            Ok(Box::new(TcpSendServer::new(data_transformer)))
        }

        fn create_receive_server(
                request_handler: Arc<RequestHandler>,
                data_transformer: Arc<DataTransformer>,
                args: &clap::ArgMatches) -> Result<Box<ReceiveServer>> {
            let port = args.value_of("port").unwrap()
                .parse::<u16>().chain_err(|| "")?;
            Ok(Box::new(TcpReceiveServer::new(
                request_handler, data_transformer.clone(), port)?))
        }
    } else {
        fn create_send_server(
                data_transformer: Arc<DataTransformer>) ->
                Result<Box<SendServer>> {
            Err(ErrorKind::ConfigError(
                "A server feature was not selected".into()).into())
        }
        fn create_receive_server(
                request_handler: Arc<RequestHandler>,
                data_transformer: Arc<DataTransformer>,
                args: &clap::ArgMatches) -> Result<Box<ReceiveServer>> {
            Err(ErrorKind::ConfigError(
                "A server feature was not selected".into()).into())
        }
    }
}


// Create request handler functions
cfg_if! {
    if #[cfg(feature = "use-graph")] {
        use kipa_lib::request_handler::graph::GraphRequestHandler;
        fn create_request_handler(
                gpg_key_handler: &mut GpgKeyHandler,
                remote_server: Box<SendServer>,
                args: &clap::ArgMatches) -> Result<Arc<RequestHandler>> {

            // Get local key
            let local_key = gpg_key_handler.get_key(
                String::from(args.value_of("key_id").unwrap()))?;

            // Set up initial node
            let initial_node_key =
                gpg_key_handler.get_key(String::from(
                    args.value_of("initial_node_key_id").unwrap()))?;
            let initial_node_address = Address::from_string(
                args.value_of("initial_node_address").unwrap())?;
            let initial_node = Node::new(
                initial_node_address, initial_node_key);

            Ok(Arc::new(GraphRequestHandler::new(
                local_key, remote_server, initial_node)))
        }
    } else {
        fn create_request_handler(
                gpg_key_handler: &mut GpgKeyHandler,
                remote_server: Box<SendServer>,
                args: &clap::ArgMatches) -> Result<Arc<RequestHandler>> {
            Err(ErrorKind::ConfigError(
                "A request handler feature was not selected".into()).into())
        }
    }
}

