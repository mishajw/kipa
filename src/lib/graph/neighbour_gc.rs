//! Keep track of whether neighbours are "alive" (i.e. responding to requests),
//! and if they are not, then remove them from our neighbours.

use periodic;
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use api::Node;
use api::RequestPayload;
use graph::NeighboursStore;
use message_handler::MessageHandlerClient;
use slog::Logger;

/// Default frequency for checking "dead" neighbours
pub const DEFAULT_FREQUENCY_SEC: &str = "30";

/// Default number of retries to carry out when a failed check happens
pub const DEFAULT_NUM_RETRIES: &str = "3";

/// Default enabled flag for garbage collection
pub const DEFAULT_ENABLED: &str = "true";

/// The connection status of a neighbour
struct NeighbourStatus {
    /// How many more retries we can have before discarding the neighbour
    consecutive_failed: u32,
    /// How many iterations to wait before retrying
    retry_cooloff: u32,
}

impl NeighbourStatus {
    pub fn new() -> Self {
        NeighbourStatus {
            consecutive_failed: 0,
            retry_cooloff: 0,
        }
    }
}

pub struct NeighbourGc {
    /// Map from key ID to neighbour status
    neighbour_statuses: HashMap<String, NeighbourStatus>,
    store: Arc<NeighboursStore>,
    message_handler_client: Arc<MessageHandlerClient>,
    num_retries: u32,
    log: Logger,
}

impl NeighbourGc {
    pub fn new(
        store: Arc<NeighboursStore>,
        message_handler_client: Arc<MessageHandlerClient>,
        num_retries: u32,
        log: Logger,
    ) -> Self
    {
        NeighbourGc {
            neighbour_statuses: HashMap::new(),
            store,
            message_handler_client,
            num_retries,
            log,
        }
    }

    pub fn start(self, frequency: Duration) {
        remotery_scope!("neighbour_gc_start");
        let mutex_self = Arc::new(Mutex::new(self));
        let mut planner = periodic::Planner::new();
        planner.add(
            move || mutex_self.lock().unwrap().check_all_neighbours(),
            RandomDurationIter::new(frequency),
        );
        planner.start();
    }

    fn check_all_neighbours(&mut self) {
        remotery_scope!("neighbour_gc_check_all_neighbours");
        info!(self.log, "Checking all neighbours for liveness");

        // Update the statuses of all neighbours
        let mut key_ids = HashSet::new();
        for neighbour in self.store.get_all() {
            key_ids.insert(neighbour.key.key_id.clone());
            let message_handler_client = self.message_handler_client.clone();
            let log = self.log.clone();
            let status = self
                .neighbour_statuses
                .entry(neighbour.key.key_id.clone())
                .or_insert_with(|| NeighbourStatus::new());
            Self::update_neighbour_status(
                &neighbour,
                status,
                message_handler_client,
                log,
            )
        }

        // Remove unresponsive neighbours and clean up unused statuses
        let num_retries = self.num_retries;
        let store = self.store.clone();
        self.neighbour_statuses.retain(|key_id, status| {
            // If the status has too many consecutive failures, remove it from
            // status and neigbours
            if status.consecutive_failed >= num_retries {
                store.remove_by_key_id(key_id);
                return false;
            }
            // If the status isn't in the neighbours list, remove it from
            // statuses
            key_ids.contains(key_id)
        });
    }

    fn update_neighbour_status(
        neighbour: &Node,
        status: &mut NeighbourStatus,
        message_handler_client: Arc<MessageHandlerClient>,
        log: Logger,
    )
    {
        remotery_scope!("neighbour_gc_check_neighbour");

        debug!(
            log, "Checking liveness of neighbour";
            "neighbour" => %neighbour,
            "consecutive_failed" => status.consecutive_failed,
            "retry_cooloff" => status.retry_cooloff);

        // If retry cooloff is still effective, do nothing but decrement the
        // cooloff
        if status.retry_cooloff > 0 {
            status.retry_cooloff -= 1;
            return;
        }

        // Try to connect to the neighbour
        let response = message_handler_client.send_private_message(
            &neighbour,
            RequestPayload::VerifyRequest(),
            Duration::from_secs(3),
        );

        // If the request was successful, reset its status
        if response.is_ok() {
            status.consecutive_failed = 0;
            status.retry_cooloff = 0;
            return;
        }

        // If the request was unsucessful, increment the consecutive fails, and
        // set the cooloff proportionately
        status.consecutive_failed += 1;
        status.retry_cooloff = status.consecutive_failed;
        info!(
            log, "Failed to verify neighbour, updated status";
            "neighbour" => %neighbour,
            "consecutive_failed" => status.consecutive_failed,
            "retry_cooloff" => status.retry_cooloff);
    }
}

struct RandomDurationIter {
    now: Instant,
    average_duration_millis: u64,
}

impl RandomDurationIter {
    fn new(average_duration: Duration) -> Self {
        RandomDurationIter {
            now: Instant::now(),
            average_duration_millis: average_duration.as_secs() * 1000
                + average_duration.subsec_millis() as u64,
        }
    }
}

impl Iterator for RandomDurationIter {
    type Item = Instant;
    fn next(&mut self) -> Option<Self::Item> {
        let mut rng = thread_rng();
        let duration_multiplier: f32 = rng.gen_range(0.5, 2.0);
        let duration_millis: u64 = ((self.average_duration_millis as f32)
            * duration_multiplier) as u64;
        let duration = Duration::from_millis(duration_millis);
        self.now += duration;
        Some(self.now)
    }
}

impl periodic::IntoInstantIter for RandomDurationIter {
    type IterType = Self;
    fn into_instant_iter(self) -> Self::IterType { self }
}
