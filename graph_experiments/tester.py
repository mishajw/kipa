from itertools import permutations
from typing import NamedTuple, FrozenSet, Optional, Set

from graph_experiments import Node


class ConnectednessResults(NamedTuple):
    successful_percent: float
    mean_num_requests: float

    def __str__(self) -> str:
        return ",".join(
            [
                f"{self.successful_percent * 100:.2f}%",
                f"avg={self.mean_num_requests:.2f}",
            ]
        )


def test_nodes(nodes: FrozenSet[Node]) -> "ConnectednessResults":
    results = [
        __search(from_node, to_node, nodes)
        for from_node, to_node in permutations(nodes, 2)
    ]
    results_success = list(filter(None, results))
    successful_percent = len(results_success) / len(results) if results else 0
    mean_num_requests = (
        sum(results_success) / len(results_success) if results_success else 0
    )
    return ConnectednessResults(successful_percent, mean_num_requests)


def __search(
    from_node: Node, to_node: Node, all_nodes: FrozenSet[Node]
) -> Optional[int]:
    explored: Set[Node] = set()
    to_explore: Set[Node] = {from_node}
    while to_explore:
        exploring = min(
            to_explore, key=lambda n: to_node.key_space.distance(n.key_space)
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
