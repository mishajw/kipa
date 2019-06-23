import logging
import time

from simulation.backends import Backend
from simulation.networks import Network, NodeId

log = logging.getLogger(__name__)


def ensure_all_alive(network: Network, backend: Backend) -> None:
    log.debug("Ensuring all nodes in the network are alive")
    for node_id in network.ids():
        __ensure_alive(backend, node_id)


def __ensure_alive(backend: Backend, node_id: NodeId, attempts=3) -> None:
    if backend.run_command(node_id, ["list-neighbours"]) is not None:
        return

    assert (
        attempts > 1
    ), f"Three failed attempts to list-neighbours on node {node_id}"

    log.info(
        f"Node {node_id} did not respond to `list-neighbours`, "
        "sleeping for one second and trying again"
    )
    time.sleep(1)
    __ensure_alive(backend, node_id, attempts - 1)
