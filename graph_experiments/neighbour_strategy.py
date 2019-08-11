import random
from abc import ABC, abstractmethod
from itertools import islice
from typing import FrozenSet

from graph_experiments import Node, Args, KeySpace, Distance


class NeighbourStrategy(ABC):
    @classmethod
    def get(cls, name: str) -> "NeighbourStrategy":
        if name == "random":
            return Random()
        elif name == "closest":
            return Closest()
        elif name == "closest-random":
            return ClosestRandom()
        elif name == "closest-gaussian":
            return ClosestRandom()
        else:
            raise AssertionError(f"Unknown neighbour strategy: {name}")

    def apply(
        self,
        node: Node,
        new_neighbour: Node,
        all_nodes: FrozenSet[Node],
        distance: Distance,
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
            node.key_space, current_neighbours, new_neighbour, distance, args
        )
        return node.with_neighbours(frozenset(n.index for n in new_neighbours))

    @abstractmethod
    def select_neighbours(
        self,
        local: KeySpace,
        current_neighbours: FrozenSet[Node],
        new_neighbour: Node,
        distance: Distance,
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
        distance: Distance,
        args: Args,
    ) -> FrozenSet[Node]:
        sorted_by_metric = sorted(
            [*current_neighbours, new_neighbour],
            key=lambda n: self.metric(local, n, distance, args),
        )
        sorted_by_metric = islice(sorted_by_metric, len(current_neighbours))
        return frozenset(sorted_by_metric)

    @abstractmethod
    def metric(
        self, local: KeySpace, node: Node, distance: Distance, args: Args
    ) -> float:
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
        distance: Distance,
        args: Args,
    ) -> FrozenSet[Node]:
        all_nodes = current_neighbours.union([new_neighbour])
        sorted_by_metric = sorted(
            [*current_neighbours, new_neighbour],
            key=lambda n: self.metric(
                local, n, all_nodes.difference([n]), distance, args
            ),
        )
        sorted_by_metric = islice(sorted_by_metric, len(current_neighbours))
        return frozenset(sorted_by_metric)

    @abstractmethod
    def metric(
        self,
        local: KeySpace,
        node: Node,
        others: FrozenSet[Node],
        distance: Distance,
        args: Args,
    ) -> float:
        raise NotImplementedError()


class Random(MetricNeighbourStrategy):
    """
    Randomly selects neighbours.
    """

    def metric(
        self, local: KeySpace, node: Node, distance: Distance, args: Args
    ) -> float:
        return random.random()


class Closest(MetricNeighbourStrategy):
    """
    Selects the closest neighbours.
    """

    def metric(
        self, local: KeySpace, node: Node, distance: Distance, args: Args
    ) -> float:
        return distance.distance(local, node.key_space)


class ClosestRandom(MetricNeighbourStrategy):
    """
    Selects the closest neighbours with some randomness.
    """

    def metric(
        self, local: KeySpace, node: Node, distance: Distance, args: Args
    ) -> float:
        return (
            distance.distance(local, node.key_space)
            + random.random() * distance.max_distance(args) * 0.1
        )


class ClosestGaussian(MetricNeighbourStrategy):
    """
    Selects the closest neighbours with gaussian probability.
    """

    def metric(
        self, local: KeySpace, node: Node, distance: Distance, args: Args
    ) -> float:
        distance_to_node = distance.distance(local, node.key_space)
        gauss = abs(random.gauss(0, distance.max_distance(args)))
        return 1 if gauss > distance_to_node else 0
