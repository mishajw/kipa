import logging
import time

from simulation.backends import Backend
from simulation.backends.backend import CliCommand
from simulation.networks import Network

log = logging.getLogger(__name__)

NUM_ATTEMPTS = 10
ATTEMPT_WAIT_SECS = 10


def ensure_all_alive(network: Network, backend: Backend) -> None:
    log.debug("Ensuring all nodes in the network are alive")

    ids = network.ids()

    for attempt in range(NUM_ATTEMPTS):
        commands = [CliCommand(node_id, ["list-neighbours"]) for node_id in ids]
        results = backend.run_commands(commands)
        ids = [
            result.command.node_id
            for result in results
            if not result.successful()
        ]
        if not ids:
            log.debug("All nodes ensured alive")
            return
        log.debug(
            "At attempt %d/%d, still %d nodes not responding. Sleeping %d seconds.",
            attempt + 1,
            NUM_ATTEMPTS,
            len(ids),
            ATTEMPT_WAIT_SECS,
        )
        time.sleep(ATTEMPT_WAIT_SECS)

    raise AssertionError(
        f"{len(ids)} nodes still didn't reply after all attempts"
    )
