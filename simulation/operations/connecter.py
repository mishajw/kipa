import logging

from simulation.backends import Backend
from simulation.backends.backend import CliCommand
from simulation.networks import Network, ConnectType
from simulation.operations import ensure_all_alive

log = logging.getLogger(__name__)


def connect_network(network: Network, backend: Backend) -> None:
    log.info("Connecting network together")

    ensure_all_alive(network, backend)

    for i in range(network.num_connects):
        log.info(f"Performing connection {i + 1}/{network.num_connects}")

        ids = network.ids()
        if network.connect_type == ConnectType.CYCLICAL:
            connections = list(zip(ids[:-1], ids[1:]))
        elif network.connect_type == ConnectType.ROOTED:
            [root_id] = network.random_ids(1)
            connections = [(i, root_id) for i in ids]
        else:
            raise AssertionError()
        commands = [
            CliCommand(
                a,
                [
                    "connect",
                    "--key-id",
                    b.key_id,
                    "--address",
                    backend.get_ip_address(b),
                ],
            )
            for a, b in connections
        ]
        results = backend.run_commands(commands)
        num_failed = sum(result.successful() for result in results)
        log.info("Out of %d connections, %d failed", len(ids), num_failed)
