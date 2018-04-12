import logging

from benchmarks.networks import Network

log = logging.getLogger(__name__)


def connect_node(
        network: Network, connector_key_id: str, connectee_key_id: str) -> None:
    log.debug(f"Connecting {connector_key_id} to {connectee_key_id}")
    output = network.exec_command(connector_key_id, [
        "/root/kipa_cli",
        "connect",
        "--key-id", connectee_key_id,
        "--address", network.get_address(connectee_key_id)])
    assert "Connect successful" in output


def connect_nodes_to_one(network: Network, root_key_id: str) -> None:
    log.debug(f"Connecting all nodes to {root_key_id}")
    for k in network.get_all_keys():
        if k == root_key_id:
            continue
        connect_node(network, k, root_key_id)


def connect_nodes_cyclically(network: Network) -> None:
    log.debug("Connecting nodes cyclically")
    keys = network.get_all_keys()
    for k1, k2 in zip(keys, keys[1:] + keys[:1]):
        connect_node(network, k1, k2)
