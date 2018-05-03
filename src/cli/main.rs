extern crate clap;
extern crate error_chain;
extern crate kipa_lib;
extern crate rand;
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use kipa_lib::creators::*;
use kipa_lib::error::*;
use kipa_lib::gpg_key::GpgKeyHandler;
use kipa_lib::api::{RequestPayload, ResponsePayload};
use kipa_lib::{Address, Node};

use error_chain::ChainedError;
use rand::{thread_rng, Rng};

fn main() {
    let log = create_logger("cli");
    info!(log, "Starting CLI");

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
        .subcommand(
            clap::SubCommand::with_name("list-neighbours")
                .about("List all neighbours"),
        )
        .get_matches();

    if let Err(err) = message_daemon(&args, &log) {
        println!("{}", err.display_chain().to_string());
    }
}

fn message_daemon(args: &clap::ArgMatches, log: &slog::Logger) -> Result<()> {
    let mut gpg_key_handler = GpgKeyHandler::new(log.new(o!("gpg" => true)))?;

    let data_transformer = create_data_transformer()?;

    let local_client = create_local_client(
        data_transformer.clone(),
        args,
        log.new(o!("local_client" => true)),
    )?;

    let message_id: u32 = thread_rng().gen();
    info!(log, "Created message ID"; "message_id" => message_id);

    if let Some(search_args) = args.subcommand_matches("search") {
        let search_key = gpg_key_handler
            .get_key(String::from(search_args.value_of("key_id").unwrap()))?;
        let response = local_client
            .send(RequestPayload::SearchRequest(search_key), message_id)?;

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

        let response = local_client
            .send(RequestPayload::ConnectRequest(node), message_id)?;

        match response.payload {
            ResponsePayload::ConnectResponse() => {
                println!("Connect successful");
                Ok(())
            }
            _ => Err(ErrorKind::ParseError("Unrecognized response".into()).into()),
        }
    } else if let Some(_) = args.subcommand_matches("list-neighbours") {
        let response = local_client
            .send(RequestPayload::ListNeighboursRequest(), message_id)?;
        match response.payload {
            ResponsePayload::ListNeighboursResponse(neighbours) => {
                println!("Found neighbours:");
                for n in neighbours {
                    println!("{}", n);
                }
                Ok(())
            }
            _ => Err(ErrorKind::ParseError("Unrecognized response".into()).into()),
        }
    } else {
        Err(ErrorKind::ParseError("No commmand given".into()).into())
    }
}
