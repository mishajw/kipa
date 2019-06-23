from typing import NamedTuple, List, FrozenSet, Dict, Any

from simulation.key_creator import KeyCreator


class NodeId(NamedTuple):
    # TODO: What if we test multiple nodes with the same key?
    key_id: str

    def __str__(self) -> str:
        return self.key_id


class Node(NamedTuple):
    id: NodeId
    daemon_args: Dict[str, Any]
    additional_features: FrozenSet[str]
    clear_default_features: bool
    disconnect_before_tests: bool
    debug: bool

    def key_id(self) -> str:
        return self.id.key_id

    @classmethod
    def from_config(cls, config: dict, key_creator: KeyCreator) -> List["Node"]:
        return [
            Node(
                NodeId(key_creator.get_key_id()),
                config.get("daemon_args", {}),
                frozenset(config.get("additional_features", [])),
                config.get("clear_default_features", False),
                config.get("disconnect_before_tests", False),
                config.get("debug", False),
            )
            for _ in range(config["size"])
        ]

    def replace(self, *_, **kwargs) -> "Node":
        return self._replace(**kwargs)
