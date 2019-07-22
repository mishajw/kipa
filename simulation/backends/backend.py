from typing import List, Dict, Optional, NamedTuple

from simulation import Build
from simulation.networks import Network, NodeId


class Backend:
    def initialize_network(
        self, network: Network, node_builds: Dict[NodeId, Build]
    ) -> None:
        raise NotImplementedError()

    def get_ip_address(self, node_id: NodeId) -> str:
        raise NotImplementedError()

    def run_commands(
        self, commands: List["CliCommand"]
    ) -> List["CliCommandResult"]:
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


class CliCommand(NamedTuple):
    node_id: NodeId
    args: List[str]


class CliCommandResult(NamedTuple):
    command: CliCommand
    stdout: Optional[str]
    cli_logs: Optional[List[dict]]
    duration_sec: float

    @classmethod
    def failed(cls, command: CliCommand) -> "CliCommandResult":
        return CliCommandResult(command, None, None, 0)

    def successful(self) -> bool:
        return self.stdout is not None
