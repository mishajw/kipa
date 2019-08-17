from abc import ABC, abstractmethod
from typing import FrozenSet

from graph_experiments import Node, NeighbourStrategy


class TestStrategy(ABC):
    @classmethod
    def get(cls, name: str) -> "TestStrategy":
        if name == "all-knowing":
            return AllKnowing()
        else:
            raise AssertionError(f"Unknown test strategy: {name}")

    @abstractmethod
    def apply(
        self, nodes: FrozenSet[Node], neighbour_strategy: NeighbourStrategy
    ) -> FrozenSet[Node]:
        """
        Connects the input nodes together in some way, using a
        `NeighbourStrategy`.

        `Node.neighbours` must not be modified by this method - this method
        should only chose which new nodes to expose to the `NeighbourStrategy`.
        """
        raise NotImplementedError()


class AllKnowing(TestStrategy):
    """
    Gives every node the choice of every other node. The ideal scenario for a
    `NeighbourStrategy`.
    """

    def apply(
        self, nodes: FrozenSet[Node], neighbour_strategy: NeighbourStrategy
    ) -> FrozenSet[Node]:
        return frozenset(
            neighbour_strategy.apply(
                node, frozenset(n for n in nodes if n is not node), nodes
            )
            for node in nodes
        )
