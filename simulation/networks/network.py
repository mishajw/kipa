import logging
import multiprocessing
import random
from enum import Enum
from typing import List, NamedTuple, Optional, Callable

from simulation.key_creator import KeyCreator
from simulation.networks import Node, NodeId

log = logging.getLogger(__name__)


class Network(NamedTuple):
    nodes: List[Node]
    ipv6: bool

    num_connects: int
    num_searches: int
    connect_type: "ConnectType"
    connection_quality: Optional["ConnectionQuality"]
    num_threads: int

    # TODO: Try to not use KeyCreator here
    @classmethod
    def from_config(cls, config: dict, key_creator: KeyCreator) -> "Network":
        nodes = [
            node
            for group_config in config["groups"]
            for node in Node.from_config(group_config, key_creator)
        ]
        connection_quality = (
            ConnectionQuality.from_config(
                config.get("connection_quality", "cyclical")
            )
            if "connection_quality" in config
            else None
        )
        return Network(
            nodes,
            config.get("ipv6", False),
            config.get("num_connects", 1),
            config.get("num_searches", 50),
            ConnectType.from_str(config.get("connect_type", "cyclical")),
            connection_quality,
            config.get("num_threads", multiprocessing.cpu_count()),
        )

    def ids(self) -> List[NodeId]:
        return [node.id for node in self.nodes]

    def random_ids(self, num: int) -> List[NodeId]:
        return random.choices(self.ids(), k=num)

    def map_nodes(self, fn: Callable[[Node], Node]) -> "Network":
        return self._replace(nodes=list(map(fn, self.nodes)))

    def replace(self, *_, **kwargs) -> "Network":
        return self._replace(**kwargs)


class ConnectType(Enum):
    CYCLICAL = 0
    ROOTED = 1

    @classmethod
    def from_str(cls, s: str) -> "ConnectType":
        if s == "cyclical":
            return ConnectType.CYCLICAL
        elif s == "rooted":
            return ConnectType.ROOTED
        else:
            raise ValueError(f"Unrecognized `ConnectType`: {s}")

    def to_str(self) -> str:
        if self == ConnectType.CYCLICAL:
            return "cyclical"
        elif self == ConnectType.ROOTED:
            return "rooted"
        else:
            raise ValueError(f"Unhandled `ConnectType`: {self}")


class ConnectionQuality:
    def __init__(self, loss: float, delay: float, rate: float) -> None:
        self.loss = loss
        self.delay = delay
        self.rate = rate

    @classmethod
    def from_config(cls, config: dict) -> "ConnectionQuality":
        return cls(
            config.get("loss", 0), config.get("delay", 0), config.get("rate", 0)
        )
