import random
from abc import ABC, abstractmethod
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


class Random(NeighbourStrategy):
    """
    Randomly selects neighbours.
    """

    def select_neighbours(
        self,
        local: KeySpace,
        current_neighbours: FrozenSet[Node],
        new_neighbour: Node,
    ) -> FrozenSet[Node]:
        all_nodes = current_neighbours.union([new_neighbour])
        return frozenset(random.sample(all_nodes, len(current_neighbours)))


class Closest(NeighbourStrategy):
    """
    Selects the closest neighbours.
    """

    def select_neighbours(
        self,
        local: KeySpace,
        current_neighbours: FrozenSet[Node],
        new_neighbour: Node,
    ) -> FrozenSet[Node]:
        closest = list(
            sorted(
                [*current_neighbours, new_neighbour],
                key=lambda n: local.distance(n.key_space),
            )
        )
        return frozenset(closest[: len(current_neighbours)])


class ClosestUnwrapped(NeighbourStrategy):
    """
    Selects the closest neighbours in unwrapped key space.
    """

    def select_neighbours(
        self,
        local: KeySpace,
        current_neighbours: FrozenSet[Node],
        new_neighbour: Node,
    ) -> FrozenSet[Node]:
        closest = list(
            sorted(
                [*current_neighbours, new_neighbour],
                key=lambda n: local.distance(n.key_space, wrapped=False),
            )
        )
        return frozenset(closest[: len(current_neighbours)])
