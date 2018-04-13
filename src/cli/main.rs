extern crate clap;
extern crate error_chain;
extern crate kipa_lib;
#[macro_use]
extern crate log;
extern crate simple_logger;

use kipa_lib::creators::*;
use kipa_lib::error::*;
use kipa_lib::gpg_key::GpgKeyHandler;
use kipa_lib::api::{RequestPayload, ResponsePayload};
use kipa_lib::{Address, Node};

use error_chain::ChainedError;

fn main() {
    simple_logger::init().unwrap();
    info!("Starting CLI");

    let args = clap::App::new("kipa_daemon")
        .arg(
            clap::Arg::with_name("socket_path")
                .long("socket-path")
                .short("s")
                .help("Socket to communicate with daemon")
                .takes_value(true),
        )
        .subcommand(
            clap::SubCommand::with_name("search")
                .about("Search for a node given a key")
                .arg(
                    clap::Arg::with_name("key_id")
                        .long("key-id")
                        .short("k")
                        .help("The key to search for")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            clap::SubCommand::with_name("connect")
                .about("Connect to a node with a key and IP address")
                .arg(
                    clap::Arg::with_name("key_id")
                        .long("key-id")
                        .short("k")
                        .help("The key to connect to")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    clap::Arg::with_name("address")
                        .long("address")
                        .short("a")
                        .help("The IP address to connect to")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .get_matches();

    if let Err(err) = message_daemon(&args) {
        println!("{}", err.display_chain().to_string());
    }
}

fn message_daemon(args: &clap::ArgMatches) -> Result<()> {
    let mut gpg_key_handler = GpgKeyHandler::new()?;

    let data_transformer = create_data_transformer()?;

    let local_send_server =
        create_local_send_server(data_transformer.clone(), args)?;

    if let Some(search_args) = args.subcommand_matches("search") {
        let search_key = gpg_key_handler
            .get_key(String::from(search_args.value_of("key_id").unwrap()))?;
        let response = local_send_server
            .receive(RequestPayload::SearchRequest(search_key))?;

        match response.payload {
            ResponsePayload::SearchResponse(Some(ref node)) => {
                println!("Search success: {}.", node);
                Ok(())
            }
            ResponsePayload::SearchResponse(None) => {
                println!("Search unsuccessful.");
                Ok(())
            }
            _ => Err(ErrorKind::ParseError("Unrecognized response".into()).into()),
        }
    } else if let Some(connect_args) = args.subcommand_matches("connect") {
        // Get node from arguments
        let node_key = gpg_key_handler
            .get_key(String::from(connect_args.value_of("key_id").unwrap()))?;
        let node_address =
            Address::from_string(connect_args.value_of("address").unwrap())?;
        let node = Node::new(node_address, node_key);

        let response =
            local_send_server.receive(RequestPayload::ConnectRequest(node))?;

        match response.payload {
            ResponsePayload::ConnectResponse() => {
                println!("Connect successful");
                Ok(())
            }
            _ => Err(ErrorKind::ParseError("Unrecognized response".into()).into()),
        }
    } else {
        Err(ErrorKind::ParseError("No commmand given".into()).into())
    }
}
