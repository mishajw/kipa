"""
Benchmarking set up for testing neighbour selection algorithms for use in KIPA.

This is a simplified set up of the code in src/graph. This allows for quick
prototyping and testing of new algorithms in a perfect environment.

To try a new neighbour selection algorithm, implement the `NeighbourStrategy`
interface and choose `TestStrategy` to test it against.

For example, to benchmark the "closest neighbours" strategy against the
"all-knowing" test strategy, run:

  python -m graph_experiments \
    --neighbour-strategy closest \
    --test-strategy all-knowing
"""

import random
from abc import ABC, abstractmethod
from argparse import ArgumentParser
from itertools import permutations
from typing import NamedTuple, Set, Optional, FrozenSet, Tuple

KEY_SPACE_LOWER = -1
KEY_SPACE_UPPER = 1


def main():
    parser = ArgumentParser("graph_experiments")
    parser.add_argument("--neighbour-strategy", type=str, required=True)
    parser.add_argument("--test-strategy", type=str, required=True)
    parser.add_argument("--num-nodes", type=int, default=100)
    parser.add_argument("--key-space-dimensions", type=int, default=2)
    parser.add_argument("--max-neighbours", type=int, default=10)
    args = parser.parse_args()

    neighbour_strategy = NeighbourStrategy.get(args.neighbour_strategy)
    test_strategy = TestStrategy.get(args.test_strategy)

    nodes = frozenset(
        Node(i, KeySpace.random(args.key_space_dimensions))
        for i in range(args.num_nodes)
    )
    nodes = test_strategy.connect_nodes(
        nodes, neighbour_strategy, args.max_neighbours
    )
    results = ConnectednessResults.test(nodes)
    print(results)


class Node(NamedTuple):
    index: int
    key_space: "KeySpace"
    # We store the index of nodes rather than the nodes themselves. This fixes
    # issues with the `neighbours` nodes becoming out of date as we change their
    # neighbours.
    neighbours: FrozenSet[int] = frozenset()

    def with_neighbours(self, neighbours: FrozenSet[int]) -> "Node":
        return Node(self.index, self.key_space, neighbours)


class KeySpace(NamedTuple):
    position: Tuple[float]

    @classmethod
    def random(cls, key_space_dimensions: int) -> "KeySpace":
        return KeySpace(
            tuple(
                float(random.uniform(KEY_SPACE_LOWER, KEY_SPACE_UPPER))
                for _ in range(key_space_dimensions)
            )
        )

    def distance(self, other: "KeySpace") -> float:
        assert len(self.position) == len(other.position)
        return (
            sum((a - b) ** 2 for a, b in zip(self.position, other.position))
            ** 0.5
        )


class NeighbourStrategy(ABC):
    @classmethod
    def get(cls, name: str) -> "NeighbourStrategy":
        if name == "random":
            return RandomNeighbourStrategy()
        elif name == "closest":
            return ClosestNeighbourStrategy()
        else:
            raise AssertionError(f"Unknown neighbour strategy: {name}")

    def apply(
        self,
        node: Node,
        new_neighbour: Node,
        max_neighbours: int,
        all_nodes: FrozenSet[Node],
    ) -> Node:
        """
        Applies the neighbour selection strategy to `node` with a
        potential `new_neighbour`.
        """
        assert len(node.neighbours) <= max_neighbours
        if len(node.neighbours) < max_neighbours:
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


class TestStrategy(ABC):
    @classmethod
    def get(cls, name: str) -> "TestStrategy":
        if name == "all-knowing":
            return AllKnowingTestStrategy()
        else:
            raise AssertionError(f"Unknown test strategy: {name}")

    @abstractmethod
    def connect_nodes(
        self,
        nodes: FrozenSet[Node],
        neighbour_strategy: NeighbourStrategy,
        max_neighbours: int,
    ) -> FrozenSet[Node]:
        """
        Connects the input nodes together in some way, using a
        `NeighbourStrategy`.

        `Node.neighbours` must not be modified by this method - this method
        should only chose which new nodes to expose to the `NeighbourStrategy`.
        """
        raise NotImplementedError()


class ConnectednessResults(NamedTuple):
    fully_connected: bool
    mean_num_requests: float

    @classmethod
    def test(cls, nodes: FrozenSet[Node]) -> "ConnectednessResults":
        results = [
            cls.__search(from_node, to_node, nodes)
            for from_node, to_node in permutations(nodes, 2)
        ]
        results_success = list(filter(None, results))
        mean_num_requests = (
            sum(results_success) / len(results_success)
            if results_success
            else 0
        )
        return ConnectednessResults(None not in results, mean_num_requests)

    @staticmethod
    def __search(
        from_node: Node, to_node: Node, all_nodes: FrozenSet[Node]
    ) -> Optional[int]:
        explored: Set[Node] = set()
        to_explore: Set[Node] = {from_node}
        while to_explore:
            exploring = min(
                to_explore,
                key=lambda n: to_node.key_space.distance(n.key_space),
            )
            to_explore.remove(exploring)
            explored.add(exploring)

            if to_node.index in exploring.neighbours:
                return len(explored)
            new_nodes = frozenset(
                n for n in all_nodes if n.index in exploring.neighbours
            )
            to_explore.update(new_nodes.difference(explored))
        return None


class RandomNeighbourStrategy(NeighbourStrategy):
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


class ClosestNeighbourStrategy(NeighbourStrategy):
    """
    Selects the closes neighbours.
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


class AllKnowingTestStrategy(TestStrategy):
    """
    Gives every node the choice of every other node. The ideal scenario for a
    `NeighbourStrategy`.
    """

    def connect_nodes(
        self,
        nodes: FrozenSet[Node],
        neighbour_strategy: NeighbourStrategy,
        max_neighbours: int,
    ) -> FrozenSet[Node]:
        new_nodes = []
        for node in nodes:
            for other_node in nodes:
                if node is other_node:
                    pass
                node = neighbour_strategy.apply(
                    node, other_node, max_neighbours, nodes
                )
            new_nodes.append(node)
        return frozenset(new_nodes)


if __name__ == "__main__":
    main()
