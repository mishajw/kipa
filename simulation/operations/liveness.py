import logging
import time

from simulation.backends import Backend
from simulation.backends.backend import CliCommand
from simulation.networks import Network

log = logging.getLogger(__name__)


def ensure_all_alive(network: Network, backend: Backend) -> None:
    log.debug("Ensuring all nodes in the network are alive")

    ids = network.ids()

    for attempt in range(3):
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
            "At attempt %d, still %d nodes not responding. Sleeping 1s",
            attempt + 1,
            len(ids),
        )
        time.sleep(1)

    raise AssertionError(
        f"{len(ids)} nodes still didn't reply after all attempts"
    )
