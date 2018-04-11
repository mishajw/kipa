#[macro_use] extern crate log;
extern crate clap;
extern crate error_chain;
extern crate kipa_lib;
extern crate simple_logger;

use kipa_lib::creators::*;
use kipa_lib::error::*;
use kipa_lib::gpg_key::GpgKeyHandler;
use kipa_lib::api::{Request, Response};

use error_chain::ChainedError;

fn main() {
    simple_logger::init().unwrap();
    info!("Starting CLI");

    let args = clap::App::new("kipa_daemon")
        .arg(clap::Arg::with_name("socket_path")
             .long("socket-path")
             .short("s")
             .help("Socket to communicate with daemon")
             .takes_value(true))
        .subcommand(
            clap::SubCommand::with_name("search")
                .about("Search for a node given a key")
                .arg(clap::Arg::with_name("key_id")
                     .long("key-id")
                     .short("k")
                     .help("The key to search for")
                     .takes_value(true)
                     .required(true)))
        .get_matches();

    if let Err(err) = message_daemon(&args) {
        println!("{}", err.display_chain().to_string());
    }
}

fn message_daemon(args: &clap::ArgMatches) -> Result<()> {
    let mut gpg_key_handler = GpgKeyHandler::new()?;

    let data_transformer = create_data_transformer()?;

    let local_send_server = create_local_send_server(
        data_transformer.clone(), args)?;

    if let Some(search_args) = args.subcommand_matches("search") {
        let search_key = gpg_key_handler.get_key(
            String::from(search_args.value_of("key_id").unwrap()))?;
        let response = local_send_server.receive(
            &Request::SearchRequest(search_key))?;

        match response {
            Response::SearchResponse(Some(ref node)) => {
                println!("Search success: {}.", node);
                Ok(())
            }
            Response::SearchResponse(None) => {
                println!("Search unsuccessful.");
                Ok(())
            }
            _ => Err(ErrorKind::ParseError(
                "Unrecognized response".into()).into())
        }
    } else {
        Err(ErrorKind::ParseError("No commmand given".into()).into())
    }
}

