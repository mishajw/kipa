import random
from abc import ABC, abstractmethod
from itertools import islice
from typing import FrozenSet

from graph_experiments import Node, Args, KeySpace, Distance


class NeighbourStrategy(ABC):
    def __init__(self, distance: Distance, args: Args) -> None:
        self.distance = distance
        self.args = args

    @classmethod
    def get(
        cls, name: str, distance: Distance, args: Args
    ) -> "NeighbourStrategy":
        if name == "random":
            return Random(distance, args)
        elif name == "closest":
            return Closest(distance, args)
        elif name == "closest-random":
            return ClosestRandom(distance, args)
        elif name == "closest-gaussian":
            return ClosestRandom(distance, args)
        else:
            raise AssertionError(f"Unknown neighbour strategy: {name}")

    def apply(
        self, node: Node, new_neighbour: Node, all_nodes: FrozenSet[Node]
    ) -> Node:
        """
        Applies the neighbour selection strategy to `node` with a
        potential `new_neighbour`.
        """
        assert len(node.neighbours) <= self.args.max_neighbours
        if len(node.neighbours) < self.args.max_neighbours:
            return node.with_neighbours(
                node.neighbours.union([new_neighbour.index])
            )
        current_neighbours = frozenset(
            n for n in all_nodes if n.index in node.neighbours
        )
        new_neighbours = self.select_neighbours(
            node.key_space, current_neighbours, new_neighbour
        )
        return node.with_neighbours(frozenset(n.index for n in new_neighbours))

    @abstractmethod
    def select_neighbours(
        self,
        local: KeySpace,
        current_neighbours: FrozenSet[Node],
        new_neighbour: Node,
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
    ) -> FrozenSet[Node]:
        sorted_by_metric = sorted(
            [*current_neighbours, new_neighbour],
            key=lambda n: self.metric(local, n),
        )
        sorted_by_metric = islice(sorted_by_metric, len(current_neighbours))
        return frozenset(sorted_by_metric)

    @abstractmethod
    def metric(self, local: KeySpace, node: Node) -> float:
        raise NotImplementedError()


class ContextMetricNeighbourStrategy(NeighbourStrategy, ABC):
    """
    Strategy that picks nodes that minimize a given metric. The metric
    calculation also takes the context of the other potential nodes.
    """

    def select_neighbours(
        self,
        local: KeySpace,
        current_neighbours: FrozenSet[Node],
        new_neighbour: Node,
    ) -> FrozenSet[Node]:
        all_nodes = current_neighbours.union([new_neighbour])
        sorted_by_metric = sorted(
            [*current_neighbours, new_neighbour],
            key=lambda n: self.metric(local, n, all_nodes.difference([n])),
        )
        sorted_by_metric = islice(sorted_by_metric, len(current_neighbours))
        return frozenset(sorted_by_metric)

    @abstractmethod
    def metric(
        self, local: KeySpace, node: Node, others: FrozenSet[Node]
    ) -> float:
        raise NotImplementedError()


class Random(MetricNeighbourStrategy):
    """
    Randomly selects neighbours.
    """

    def metric(self, local: KeySpace, node: Node) -> float:
        return random.random()


class Closest(MetricNeighbourStrategy):
    """
    Selects the closest neighbours.
    """

    def metric(self, local: KeySpace, node: Node) -> float:
        return self.distance.distance(local, node.key_space)


class ClosestRandom(MetricNeighbourStrategy):
    """
    Selects the closest neighbours with some randomness.
    """

    def metric(self, local: KeySpace, node: Node) -> float:
        return (
            self.distance.distance(local, node.key_space)
            + random.random() * self.distance.max_distance() * 0.1
        )


class ClosestGaussian(MetricNeighbourStrategy):
    """
    Selects the closest neighbours with gaussian probability.
    """

    def metric(self, local: KeySpace, node: Node) -> float:
        distance_to_node = self.distance.distance(local, node.key_space)
        gauss = abs(random.gauss(0, self.distance.max_distance()))
        return 1 if gauss > distance_to_node else 0
