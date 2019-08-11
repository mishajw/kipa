import random
from abc import ABC, abstractmethod
from itertools import islice
from typing import FrozenSet

from graph_experiments import Node, Args, KeySpace


class NeighbourStrategy(ABC):
    @classmethod
    def get(cls, name: str) -> "NeighbourStrategy":
        if name == "random":
            return Random()
        elif name == "closest":
            return Closest()
        elif name == "closest-unwrapped":
            return ClosestUnwrapped()
        elif name == "closest-random":
            return ClosestRandom()
        else:
            raise AssertionError(f"Unknown neighbour strategy: {name}")

    def apply(
        self,
        node: Node,
        new_neighbour: Node,
        all_nodes: FrozenSet[Node],
        args: Args,
    ) -> Node:
        """
        Applies the neighbour selection strategy to `node` with a
        potential `new_neighbour`.
        """
        assert len(node.neighbours) <= args.max_neighbours
        if len(node.neighbours) < args.max_neighbours:
            return node.with_neighbours(
                node.neighbours.union([new_neighbour.index])
            )
        current_neighbours = frozenset(
            n for n in all_nodes if n.index in node.neighbours
        )
        new_neighbours = self.select_neighbours(
            node.key_space, current_neighbours, new_neighbour, args
        )
        return node.with_neighbours(frozenset(n.index for n in new_neighbours))

    @abstractmethod
    def select_neighbours(
        self,
        local: KeySpace,
        current_neighbours: FrozenSet[Node],
        new_neighbour: Node,
        args: Args,
    ) -> FrozenSet[Node]:
        """
        Selects which neighbours to keep out of the current and a new one.

        `local` is the key space of the node that is selecting the neighbours.

        The number of returned neighbours must always be equal to the number of
        `node.neighbours`. If we have less neighbours than the max, this
        function isn't called and any new neighbours are automatically added.
        """
        raise NotImplementedError()


class MetricNeighbourStrategy(NeighbourStrategy, ABC):
    """
    Strategy that picks nodes that minimize a given metric.
    """

    def select_neighbours(
        self,
        local: KeySpace,
        current_neighbours: FrozenSet[Node],
        new_neighbour: Node,
        args: Args,
    ) -> FrozenSet[Node]:
        sorted_by_metric = sorted(
            [*current_neighbours, new_neighbour],
            key=lambda n: self.metric(local, n, args),
        )
        sorted_by_metric = islice(sorted_by_metric, len(current_neighbours))
        return frozenset(sorted_by_metric)

    @abstractmethod
    def metric(self, local: KeySpace, other: Node, args: Args) -> float:
        raise NotImplementedError()


class Random(MetricNeighbourStrategy):
    """
    Randomly selects neighbours.
    """

    def metric(self, local: KeySpace, other: Node, args: Args) -> float:
        return random.random()


class Closest(MetricNeighbourStrategy):
    """
    Selects the closest neighbours.
    """

    def metric(self, local: KeySpace, other: Node, args: Args) -> float:
        return local.distance(other.key_space)


class ClosestUnwrapped(MetricNeighbourStrategy):
    """
    Selects the closest neighbours in unwrapped key space.
    """

    def metric(self, local: KeySpace, other: Node, args: Args) -> float:
        return local.distance(other.key_space, wrapped=False)


class ClosestRandom(MetricNeighbourStrategy):
    """
    Selects the closest neighbours with some randomness.
    """

    def metric(self, local: KeySpace, other: Node, args: Args) -> float:
        return local.distance(
            other.key_space
        ) + random.random() * KeySpace.max_distance(args)
