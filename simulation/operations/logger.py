import json
import logging
from pathlib import Path
from typing import NamedTuple, List, Dict

from simulation.backends import Backend
from simulation.networks import Network, NodeId
from simulation.operations import ensure_all_alive

log = logging.getLogger(__name__)


class NetworkLogs(NamedTuple):
    logs: Dict[NodeId, "NodeLogs"]

    def node_ids(self) -> List[NodeId]:
        return list(self.logs.keys())

    def get(self, node_id: NodeId) -> "NodeLogs":
        return self.logs[node_id]


class NodeLogs(NamedTuple):
    logs: List[dict]
    human_readable_logs: bytes


def get_logs(network: Network, backend: Backend) -> NetworkLogs:
    log.info("Getting logs")

    # This will call `list-neighbours` so that we have an up-to-date account
    # of each node's neighbours in the logs
    ensure_all_alive(network, backend)

    return NetworkLogs(
        {
            node.id: NodeLogs(
                backend.get_logs(node.id),
                backend.get_human_readable_logs(node.id),
            )
            for node in network.nodes
        }
    )


def write_logs(logs: NetworkLogs, output_directory: Path) -> None:
    log_directory = output_directory / "logs"
    if not log_directory.is_dir():
        log_directory.mkdir(parents=True)

    log.info(f"Saving logs to {log_directory}")
    for node_id in logs.node_ids():
        with open(str(log_directory / f"{node_id.key_id}.json"), "w") as file:
            json.dump(logs.get(node_id).logs, file)
        with open(str(log_directory / f"{node_id.key_id}.txt"), "wb") as file:
            file.write(logs.get(node_id).human_readable_logs)
