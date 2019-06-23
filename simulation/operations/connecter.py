import logging

from simulation.backends import Backend
from simulation.networks import Network, ConnectType, NodeId
from simulation.operations import ensure_all_alive

log = logging.getLogger(__name__)


def connect_network(network: Network, backend: Backend) -> None:
    log.info("Connecting network together")

    ensure_all_alive(network, backend)

    for i in range(network.num_connects):
        log.info(f"Performing connection {i + 1}/{network.num_connects}")
        if network.connect_type == ConnectType.CYCLICAL:
            __connect_nodes_cyclically(network, backend)
        elif network.connect_type == ConnectType.ROOTED:
            [root_id] = network.random_ids(1)
            __connect_nodes_to_one(network, backend, root_id)


def __connect_node(
    backend: Backend, connector_id: NodeId, connect_to_id: NodeId
) -> None:
    log.debug(f"Connecting {connector_id} to {connect_to_id}")
    output = backend.run_command(
        connector_id,
        [
            "connect",
            "--key-id",
            connect_to_id.key_id,
            "--address",
            backend.get_ip_address(connect_to_id),
        ],
    )
    if output is None:
        log.error(f"Connection failed")
    elif "Connect successful" not in output:
        log.error(f"Connection failed with output: {output}")


def __connect_nodes_to_one(
    network: Network, backend: Backend, root_id: NodeId
) -> None:
    log.debug(f"Connecting all nodes to {root_id}")
    for node_id in network.ids():
        if node_id == root_id:
            continue
        __connect_node(backend, node_id, root_id)


def __connect_nodes_cyclically(network: Network, backend: Backend) -> None:
    log.debug("Connecting nodes cyclically")
    ids = network.ids()
    for k1, k2 in zip(ids, ids[1:] + ids[:1]):
        __connect_node(backend, k1, k2)
