from typing import List, Dict, Optional

from simulation import Build
from simulation.networks import Network, NodeId


class Backend:
    def initialize_network(
        self, network: Network, node_builds: Dict[NodeId, Build]
    ) -> None:
        raise NotImplementedError()

    def get_ip_address(self, node_id: NodeId) -> str:
        raise NotImplementedError()

    def run_command(
        self, node_id: NodeId, arguments: List[str]
    ) -> Optional[str]:
        raise NotImplementedError()

    def stop_networking(self, node_id: NodeId):
        raise NotImplementedError()

    def get_logs(self, node_id: NodeId) -> List[dict]:
        raise NotImplementedError()

    def get_cli_logs(self, node_id: NodeId) -> List[dict]:
        raise NotImplementedError()

    def get_human_readable_logs(self, node_id: NodeId) -> bytes:
        raise NotImplementedError()

    def clean(self) -> None:
        raise NotImplementedError()
