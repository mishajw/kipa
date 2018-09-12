//! Keep track of whether neighbours are "alive" (i.e. responding to requests),
//! and if they are not, then remove them from our neighbours.

use periodic;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::RequestPayload;
use message_handler::MessageHandlerClient;
use node::Node;
use payload_handler::graph::NeighboursStore;
use slog::Logger;

/// Default frequency for checking "dead" neighbours
pub const DEFAULT_FREQUENCY_SEC: &str = "30";

/// Default number of retries to carry out when a failed check happens
pub const DEFAULT_NUM_RETRIES: &str = "3";

/// Default time gap between retries
pub const DEFAULT_RETRY_FREQUENCY_SEC: &str = "10";

/// Start the "garbage collector" for "dead" neighbours
///
/// By "dead", we mean neighbours that are no longer responding to requests.
/// We check every `frequency`, and if a neighbour does not respond, we retry
/// `num_retries` times before removing the neighbour.
pub fn start_gc(
    store: Arc<NeighboursStore>,
    message_handler_client: Arc<MessageHandlerClient>,
    frequency: Duration,
    num_retries: u32,
    retry_frequency: Duration,
    log: Logger,
)
{
    let planner = Arc::new(Mutex::new(periodic::Planner::new()));

    let check_all_planner = planner.clone();
    let check_all_neighbours_fn = move || {
        remotery_scope!("gc_check_all_neighbours");
        let neighbours = store.get_all();
        info!(
            log, "Checking all neighbours for liveness";
            "num_neighburs" => neighbours.len());
        for n in neighbours {
            check_neighbour_fn(
                n,
                num_retries,
                message_handler_client.clone(),
                store.clone(),
                retry_frequency,
                check_all_planner.clone(),
                log.clone(),
            );
        }
    };

    planner
        .lock()
        .unwrap()
        .add(check_all_neighbours_fn, periodic::Every::new(frequency));

    planner.lock().unwrap().start();
}

fn check_neighbour_fn(
    neighbour: Node,
    num_retires_left: u32,
    message_handler_client: Arc<MessageHandlerClient>,
    store: Arc<NeighboursStore>,
    retry_frequency: Duration,
    planner: Arc<Mutex<periodic::Planner>>,
    log: Logger,
)
{
    remotery_scope!("gc_check_neighbour");

    debug!(
        log, "Checking liveness of neighbour";
        "neighbour" => %neighbour, "num_retires_left" => num_retires_left);

    let response = message_handler_client.send_private_message(
        &neighbour,
        RequestPayload::VerifyRequest(),
        Duration::from_secs(3),
    );

    // If the response was unsucessful...
    if response.is_err() {
        if num_retires_left == 0 {
            // ...and if we have no more retries, remove the key as a neighbour
            info!(
                log,
                "Failed to verify neighbour, no retries left, removing \
                 neighbour";
                "neighbour" => %neighbour);
            store.remove_by_key(&neighbour.key);
        } else {
            // ...but if we have retries left, spawn a new planned thread to
            // check again in after `retry_frequency` has elapsed
            info!(
                log,
                "Failed to verify neighbour, retrying";
                "neighbour" => %neighbour,
                "num_retires_left" => num_retires_left);
            let planner_clone = planner.clone();
            planner.lock().unwrap().add(
                move || {
                    check_neighbour_fn(
                        neighbour.clone(),
                        num_retires_left - 1,
                        message_handler_client.clone(),
                        store.clone(),
                        retry_frequency,
                        planner_clone.clone(),
                        log.clone(),
                    )
                },
                periodic::After::new(retry_frequency),
            );
        }
    }

    // If the response was successful, exit the function without doing anything
}
