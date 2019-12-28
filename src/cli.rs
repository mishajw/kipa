extern crate clap;
extern crate error_chain;
extern crate kipa_lib;
extern crate rand;
#[macro_use]
extern crate slog;
extern crate slog_async;
extern crate slog_term;

use kipa_lib::api::{Address, Node};
use kipa_lib::api::{RequestPayload, ResponsePayload};
use kipa_lib::creators::*;
use kipa_lib::data_transformer::DataTransformer;
use kipa_lib::error::*;
use kipa_lib::message_handler::MessageHandlerLocalClient;
use kipa_lib::pgp::GnupgKeyLoader;
use kipa_lib::server::{LocalClient, LocalServer};

use error_chain::ChainedError;
use std::sync::Arc;

// TODO: Change from returning `ApiResult<()>` to an error code linked to
// `ApiErrorType` - should be possible with `std::process::Termination`, but
// this is only available in nightly. Keep an eye on issue #43301
fn main() -> ApiResult<()> {
    let mut creator_args = vec![];
    creator_args.append(&mut slog::Logger::get_clap_args());
    creator_args.append(&mut DataTransformer::get_clap_args());
    creator_args.append(&mut LocalServer::get_clap_args());
    creator_args.append(&mut LocalClient::get_clap_args());

    let args = clap::App::new("kipa-cli")
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
                )
                .arg(
                    clap::Arg::with_name("print")
                        .long("print")
                        .short("p")
                        .help("What results to print")
                        .takes_value(true)
                        .possible_values(&vec!["ip", "port", "all"])
                        .default_value("all"),
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
        .subcommand(clap::SubCommand::with_name("list-neighbours").about("List all neighbours"))
        .args(&creator_args)
        .get_matches();

    let log: slog::Logger = get_logger("cli", &args);
    info!(
        log, "Starting CLI";
        "args" => ::std::env::args().skip(1).collect::<Vec<_>>().join(" "));

    match message_daemon(&args, &log) {
        Ok(()) => Ok(()),
        Err(InternalError::PublicError(err, priv_err_opt)) => {
            if let Some(priv_err) = priv_err_opt {
                crit!(
                    log, "Error occurred when performing command";
                    "err_message" => %priv_err.display_chain());
            }
            println!("Error: {}", err.message);
            Err(err)
        }
        Err(InternalError::PrivateError(err)) => {
            crit!(
                log, "Error occurred when performing command";
                "err_message" => %err.display_chain());
            Err(ApiError::new(
                "Internal error (check logs)".into(),
                ApiErrorType::Internal,
            ))
        }
    }
}

fn message_daemon(args: &clap::ArgMatches, log: &slog::Logger) -> InternalResult<()> {
    let gnupg_key_loader: GnupgKeyLoader =
        *GnupgKeyLoader::create((), args, log.new(o!("gnupg_key_loader" => true)))?;

    let data_transformer: Arc<dyn DataTransformer> =
        DataTransformer::create((), args, log.new(o!("data_transformer" => true)))?.into();

    let local_client: Arc<dyn LocalClient> =
        LocalClient::create((), args, log.new(o!("local_client" => true)))?.into();

    let message_handler_local_client = MessageHandlerLocalClient::create(
        (local_client, data_transformer),
        args,
        log.new(o!("message_handler_local_client" => true)),
    )?;

    if let Some(search_args) = args.subcommand_matches("search") {
        let search_key = gnupg_key_loader
            .get_recipient_public_key(String::from(search_args.value_of("key_id").unwrap()))?;
        let response =
            message_handler_local_client.send(RequestPayload::SearchRequest(search_key))?;

        match response {
            ResponsePayload::SearchResponse(Some(ref node)) => {
                match search_args.value_of("print").unwrap() {
                    "all" => println!("{}", node.address),
                    "ip" => println!("{}", node.address.to_socket_addr().ip()),
                    "port" => println!("{}", node.address.port),
                    _ => panic!("Impossible print value"),
                };
                Ok(())
            }
            ResponsePayload::SearchResponse(None) => {
                println!("Search unsuccessful.");
                Ok(())
            }
            _ => Err(InternalError::private(ErrorKind::ParseError(
                "Unrecognized response".into(),
            ))),
        }
    } else if let Some(connect_args) = args.subcommand_matches("connect") {
        // Get node from arguments
        let node_key = gnupg_key_loader
            .get_recipient_public_key(String::from(connect_args.value_of("key_id").unwrap()))?;
        let node_address = Address::from_string(connect_args.value_of("address").unwrap())?;
        let node = Node::new(node_address, node_key);

        let response = message_handler_local_client.send(RequestPayload::ConnectRequest(node))?;

        match response {
            ResponsePayload::ConnectResponse() => {
                println!("Connect successful");
                Ok(())
            }
            _ => Err(InternalError::private(ErrorKind::ParseError(
                "Unrecognized response".into(),
            ))),
        }
    } else if let Some(_) = args.subcommand_matches("list-neighbours") {
        let response = message_handler_local_client.send(RequestPayload::ListNeighboursRequest())?;

        match response {
            ResponsePayload::ListNeighboursResponse(ref neighbours) => {
                println!("Found neighbours:");
                for n in neighbours {
                    println!("{}", n);
                }
                Ok(())
            }
            _ => Err(InternalError::private(ErrorKind::ParseError(
                "Unrecognized response".into(),
            ))),
        }
    } else {
        Err(InternalError::public(
            "No commmand given",
            ApiErrorType::Configuration,
        ))
    }
}
